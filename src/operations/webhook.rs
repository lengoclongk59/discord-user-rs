//! Webhook operations for DiscordUser

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{context::DiscordContext, error::Result, route::Route, types::*};

/// A standard sticker pack listed under `/sticker-packs`.
///
/// Discord groups its first-party "Standard" stickers into packs that any
/// Nitro user can ship. The structure mirrors the documented payload at
/// `developers/resources/sticker.mdx#sticker-pack-object`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerPack {
    /// Snowflake ID of the sticker pack (string in JSON).
    pub id: String,
    /// Stickers contained in the pack.
    #[serde(default)]
    pub stickers: Vec<Sticker>,
    /// Display name of the pack.
    pub name: String,
    /// SKU snowflake associated with the pack.
    pub sku_id: String,
    /// Sticker shown as the pack's cover, when set.
    #[serde(default)]
    pub cover_sticker_id: Option<String>,
    /// Marketing description for the pack.
    #[serde(default)]
    pub description: String,
    /// Asset hash for the banner image, when present.
    #[serde(default)]
    pub banner_asset_id: Option<String>,
}

/// Response shape for `GET /sticker-packs` per Discord documentation.
#[derive(Debug, Clone, Deserialize)]
struct StickerPacksResponse {
    sticker_packs: Vec<StickerPack>,
}

/// Response shape for `GET /applications/{app_id}/emojis` per Discord
/// documentation. Application emoji listing is always wrapped in `items`.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationEmojiList {
    /// The emojis owned by the application.
    pub items: Vec<Emoji>,
}

impl<T: DiscordContext + Send + Sync> WebhookOps for T {}

/// Extension trait providing webhook operations
#[allow(async_fn_in_trait)]
pub trait WebhookOps: DiscordContext {
    /// Get all webhooks in a channel.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_WEBHOOKS permission.
    async fn get_channel_webhooks(&self, channel_id: &ChannelId) -> Result<Vec<Webhook>> {
        self.http().get(Route::GetChannelWebhooks { channel_id: channel_id.get() }).await
    }

    /// Get all webhooks in a guild.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_WEBHOOKS permission.
    async fn get_guild_webhooks(&self, guild_id: &GuildId) -> Result<Vec<Webhook>> {
        self.http().get(Route::GetGuildWebhooks { guild_id: guild_id.get() }).await
    }

    /// Create a webhook in a channel.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_WEBHOOKS permission.
    async fn create_webhook(&self, channel_id: &ChannelId, req: CreateWebhookRequest) -> Result<Webhook> {
        self.http().post(Route::CreateWebhook { channel_id: channel_id.get() }, req).await
    }

    /// Get a webhook by ID (requires authentication).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the webhook is not
    /// found.
    async fn get_webhook(&self, webhook_id: &WebhookId) -> Result<Webhook> {
        self.http().get(Route::GetWebhook { webhook_id: webhook_id.get() }).await
    }

    /// Get a webhook using its ID and token (no authentication required).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the webhook/token
    /// pair is invalid.
    async fn get_webhook_with_token(&self, webhook_id: &WebhookId, token: &str) -> Result<Webhook> {
        self.http().get(Route::GetWebhookWithToken { webhook_id: webhook_id.get(), token }).await
    }

    /// Edit a webhook's name, avatar, or channel.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_WEBHOOKS permission.
    async fn edit_webhook(&self, webhook_id: &WebhookId, req: EditWebhookRequest) -> Result<Webhook> {
        self.http().patch(Route::EditWebhook { webhook_id: webhook_id.get() }, req).await
    }

    /// Edit a webhook using its token (no authentication required).
    ///
    /// Note: changing the channel is not supported via the token-authenticated
    /// endpoint.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn edit_webhook_with_token(&self, webhook_id: &WebhookId, token: &str, req: EditWebhookRequest) -> Result<Webhook> {
        self.http().patch(Route::EditWebhookWithToken { webhook_id: webhook_id.get(), token }, req).await
    }

    /// Delete a webhook permanently.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_WEBHOOKS permission.
    async fn delete_webhook(&self, webhook_id: &WebhookId) -> Result<()> {
        self.http().delete(Route::DeleteWebhook { webhook_id: webhook_id.get() }).await
    }

    /// Delete a webhook using its token (no authentication required).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn delete_webhook_with_token(&self, webhook_id: &WebhookId, token: &str) -> Result<()> {
        self.http().delete(Route::DeleteWebhookWithToken { webhook_id: webhook_id.get(), token }).await
    }

    /// Execute a webhook — sends a message via the webhook.
    ///
    /// Discord returns 204 No Content by default. To receive the posted
    /// `Message` back, append `?wait=true` — not yet implemented here.
    async fn execute_webhook(&self, webhook_id: &WebhookId, token: &str, req: ExecuteWebhookRequest) -> Result<()> {
        self.http().post_no_response(Route::ExecuteWebhook { webhook_id: webhook_id.get(), token }, req).await
    }

    /// Execute a webhook with a Slack-formatted payload.
    ///
    /// Targets `POST /webhooks/{webhook.id}/{webhook.token}/slack`.
    /// `body` should match the Slack incoming-webhook payload shape that
    /// Discord parses on this endpoint. When `wait` is `true`, Discord
    /// returns the created [`Message`]; otherwise it returns 204 and this
    /// method yields `Ok(None)`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn execute_webhook_slack(&self, webhook_id: &WebhookId, token: &str, wait: bool, thread_id: Option<u64>, body: Value) -> Result<Option<Message>> {
        // Inline the URL: stuff `slack` plus the query string into the
        // existing `ExecuteWebhook` route's `token` field, which produces
        // `webhooks/{id}/{token}` — yielding the full path we need.
        let suffix: String = match thread_id {
            Some(tid) => format!("{}/slack?wait={}&thread_id={}", token, wait, tid),
            None => format!("{}/slack?wait={}", token, wait),
        };
        if wait {
            let msg: Message = self.http().post(Route::ExecuteWebhook { webhook_id: webhook_id.get(), token: &suffix }, body).await?;
            Ok(Some(msg))
        } else {
            self.http().post_no_response(Route::ExecuteWebhook { webhook_id: webhook_id.get(), token: &suffix }, body).await?;
            Ok(None)
        }
    }

    /// Execute a webhook with a GitHub-formatted payload.
    ///
    /// Targets `POST /webhooks/{webhook.id}/{webhook.token}/github`.
    /// `body` should match the GitHub webhook payload Discord understands
    /// (e.g. push / pull_request events). Returns the created [`Message`]
    /// when `wait` is `true`, otherwise `None`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn execute_webhook_github(&self, webhook_id: &WebhookId, token: &str, wait: bool, thread_id: Option<u64>, body: Value) -> Result<Option<Message>> {
        let suffix: String = match thread_id {
            Some(tid) => format!("{}/github?wait={}&thread_id={}", token, wait, tid),
            None => format!("{}/github?wait={}", token, wait),
        };
        if wait {
            let msg: Message = self.http().post(Route::ExecuteWebhook { webhook_id: webhook_id.get(), token: &suffix }, body).await?;
            Ok(Some(msg))
        } else {
            self.http().post_no_response(Route::ExecuteWebhook { webhook_id: webhook_id.get(), token: &suffix }, body).await?;
            Ok(None)
        }
    }

    /// Get a previously-sent webhook message.
    ///
    /// Targets `GET /webhooks/{webhook.id}/{webhook.token}/messages/{message.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the message has
    /// already been deleted.
    async fn get_webhook_message(&self, webhook_id: &WebhookId, token: &str, message_id: &MessageId, thread_id: Option<u64>) -> Result<Message> {
        // Compose the trailing path inline: `{token}/messages/{message_id}`
        // (plus optional thread_id query) lands inside `ExecuteWebhook`'s
        // token slot to produce `webhooks/{id}/{token}/messages/{mid}`.
        let suffix: String = match thread_id {
            Some(tid) => format!("{}/messages/{}?thread_id={}", token, message_id.get(), tid),
            None => format!("{}/messages/{}", token, message_id.get()),
        };
        self.http().get(Route::GetWebhookWithToken { webhook_id: webhook_id.get(), token: &suffix }).await
    }

    /// Edit a previously-sent webhook message.
    ///
    /// Targets `PATCH /webhooks/{webhook.id}/{webhook.token}/messages/{message.id}`.
    /// The `body` mirrors the create-message payload (content / embeds /
    /// allowed_mentions / components / attachments / flags).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn edit_webhook_message(&self, webhook_id: &WebhookId, token: &str, message_id: &MessageId, thread_id: Option<u64>, body: Value) -> Result<Message> {
        let suffix: String = match thread_id {
            Some(tid) => format!("{}/messages/{}?thread_id={}", token, message_id.get(), tid),
            None => format!("{}/messages/{}", token, message_id.get()),
        };
        self.http().patch(Route::EditWebhookWithToken { webhook_id: webhook_id.get(), token: &suffix }, body).await
    }

    /// Delete a previously-sent webhook message.
    ///
    /// Targets `DELETE /webhooks/{webhook.id}/{webhook.token}/messages/{message.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn delete_webhook_message(&self, webhook_id: &WebhookId, token: &str, message_id: &MessageId, thread_id: Option<u64>) -> Result<()> {
        let suffix: String = match thread_id {
            Some(tid) => format!("{}/messages/{}?thread_id={}", token, message_id.get(), tid),
            None => format!("{}/messages/{}", token, message_id.get()),
        };
        self.http().delete(Route::DeleteWebhookWithToken { webhook_id: webhook_id.get(), token: &suffix }).await
    }

    /// List all emojis owned by an application.
    ///
    /// Targets `GET /applications/{application.id}/emojis`. Discord wraps
    /// the list in an `items` envelope which this method unwraps for
    /// callers.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn list_application_emojis(&self, application_id: &ApplicationId) -> Result<Vec<Emoji>> {
        let resp: ApplicationEmojiList = self.http().get(Route::ApplicationEmojis { application_id: application_id.get() }).await?;
        Ok(resp.items)
    }

    /// Get a single application-owned emoji.
    ///
    /// Targets `GET /applications/{application.id}/emojis/{emoji.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the emoji is
    /// not owned by the application.
    async fn get_application_emoji(&self, application_id: &ApplicationId, emoji_id: &EmojiId) -> Result<Emoji> {
        self.http().get(Route::ApplicationEmoji { application_id: application_id.get(), emoji_id: emoji_id.get() }).await
    }

    /// Upload a new emoji owned by the application.
    ///
    /// Targets `POST /applications/{application.id}/emojis`. The `image`
    /// argument must be a [data URI](https://discord.com/developers/docs/reference#image-data)
    /// such as `data:image/png;base64,...`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn create_application_emoji(&self, application_id: &ApplicationId, name: &str, image: &str) -> Result<Emoji> {
        let body = json!({ "name": name, "image": image });
        self.http().post(Route::ApplicationEmojis { application_id: application_id.get() }, body).await
    }

    /// Rename an application-owned emoji.
    ///
    /// Targets `PATCH /applications/{application.id}/emojis/{emoji.id}`.
    /// Only the `name` field may be modified at the application scope.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn modify_application_emoji(&self, application_id: &ApplicationId, emoji_id: &EmojiId, name: &str) -> Result<Emoji> {
        let body = json!({ "name": name });
        self.http().patch(Route::ApplicationEmoji { application_id: application_id.get(), emoji_id: emoji_id.get() }, body).await
    }

    /// Delete an application-owned emoji.
    ///
    /// Targets `DELETE /applications/{application.id}/emojis/{emoji.id}`.
    /// Discord returns 204 No Content on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn delete_application_emoji(&self, application_id: &ApplicationId, emoji_id: &EmojiId) -> Result<()> {
        self.http().delete(Route::ApplicationEmoji { application_id: application_id.get(), emoji_id: emoji_id.get() }).await
    }

    /// Get a single sticker by ID, regardless of pack or guild.
    ///
    /// Targets `GET /stickers/{sticker.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the sticker
    /// does not exist.
    async fn get_sticker(&self, sticker_id: &StickerId) -> Result<Sticker> {
        self.http().get(Route::Sticker { sticker_id: sticker_id.get() }).await
    }

    /// List the standard sticker packs available to all users.
    ///
    /// Targets `GET /sticker-packs`. Discord wraps the response in
    /// `{ "sticker_packs": [...] }`; this method unwraps and returns the
    /// inner list.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn list_sticker_packs(&self) -> Result<Vec<StickerPack>> {
        let resp: StickerPacksResponse = self.http().get(Route::StickerPacks).await?;
        Ok(resp.sticker_packs)
    }

    /// Get a single standard sticker pack by ID.
    ///
    /// Targets `GET /sticker-packs/{pack.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the pack does
    /// not exist.
    async fn get_sticker_pack(&self, pack_id: u64) -> Result<StickerPack> {
        self.http().get(Route::StickerPack { pack_id }).await
    }

    /// Get an invite by its code.
    ///
    /// Targets `GET /invites/{code}`. Optional flags expand the response:
    /// - `with_counts` adds approximate member / presence counts.
    /// - `with_expiration` includes the `expires_at` timestamp.
    /// - `guild_scheduled_event_id` resolves a specific scheduled event
    ///   the invite is anchored to.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the invite is
    /// invalid / expired.
    async fn get_invite(&self, code: &str, with_counts: Option<bool>, with_expiration: Option<bool>, guild_scheduled_event_id: Option<u64>) -> Result<crate::types::guild::Invite> {
        self.http()
            .get(Route::Invite {
                code: Cow::Borrowed(code),
                with_counts,
                with_expiration,
                guild_scheduled_event_id,
            })
            .await
    }
}
