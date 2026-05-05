//! Guild operations for DiscordUser

use std::time::Duration;

#[cfg(feature = "collector")]
use async_stream;
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{context::DiscordContext, error::Result, route::Route, types::*};

/// Discord REST API base used by raw helpers in this module.
///
/// Kept in sync with `client::API_BASE` (private). Endpoints added here that
/// don't yet have a [`Route`] variant build URLs via [`format!`] against this
/// base so that another agent owning `route.rs` can land routes independently.

/// Response payload for `POST /guilds/{guild.id}/bulk-ban`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkBanResponse {
    /// User IDs that were successfully banned.
    #[serde(default)]
    pub banned_users: Vec<String>,
    /// User IDs that could not be banned (already banned, missing perms, etc.).
    #[serde(default)]
    pub failed_users: Vec<String>,
}

/// Reduced guild summary returned by `GET /guilds/{guild.id}/preview`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildPreview {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub splash: Option<String>,
    #[serde(default)]
    pub discovery_splash: Option<String>,
    #[serde(default)]
    pub emojis: Vec<Value>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub approximate_member_count: Option<u64>,
    #[serde(default)]
    pub approximate_presence_count: Option<u64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub stickers: Vec<Value>,
}

/// Response payload for `GET /guilds/{guild.id}/vanity-url`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VanityUrl {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub uses: u32,
}

/// Response payload for `GET /guilds/{guild.id}/prune`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneCount {
    #[serde(default)]
    pub pruned: Option<u64>,
}

impl<T: DiscordContext + Send + Sync> GuildOps for T {}

/// Extension trait providing guild operations
#[allow(async_fn_in_trait)]
pub trait GuildOps: DiscordContext {
    /// Fetch all roles in a guild.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_guild_roles(&self, guild_id: &GuildId) -> Result<Vec<Role>> {
        self.http().get(Route::GetGuildRoles { guild_id: guild_id.get() }).await
    }

    /// Create a new role in a guild.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_ROLES permission.
    async fn create_role(&self, guild_id: &GuildId, req: CreateRoleRequest) -> Result<Role> {
        self.http().post(Route::CreateGuildRole { guild_id: guild_id.get() }, req).await
    }

    /// Edit an existing guild role's properties.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_ROLES permission. The bot's highest role must be higher
    /// than the target role.
    async fn edit_role(&self, guild_id: &GuildId, role_id: &RoleId, req: EditRoleRequest) -> Result<Role> {
        self.http().patch(Route::EditGuildRole { guild_id: guild_id.get(), role_id: role_id.get() }, req).await
    }

    /// Delete a guild role permanently.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_ROLES permission. The bot's highest role must be higher
    /// than the target role.
    async fn delete_role(&self, guild_id: &GuildId, role_id: &RoleId) -> Result<()> {
        self.http().delete(Route::DeleteGuildRole { guild_id: guild_id.get(), role_id: role_id.get() }).await
    }

    /// Fetch the guild audit log.
    ///
    /// # Arguments
    /// * `user_id`     — Filter by the user who performed the action
    /// * `action_type` — Filter by [audit log event type](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events)
    /// * `before`      — Return entries before this entry ID
    /// * `after`       — Return entries after this entry ID
    /// * `limit`       — Number of entries to return (1–100, default 50)
    async fn get_audit_logs(&self, guild_id: &GuildId, user_id: Option<&UserId>, action_type: Option<u32>, before: Option<u64>, after: Option<u64>, limit: Option<u8>) -> Result<AuditLog> {
        self.http().get(Route::GetGuildAuditLogs { guild_id: guild_id.get(), user_id: user_id.map(|u| u.get()), action_type, before, after, limit }).await
    }

    /// Set member roles
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_ROLES permission.
    async fn set_member_roles(&self, guild_id: &GuildId, member_id: &UserId, roles: Vec<RoleId>) -> Result<Member> {
        let role_ids: Vec<String> = roles.iter().map(|id| id.get().to_string()).collect();
        self.http().patch(Route::EditGuildMember { guild_id: guild_id.get(), member_id: member_id.get() }, json!({ "roles": role_ids })).await
    }

    /// Get guild invites
    async fn get_guild_invites(&self, guild_id: &GuildId) -> Result<Vec<Invite>> {
        self.http().get(Route::GetGuildInvites { guild_id: guild_id.get() }).await
    }

    /// Create a guild invite
    async fn get_guild_invite_link(&self, channel_id: &ChannelId, max_age: u32, max_uses: u32) -> Result<Invite> {
        self.http()
            .post(
                Route::CreateChannelInvite { channel_id: channel_id.get() },
                json!({
                    "max_age": max_age,
                    "max_uses": max_uses,
                    "target_type": null,
                    "temporary": false,
                    "flags": 0
                }),
            )
            .await
    }

    /// Delete an invite
    async fn delete_guild_invite(&self, code: &str) -> Result<()> {
        self.http().delete(Route::DeleteInvite { code }).await
    }

    /// Get all guild stickers.
    async fn get_guild_stickers(&self, guild_id: &GuildId) -> Result<Vec<Sticker>> {
        self.http().get(Route::GetGuildStickers { guild_id: guild_id.get() }).await
    }

    /// Get a single guild sticker by ID.
    async fn get_guild_sticker(&self, guild_id: &GuildId, sticker_id: &StickerId) -> Result<Sticker> {
        self.http().get(Route::GetGuildSticker { guild_id: guild_id.get(), sticker_id: sticker_id.get() }).await
    }

    /// Create a guild sticker via multipart upload.
    ///
    /// `name` (2-30 chars) and `tags` (autocomplete keywords) are required.
    /// `description` is optional (2-100 chars).
    /// `file` is the sticker image as raw bytes; `filename` must end in
    /// `.png`, `.apng`, `.gif`, or `.json` (Lottie).  Max size 512 KB.
    async fn create_guild_sticker(&self, guild_id: &GuildId, name: &str, description: &str, tags: &str, filename: &str, file: Vec<u8>) -> Result<Sticker> {
        use reqwest::multipart::{Form, Part};
        let form = Form::new().text("name", name.to_string()).text("description", description.to_string()).text("tags", tags.to_string()).part("file", Part::bytes(file).file_name(filename.to_string()));
        let path = Route::CreateGuildSticker { guild_id: guild_id.get() }.path().into_owned();
        self.http().post_raw_multipart(path, form).await
    }

    /// Edit a guild sticker's name, description, or tags.
    async fn edit_guild_sticker(&self, guild_id: &GuildId, sticker_id: &StickerId, req: EditStickerRequest) -> Result<Sticker> {
        self.http().patch(Route::EditGuildSticker { guild_id: guild_id.get(), sticker_id: sticker_id.get() }, req).await
    }

    /// Delete a guild sticker.
    async fn delete_guild_sticker(&self, guild_id: &GuildId, sticker_id: &StickerId) -> Result<()> {
        self.http().delete(Route::DeleteGuildSticker { guild_id: guild_id.get(), sticker_id: sticker_id.get() }).await
    }

    /// List all custom emojis in a guild.
    async fn get_guild_emojis(&self, guild_id: &GuildId) -> Result<Vec<GuildEmoji>> {
        self.http().get(Route::GetGuildEmojis { guild_id: guild_id.get() }).await
    }

    /// Get a single custom emoji by ID.
    async fn get_guild_emoji(&self, guild_id: &GuildId, emoji_id: &EmojiId) -> Result<GuildEmoji> {
        self.http().get(Route::GetGuildEmoji { guild_id: guild_id.get(), emoji_id: emoji_id.get() }).await
    }

    /// Create a new custom emoji. `image` must be a base64 data URI.
    async fn create_emoji(&self, guild_id: &GuildId, req: CreateEmojiRequest) -> Result<GuildEmoji> {
        self.http().post(Route::CreateGuildEmoji { guild_id: guild_id.get() }, req).await
    }

    /// Edit an existing custom emoji (name and/or role restrictions).
    async fn edit_emoji(&self, guild_id: &GuildId, emoji_id: &EmojiId, req: EditEmojiRequest) -> Result<GuildEmoji> {
        self.http().patch(Route::EditGuildEmoji { guild_id: guild_id.get(), emoji_id: emoji_id.get() }, req).await
    }

    /// Delete a custom emoji.
    async fn delete_emoji(&self, guild_id: &GuildId, emoji_id: &EmojiId) -> Result<()> {
        self.http().delete(Route::DeleteGuildEmoji { guild_id: guild_id.get(), emoji_id: emoji_id.get() }).await
    }

    /// Create a new guild.
    ///
    /// Note: User accounts can only create guilds if they are in fewer than 10.
    async fn create_guild(&self, req: CreateGuildRequest) -> Result<Guild> {
        self.http().post(Route::CreateGuild, req).await
    }

    /// Delete a guild. The current user must be the owner.
    async fn delete_guild(&self, guild_id: &GuildId) -> Result<()> {
        self.http().delete(Route::DeleteGuild { guild_id: guild_id.get() }).await
    }

    /// Leave a guild as the current user.
    async fn leave_guild(&self, guild_id: &GuildId) -> Result<()> {
        self.http().delete(Route::LeaveGuild { guild_id: guild_id.get() }).await
    }

    /// Edit guild settings.
    ///
    /// Only the fields you set on [`EditGuildRequest`] will be modified.
    async fn edit_guild(&self, guild_id: &GuildId, req: EditGuildRequest) -> Result<Guild> {
        self.http().patch(Route::EditGuild { guild_id: guild_id.get() }, req).await
    }

    /// Kick a member from the guild.
    ///
    /// Requires KICK_MEMBERS permission.
    async fn kick_member(&self, guild_id: &GuildId, user_id: &UserId) -> Result<()> {
        self.http().delete(Route::KickMember { guild_id: guild_id.get(), user_id: user_id.get() }).await
    }

    /// Ban a user from the guild.
    ///
    /// # Arguments
    /// * `delete_message_seconds` - Seconds of messages to delete (0–604800)
    async fn ban_user(&self, guild_id: &GuildId, user_id: &UserId, delete_message_seconds: u32) -> Result<()> {
        self.http().put(Route::CreateGuildBan { guild_id: guild_id.get(), user_id: user_id.get() }, json!({ "delete_message_seconds": delete_message_seconds.min(604800) })).await
    }

    /// Ban a user with an audit log reason.
    async fn ban_with_reason(&self, guild_id: &GuildId, user_id: &UserId, delete_message_seconds: u32, reason: &str) -> Result<()> {
        self.http().put(Route::CreateGuildBan { guild_id: guild_id.get(), user_id: user_id.get() }, json!({ "delete_message_seconds": delete_message_seconds.min(604800), "reason": reason })).await
    }

    /// Unban a user from the guild.
    async fn unban(&self, guild_id: &GuildId, user_id: &UserId) -> Result<()> {
        self.http().delete(Route::RemoveGuildBan { guild_id: guild_id.get(), user_id: user_id.get() }).await
    }

    /// Get all active bans for a guild.
    async fn get_bans(&self, guild_id: &GuildId) -> Result<Vec<Ban>> {
        self.http().get(Route::GetGuildBans { guild_id: guild_id.get() }).await
    }

    /// Get a specific ban entry for a user.
    async fn get_ban(&self, guild_id: &GuildId, user_id: &UserId) -> Result<Ban> {
        self.http().get(Route::GetGuildBan { guild_id: guild_id.get(), user_id: user_id.get() }).await
    }

    /// Greet a new guild member with a random welcome sticker
    ///
    /// # Arguments
    /// * `guild_id` - The guild ID
    /// * `channel_id` - The channel where the member joined
    /// * `message_id` - The join message ID to reply to
    ///
    /// Uses one of Discord's built-in welcome stickers randomly
    async fn greet_new_guild_member(&self, guild_id: &GuildId, channel_id: &ChannelId, message_id: &MessageId) -> Result<Message> {
        // Discord's built-in welcome stickers
        const WELCOME_STICKERS: &[&str] = &["816087792291282944", "749054660769218631", "751606379340365864", "754108890559283200", "819128604311027752"];

        let sticker_id = WELCOME_STICKERS.choose(&mut rand::rng()).unwrap_or(&WELCOME_STICKERS[0]);

        self.http()
            .post_with_referer(
                Route::CreateMessage { channel_id: channel_id.get() },
                json!({
                    "sticker_ids": [sticker_id],
                    "message_reference": {
                        "guild_id": guild_id.get(),
                        "channel_id": channel_id.get(),
                        "message_id": message_id.get()
                    }
                }),
                &format!("https://discord.com/channels/{}/{}", guild_id.get(), channel_id.get()),
            )
            .await
    }

    /// Get full guild members using "queryless search" scraping technique
    async fn get_full_guild_members(&self, guild_id: &GuildId) -> Result<Vec<Member>> {
        let mut all_members: Vec<Member> = Vec::new();
        let mut cursor: Option<(String, String)> = None; // (guild_joined_at, user_id)

        loop {
            let mut payload = json!({
                "or_query": {},
                "and_query": {},
                "limit": 100
            });

            if let Some((ref ts, ref uid)) = cursor {
                payload["after"] = json!({ "guild_joined_at": ts, "user_id": uid });
            }

            let response: Value = self.http().post(Route::SearchGuildMembers { guild_id: guild_id.get() }, payload).await?;

            let members: Vec<Member> = response
                .get("members")
                .map(|m| serde_json::from_value::<Vec<Member>>(m.clone()))
                .transpose()
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to deserialize guild members: {}", e);
                    Some(Vec::new())
                })
                .unwrap_or_default();

            if members.is_empty() {
                break;
            }

            // Use the last member's joined_at + user_id as the pagination cursor
            if let Some(last) = members.last() {
                let user_id = last.user.as_ref().map(|u| u.id.clone()).or_else(|| last.user_id.clone()).unwrap_or_default();
                let joined_at = last.joined_at.clone().unwrap_or_default();
                cursor = Some((joined_at, user_id));
            }

            let member_count = members.len();
            all_members.extend(members);

            // If we got fewer than 100, we're done
            if member_count < 100 {
                break;
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        Ok(all_members)
    }

    /// Edit a guild member's settings (nick, roles, mute, deaf, voice channel,
    /// timeout).
    ///
    /// Mirrors serenity's `EditMember` / `Http::edit_member()`.
    /// Only fields present in `req` are sent in the PATCH body.
    ///
    /// # Example
    /// ```ignore
    /// guild_ops.edit_guild_member(
    ///     &guild_id, &user_id,
    ///     EditGuildMemberRequest { nick: Some("NewNick".into()), ..Default::default() },
    /// ).await?;
    /// ```
    async fn edit_guild_member(&self, guild_id: &GuildId, member_id: &UserId, req: EditGuildMemberRequest) -> Result<Member> {
        self.http().patch(Route::EditGuildMember { guild_id: guild_id.get(), member_id: member_id.get() }, req).await
    }

    /// Search guild members by username prefix.
    ///
    /// Returns up to `limit` members whose username or nickname starts with
    /// `query`.  `limit` is capped to 1000 by Discord.
    ///
    /// Mirrors serenity's `Http::search_guild_members()`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn search_guild_members(&self, guild_id: &GuildId, query: &str, limit: Option<u32>) -> Result<Vec<Member>> {
        let limit = limit.unwrap_or(100).min(1000);
        self.http().get(Route::GetGuildMembersByQuery { guild_id: guild_id.get(), query: query.to_owned(), limit }).await
    }

    /// Lazily iterate all members of a guild as an async Stream.
    ///
    /// Paginates using the `after` cursor (guild_joined_at + user_id) in pages
    /// of 100, yielding each member individually.  The stream ends when the API
    /// returns fewer members than requested.
    ///
    /// Requires the `collector` feature flag.
    ///
    /// # Errors
    /// Each item is a `Result<Member, DiscordError>`.  HTTP failures are
    /// propagated as stream items.
    #[cfg(feature = "collector")]
    fn guild_members_iter<'a>(&'a self, guild_id: &'a GuildId) -> impl futures::Stream<Item = crate::error::Result<Member>> + 'a {
        async_stream::try_stream! {
            let mut cursor: Option<(String, String)> = None; // (guild_joined_at, user_id)
            loop {
                let mut payload = json!({
                    "or_query": {},
                    "and_query": {},
                    "limit": 100
                });
                if let Some((ref ts, ref uid)) = cursor {
                    payload["after"] = json!({ "guild_joined_at": ts, "user_id": uid });
                }
                let response: serde_json::Value = self.http()
                    .post(Route::SearchGuildMembers { guild_id: guild_id.get() }, payload)
                    .await?;
                let members: Vec<Member> = response.get("members")
                    .map(|m| serde_json::from_value::<Vec<Member>>(m.clone()))
                    .transpose()
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to deserialize guild members in iter: {}", e);
                        Some(Vec::new())
                    })
                    .unwrap_or_default();
                let done = members.len() < 100;
                // Update cursor before consuming the page
                if let Some(last) = members.last() {
                    let user_id = last.user.as_ref().map(|u| u.id.clone())
                        .or_else(|| last.user_id.clone())
                        .unwrap_or_default();
                    let joined_at = last.joined_at.clone().unwrap_or_default();
                    cursor = Some((joined_at, user_id));
                }
                for member in members {
                    yield member;
                }
                if done { break; }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }

    // ---------------------------------------------------------------------
    // 1. Bulk ban
    // ---------------------------------------------------------------------

    /// Ban up to 200 users from a guild in a single call.
    ///
    /// `delete_message_seconds` is clamped to Discord's 0-604800 range when
    /// supplied. Returns lists of users that were banned and users that
    /// failed (already banned, missing permissions, etc.).
    ///
    /// `POST /guilds/{guild.id}/bulk-ban`
    async fn bulk_ban(&self, guild_id: &GuildId, user_ids: Vec<String>, delete_message_seconds: Option<u32>) -> Result<BulkBanResponse> {
        let user_ids: Vec<String> = user_ids.into_iter().take(200).collect();
        let mut body = json!({ "user_ids": user_ids });
        if let Some(secs) = delete_message_seconds {
            body["delete_message_seconds"] = json!(secs.min(604800));
        }
        self.http().post(Route::BulkBan { guild_id: guild_id.get() }, body).await
    }

    // ---------------------------------------------------------------------
    // 2. Add guild member (requires OAuth2 access token with `guilds.join`)
    // ---------------------------------------------------------------------

    /// Add a user to a guild using an OAuth2 access token.
    ///
    /// Returns `Some(Member)` on 201 Created, `None` if the user was already a
    /// member (204 No Content).
    ///
    /// `PUT /guilds/{guild.id}/members/{user.id}`
    async fn add_guild_member(&self, guild_id: &GuildId, user_id: &UserId, access_token: &str, nick: Option<&str>, roles: Option<Vec<RoleId>>, mute: Option<bool>, deaf: Option<bool>) -> Result<Option<Member>> {
        let mut body = json!({ "access_token": access_token });
        if let Some(n) = nick {
            body["nick"] = json!(n);
        }
        if let Some(r) = roles {
            let role_ids: Vec<String> = r.iter().map(|id| id.get().to_string()).collect();
            body["roles"] = json!(role_ids);
        }
        if let Some(m) = mute {
            body["mute"] = json!(m);
        }
        if let Some(d) = deaf {
            body["deaf"] = json!(d);
        }

        self.http().put_optional(Route::AddGuildMember { guild_id: guild_id.get(), user_id: user_id.get() }, body).await
    }

    // ---------------------------------------------------------------------
    // 3. Modify current member
    // ---------------------------------------------------------------------

    /// Modify the current user's nickname in a guild.
    ///
    /// Pass `Some("")` to clear the nickname, `None` to leave it unchanged.
    ///
    /// `PATCH /guilds/{guild.id}/members/@me`
    async fn modify_current_member(&self, guild_id: &GuildId, nick: Option<String>) -> Result<Member> {
        let body = match nick {
            Some(n) => json!({ "nick": n }),
            None => json!({}),
        };
        self.http().patch(Route::ModifyCurrentMember { guild_id: guild_id.get() }, body).await
    }

    // ---------------------------------------------------------------------
    // 4. Guild preview
    // ---------------------------------------------------------------------

    /// Fetch the public preview for a guild — works for any discoverable guild
    /// even if the current user isn't a member.
    ///
    /// `GET /guilds/{guild.id}/preview`
    async fn get_guild_preview(&self, guild_id: &GuildId) -> Result<GuildPreview> {
        self.http().get(Route::GuildPreview { guild_id: guild_id.get() }).await
    }

    // ---------------------------------------------------------------------
    // 5. Welcome screen
    // ---------------------------------------------------------------------

    /// Fetch the welcome screen for a Community guild.
    ///
    /// `GET /guilds/{guild.id}/welcome-screen`
    async fn get_welcome_screen(&self, guild_id: &GuildId) -> Result<crate::types::guild::WelcomeScreen> {
        self.http().get(Route::WelcomeScreen { guild_id: guild_id.get() }).await
    }

    /// Modify the welcome screen for a Community guild.
    ///
    /// Each argument is independently optional — `None` leaves that field
    /// untouched.
    ///
    /// `PATCH /guilds/{guild.id}/welcome-screen`
    async fn modify_welcome_screen(&self, guild_id: &GuildId, enabled: Option<bool>, welcome_channels: Option<Value>, description: Option<String>) -> Result<crate::types::guild::WelcomeScreen> {
        let mut body = serde_json::Map::new();
        if let Some(e) = enabled {
            body.insert("enabled".into(), json!(e));
        }
        if let Some(c) = welcome_channels {
            body.insert("welcome_channels".into(), c);
        }
        if let Some(d) = description {
            body.insert("description".into(), json!(d));
        }
        self.http().patch(Route::WelcomeScreen { guild_id: guild_id.get() }, Value::Object(body)).await
    }

    // ---------------------------------------------------------------------
    // 6. Onboarding
    // ---------------------------------------------------------------------

    /// Fetch the onboarding configuration for a guild.
    ///
    /// `GET /guilds/{guild.id}/onboarding`
    async fn get_guild_onboarding(&self, guild_id: &GuildId) -> Result<Value> {
        self.http().get(Route::GuildOnboarding { guild_id: guild_id.get() }).await
    }

    /// Replace the onboarding configuration for a guild.
    ///
    /// `PUT /guilds/{guild.id}/onboarding`
    async fn modify_guild_onboarding(&self, guild_id: &GuildId, body: Value) -> Result<Value> {
        self.http().put(Route::GuildOnboarding { guild_id: guild_id.get() }, body).await
    }

    // ---------------------------------------------------------------------
    // 7. Guild templates
    // ---------------------------------------------------------------------

    /// Resolve a guild template code without joining the guild.
    ///
    /// `GET /guilds/templates/{template.code}`
    async fn get_guild_template(&self, code: &str) -> Result<Value> {
        self.http().get(Route::GuildTemplate { code: std::borrow::Cow::Borrowed(code) }).await
    }

    /// Create a brand-new guild from a template.
    ///
    /// `icon` is a base64 data URI when present.
    ///
    /// `POST /guilds/templates/{template.code}`
    async fn create_guild_from_template(&self, code: &str, name: &str, icon: Option<&str>) -> Result<Guild> {
        let mut body = json!({ "name": name });
        if let Some(i) = icon {
            body["icon"] = json!(i);
        }
        self.http().post(Route::GuildTemplate { code: std::borrow::Cow::Borrowed(code) }, body).await
    }

    /// List all templates owned by a guild.
    ///
    /// `GET /guilds/{guild.id}/templates`
    async fn get_guild_templates(&self, guild_id: &GuildId) -> Result<Vec<Value>> {
        self.http().get(Route::GuildTemplates { guild_id: guild_id.get() }).await
    }

    /// Create a new template snapshot of the guild.
    ///
    /// `POST /guilds/{guild.id}/templates`
    async fn create_guild_template(&self, guild_id: &GuildId, name: &str, description: Option<&str>) -> Result<Value> {
        let mut body = json!({ "name": name });
        if let Some(d) = description {
            body["description"] = json!(d);
        }
        self.http().post(Route::GuildTemplates { guild_id: guild_id.get() }, body).await
    }

    /// Re-snapshot the guild into an existing template.
    ///
    /// `PUT /guilds/{guild.id}/templates/{code}`
    async fn sync_guild_template(&self, guild_id: &GuildId, code: &str) -> Result<Value> {
        self.http().put(Route::GuildSpecificTemplate { guild_id: guild_id.get(), code: std::borrow::Cow::Borrowed(code) }, Value::Null).await
    }

    /// Edit a template's metadata.
    ///
    /// `PATCH /guilds/{guild.id}/templates/{code}`
    async fn modify_guild_template(&self, guild_id: &GuildId, code: &str, body: Value) -> Result<Value> {
        self.http().patch(Route::GuildSpecificTemplate { guild_id: guild_id.get(), code: std::borrow::Cow::Borrowed(code) }, body).await
    }

    /// Permanently delete a guild template.
    ///
    /// `DELETE /guilds/{guild.id}/templates/{code}`
    async fn delete_guild_template(&self, guild_id: &GuildId, code: &str) -> Result<Value> {
        self.http().delete_with_response(Route::GuildSpecificTemplate { guild_id: guild_id.get(), code: std::borrow::Cow::Borrowed(code) }).await
    }

    // ---------------------------------------------------------------------
    // 8. Widget
    // ---------------------------------------------------------------------

    /// Fetch the widget settings (JSON, requires `MANAGE_GUILD`).
    ///
    /// `GET /guilds/{guild.id}/widget`
    async fn get_guild_widget_settings(&self, guild_id: &GuildId) -> Result<Value> {
        self.http().get(Route::GuildWidgetSettings { guild_id: guild_id.get() }).await
    }

    /// Modify the widget settings.
    ///
    /// `PATCH /guilds/{guild.id}/widget`
    async fn modify_guild_widget(&self, guild_id: &GuildId, enabled: Option<bool>, channel_id: Option<&ChannelId>) -> Result<Value> {
        let mut body = serde_json::Map::new();
        if let Some(e) = enabled {
            body.insert("enabled".into(), json!(e));
        }
        if let Some(cid) = channel_id {
            body.insert("channel_id".into(), json!(cid.get().to_string()));
        }
        self.http().patch(Route::GuildWidgetSettings { guild_id: guild_id.get() }, Value::Object(body)).await
    }

    /// Fetch the public widget JSON. No authentication is enforced server-side
    /// for this endpoint, but we still send the bearer header for consistency.
    ///
    /// `GET /guilds/{guild.id}/widget.json`
    async fn get_guild_widget(&self, guild_id: &GuildId) -> Result<Value> {
        self.http().get(Route::GuildWidgetJson { guild_id: guild_id.get() }).await
    }

    /// Fetch the widget PNG image. `style` is one of
    /// `shield` | `banner1` | `banner2` | `banner3` | `banner4`.
    ///
    /// `GET /guilds/{guild.id}/widget.png?style={style}`
    async fn get_guild_widget_image(&self, guild_id: &GuildId, style: Option<&str>) -> Result<Vec<u8>> {
        self.http().get_bytes(Route::GuildWidgetImage {
            guild_id: guild_id.get(),
            style: style.map(|s| std::borrow::Cow::Owned(s.to_string())),
        }).await
    }

    // ---------------------------------------------------------------------
    // 9. Vanity URL
    // ---------------------------------------------------------------------

    /// Fetch the vanity invite URL for a guild (requires `MANAGE_GUILD`).
    ///
    /// `GET /guilds/{guild.id}/vanity-url`
    async fn get_guild_vanity_url(&self, guild_id: &GuildId) -> Result<VanityUrl> {
        self.http().get(Route::GuildVanityUrl { guild_id: guild_id.get() }).await
    }

    // ---------------------------------------------------------------------
    // 10. Integrations
    // ---------------------------------------------------------------------

    /// List integrations attached to a guild (bots, Twitch, YouTube, etc.).
    ///
    /// `GET /guilds/{guild.id}/integrations`
    async fn get_guild_integrations(&self, guild_id: &GuildId) -> Result<Vec<Value>> {
        self.http().get(Route::GuildIntegrations { guild_id: guild_id.get() }).await
    }

    /// Detach (and kick the owning bot of) a guild integration.
    ///
    /// `DELETE /guilds/{guild.id}/integrations/{integration.id}`
    async fn delete_guild_integration(&self, guild_id: &GuildId, integration_id: u64) -> Result<()> {
        self.http().delete(Route::DeleteGuildIntegration { guild_id: guild_id.get(), integration_id }).await
    }

    // ---------------------------------------------------------------------
    // 11. Prune
    // ---------------------------------------------------------------------

    /// Compute (without applying) the prune count for a guild.
    ///
    /// `days` defaults to 7 server-side; `include_roles` is a list of role IDs
    /// that should still be considered for pruning (Discord excludes any
    /// member with an extra role unless that role is whitelisted here).
    ///
    /// `GET /guilds/{guild.id}/prune?days=...&include_roles=...`
    async fn get_guild_prune_count(&self, guild_id: &GuildId, days: Option<u32>, include_roles: Option<Vec<RoleId>>) -> Result<PruneCount> {
        let days = days.map(|d| d.clamp(1, 30));
        let include_roles = include_roles.and_then(|roles| {
            if roles.is_empty() {
                None
            } else {
                Some(std::borrow::Cow::Owned(roles.iter().map(|r| r.get().to_string()).collect::<Vec<_>>().join(",")))
            }
        });
        self.http().get(Route::GuildPrune { guild_id: guild_id.get(), days, include_roles }).await
    }

    /// Begin a prune operation against the guild.
    ///
    /// When `compute_prune_count` is `false`, Discord returns `{ "pruned":
    /// null }` immediately and runs the prune asynchronously — preferred for
    /// large guilds.
    ///
    /// `POST /guilds/{guild.id}/prune`
    async fn begin_guild_prune(&self, guild_id: &GuildId, days: Option<u32>, compute_prune_count: Option<bool>, include_roles: Option<Vec<RoleId>>, reason: Option<&str>) -> Result<PruneCount> {
        let mut body = serde_json::Map::new();
        if let Some(d) = days {
            body.insert("days".into(), json!(d.clamp(1, 30)));
        }
        if let Some(c) = compute_prune_count {
            body.insert("compute_prune_count".into(), json!(c));
        }
        if let Some(roles) = include_roles {
            let role_ids: Vec<String> = roles.iter().map(|r| r.get().to_string()).collect();
            body.insert("include_roles".into(), json!(role_ids));
        }
        if let Some(r) = reason {
            body.insert("reason".into(), json!(r));
        }
        self.http().post(Route::GuildPrune { guild_id: guild_id.get(), days: None, include_roles: None }, Value::Object(body)).await
    }

    // ---------------------------------------------------------------------
    // 12. Role member helpers
    // ---------------------------------------------------------------------

    /// Add a single role to a guild member without overwriting their other
    /// roles.
    ///
    /// `PUT /guilds/{guild.id}/members/{user.id}/roles/{role.id}`
    async fn add_guild_member_role(&self, guild_id: &GuildId, user_id: &UserId, role_id: &RoleId) -> Result<()> {
        // Discord returns 204 No Content for this endpoint. The central client's
        // `put<T>` always parses a JSON body, so we deserialize into
        // `serde_json::Value` (which accepts `null`/empty) and discard it.
        self.http()
            .put::<serde_json::Value, _>(Route::AddGuildMemberRole { guild_id: guild_id.get(), user_id: user_id.get(), role_id: role_id.get() }, json!({}))
            .await
            .map(|_| ())
    }

    /// Remove a single role from a guild member without overwriting their
    /// other roles.
    ///
    /// `DELETE /guilds/{guild.id}/members/{user.id}/roles/{role.id}`
    async fn remove_guild_member_role(&self, guild_id: &GuildId, user_id: &UserId, role_id: &RoleId) -> Result<()> {
        self.http().delete(Route::RemoveGuildMemberRole { guild_id: guild_id.get(), user_id: user_id.get(), role_id: role_id.get() }).await
    }
}
