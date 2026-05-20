//! Channel operations for DiscordUser

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{context::DiscordContext, error::Result, route::Route, types::*};

impl<T: DiscordContext + Send + Sync> ChannelOps for T {}

/// Response body for `POST /channels/{channel.id}/followers`.
///
/// Returned when an announcement channel is followed — contains the
/// announcement channel id and the webhook id created in the target channel
/// to forward crossposted messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowedChannel {
    /// The announcement channel that was followed.
    pub channel_id: String,
    /// The id of the webhook created in the target channel that forwards
    /// crossposted messages.
    pub webhook_id: String,
}

/// Extension trait providing channel operations
#[allow(async_fn_in_trait)]
pub trait ChannelOps: DiscordContext {
    /// Get or create a private channel (DM)
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_my_private_channel(&self, recipients: Vec<UserId>) -> Result<Channel> {
        let ids: Vec<String> = recipients.iter().map(|id| id.get().to_string()).collect();
        self.http().post(Route::CreateDm, json!({ "recipients": ids })).await
    }

    /// Trigger a typing indicator in a channel (lasts ~10 seconds).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn broadcast_typing(&self, channel_id: &ChannelId) -> Result<()> {
        self.http().post_empty(Route::TriggerTyping { channel_id: channel_id.get() }).await
    }

    /// Set a voice channel status message (visible in the channel header).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires being connected to the voice channel, or MANAGE_CHANNELS.
    async fn set_channel_status(&self, channel_id: &ChannelId, status: &str) -> Result<()> {
        self.http().put(Route::UpdateVoiceStatus { channel_id: channel_id.get() }, json!({ "status": status })).await
    }

    /// Fetch a channel's current settings and metadata.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the channel is not
    /// found.
    async fn get_channel(&self, channel_id: &ChannelId) -> Result<Channel> {
        self.http().get(Route::GetChannel { channel_id: channel_id.get() }).await
    }

    /// Create a new channel in a guild (POST /guilds/{id}/channels).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_CHANNELS permission in the guild.
    async fn create_channel(&self, guild_id: &GuildId, req: CreateChannelRequest) -> Result<Channel> {
        self.http().post(Route::CreateGuildChannel { guild_id: guild_id.get() }, req).await
    }

    /// List all channels (including categories) in a guild
    /// (`GET /guilds/{id}/channels`).
    ///
    /// `Guild.channels` is populated only when the `Guild` object is
    /// received via the GUILD_CREATE / GUILD_UPDATE gateway dispatch —
    /// the REST endpoint `GET /guilds/{id}` does NOT include channels.
    /// Use this method when you need a fresh REST-driven snapshot
    /// (e.g. bot startup, cache rebuild).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_guild_channels(&self, guild_id: &GuildId) -> Result<Vec<Channel>> {
        self.http().get(Route::GetGuildChannels { guild_id: guild_id.get() }).await
    }

    /// Edit a channel's settings (PATCH /channels/{id}).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_CHANNELS permission.
    async fn edit_channel(&self, channel_id: &ChannelId, req: EditChannelRequest) -> Result<Channel> {
        self.http().patch(Route::EditChannel { channel_id: channel_id.get() }, req).await
    }

    /// Delete or close a channel (DELETE /channels/{id}).
    ///
    /// For guild channels this permanently deletes the channel.
    /// For DM channels it closes the conversation (can be re-opened).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_CHANNELS for guild channels.
    async fn delete_channel(&self, channel_id: &ChannelId) -> Result<Channel> {
        self.http().delete_with_response(Route::DeleteChannel { channel_id: channel_id.get() }).await
    }

    /// Get guild information
    ///
    /// # Arguments
    /// * `guild_id` - The guild ID
    /// * `with_counts` - Whether to include approximate member and presence
    ///   counts
    async fn get_guild(&self, guild_id: &GuildId, with_counts: bool) -> Result<Guild> {
        self.http().get(Route::GetGuild { guild_id: guild_id.get(), with_counts }).await
    }

    /// Get user profile
    ///
    /// # Arguments
    /// * `user_id` - The user ID to get profile for
    /// * `guild_id` - Optional guild ID for guild-specific profile data
    ///
    /// # Returns
    /// User profile data including bio, banner, connected accounts, etc.
    async fn get_user_profile(&self, user_id: &UserId, guild_id: Option<&GuildId>) -> Result<UserProfile> {
        self.http().get(Route::GetUserProfile { user_id: user_id.get(), guild_id: guild_id.map(|g| g.get()) }).await
    }

    /// List the invites (with metadata) for a channel
    /// (`GET /channels/{channel.id}/invites`).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_CHANNELS permission.
    async fn get_channel_invites(&self, channel_id: &ChannelId) -> Result<Vec<Invite>> {
        self.http().get(Route::GetChannelInvites { channel_id: channel_id.get() }).await
    }

    /// Edit the permission overwrites for a user or role in a channel
    /// (`PUT /channels/{channel.id}/permissions/{overwrite.id}`).
    ///
    /// `allow` and `deny` are bitfields stringified as decimal numbers.
    /// `overwrite_type` is `0` for a role overwrite or `1` for a member
    /// overwrite. Returns 204 No Content on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_ROLES permission.
    async fn edit_channel_permissions(&self, channel_id: &ChannelId, overwrite_id: u64, allow: Option<&str>, deny: Option<&str>, overwrite_type: u8) -> Result<()> {
        let body = EditChannelPermissionsRequest { allow: allow.map(|s| s.to_string()), deny: deny.map(|s| s.to_string()), overwrite_type };
        self.http().put(Route::EditChannelPermissions { channel_id: channel_id.get(), overwrite_id }, body).await
    }

    /// Delete a channel permission overwrite for a user or role
    /// (`DELETE /channels/{channel.id}/permissions/{overwrite.id}`).
    ///
    /// Returns 204 No Content on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_ROLES permission.
    async fn delete_channel_permission(&self, channel_id: &ChannelId, overwrite_id: u64) -> Result<()> {
        self.http().delete(Route::DeleteChannelPermission { channel_id: channel_id.get(), overwrite_id }).await
    }

    /// Follow an announcement channel and have its messages crossposted to
    /// the target channel (`POST /channels/{channel.id}/followers`).
    ///
    /// `webhook_channel_id` is the id of the target channel that will receive
    /// crossposted messages.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_WEBHOOKS permission in the target channel.
    async fn follow_announcement_channel(&self, channel_id: &ChannelId, webhook_channel_id: u64) -> Result<FollowedChannel> {
        self.http().post(Route::FollowAnnouncementChannel { channel_id: channel_id.get() }, json!({ "webhook_channel_id": webhook_channel_id.to_string() })).await
    }

    /// Add a recipient to a Group DM
    /// (`PUT /channels/{channel.id}/recipients/{user.id}`).
    ///
    /// Both `access_token` (an OAuth2 access token with the `gdm.join` scope)
    /// and `nick` are optional in the wire format but typically required by
    /// Discord. Returns 204 No Content on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn group_dm_add_recipient(&self, channel_id: &ChannelId, user_id: &UserId, access_token: Option<&str>, nick: Option<&str>) -> Result<()> {
        let mut body = serde_json::Map::new();
        if let Some(tok) = access_token {
            body.insert("access_token".to_string(), serde_json::Value::String(tok.to_string()));
        }
        if let Some(n) = nick {
            body.insert("nick".to_string(), serde_json::Value::String(n.to_string()));
        }
        self.http().put(Route::GroupDmAddRecipient { channel_id: channel_id.get(), user_id: user_id.get() }, serde_json::Value::Object(body)).await
    }

    /// Remove a recipient from a Group DM
    /// (`DELETE /channels/{channel.id}/recipients/{user.id}`).
    ///
    /// Returns 204 No Content on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn group_dm_remove_recipient(&self, channel_id: &ChannelId, user_id: &UserId) -> Result<()> {
        self.http().delete(Route::GroupDmRemoveRecipient { channel_id: channel_id.get(), user_id: user_id.get() }).await
    }
}
