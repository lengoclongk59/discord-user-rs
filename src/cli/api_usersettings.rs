//! `WriteApi` extensions: user-account notification settings and read-state
//! acknowledgement — the per-user client features (mute, notification level,
//! mark-as-read) that distinguish a self-bot from a bot.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

/// Map a human notification-level string to Discord's `message_notifications`
/// integer: 0 = all, 1 = only mentions, 2 = nothing.
pub fn parse_notification_level(s: &str) -> Result<u8> {
    match s.to_ascii_lowercase().as_str() {
        "all" => Ok(0),
        "mentions" | "only-mentions" => Ok(1),
        "nothing" | "none" => Ok(2),
        other => Err(anyhow!(
            "invalid notification level {:?}; expected: all, mentions, nothing",
            other
        )),
    }
}

impl WriteApi {
    /// PATCH /users/@me/guilds/{guild_id}/settings — guild-level notification
    /// overrides (mute the guild, set its default level, suppress @everyone /
    /// role mentions).
    pub async fn notif_guild_settings(
        &self,
        guild_id: &str,
        muted: Option<bool>,
        level: Option<u8>,
        suppress_everyone: Option<bool>,
        suppress_roles: Option<bool>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let mut body = Map::new();
        if let Some(m) = muted {
            body.insert("muted".into(), json!(m));
        }
        if let Some(l) = level {
            body.insert("message_notifications".into(), json!(l));
        }
        if let Some(s) = suppress_everyone {
            body.insert("suppress_everyone".into(), json!(s));
        }
        if let Some(s) = suppress_roles {
            body.insert("suppress_roles".into(), json!(s));
        }
        if body.is_empty() {
            return Err(anyhow!(
                "nothing to change: pass --muted, --level, --suppress-everyone, or --suppress-roles"
            ));
        }
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("users/@me/guilds/{}/settings", guild_id).into()),
                Value::Object(body),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PATCH /users/@me/guilds/{guild_id}/settings — per-channel override,
    /// expressed as a single-element `channel_overrides` array. Use guild
    /// `"@me"` for DM/group-DM channels.
    pub async fn notif_channel_override(
        &self,
        guild_id: &str,
        channel_id: &str,
        muted: Option<bool>,
        level: Option<u8>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        if guild_id != "@me" {
            crate::cli::api::parse_snowflake(guild_id)?;
        }
        let mut ov = Map::new();
        ov.insert("channel_id".into(), json!(channel_id));
        if let Some(m) = muted {
            ov.insert("muted".into(), json!(m));
        }
        if let Some(l) = level {
            ov.insert("message_notifications".into(), json!(l));
        }
        if ov.len() == 1 {
            return Err(anyhow!("nothing to change: pass --muted and/or --level"));
        }
        let body = json!({ "channel_overrides": [Value::Object(ov)] });
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("users/@me/guilds/{}/settings", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST /channels/{channel_id}/messages/{message_id}/ack — mark a channel
    /// read up to (and including) the given message.
    pub async fn ack_channel(&self, channel_id: &str, message_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        let body = json!({ "token": Value::Null });
        self.raw()
            .post_no_response(
                Route::Custom(
                    format!("channels/{}/messages/{}/ack", channel_id, message_id).into(),
                ),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST /guilds/{guild_id}/ack — mark every channel in a guild read.
    pub async fn ack_guild(&self, guild_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .post_no_response(
                Route::Custom(format!("guilds/{}/ack", guild_id).into()),
                json!({}),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// GET /users/@me/channels — list open DM and group-DM channels.
    pub async fn list_dm_channels(&self) -> Result<Value> {
        self.raw()
            .get::<Value>(Route::Custom("users/@me/channels".into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PATCH /users/@me/relationships/{user_id} — set (or clear) a friend
    /// nickname.
    pub async fn friend_set_nickname(&self, user_id: &str, nickname: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(user_id)?;
        let body = json!({ "nickname": nickname });
        self.raw()
            .patch_no_response(
                Route::Custom(format!("users/@me/relationships/{}", user_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
