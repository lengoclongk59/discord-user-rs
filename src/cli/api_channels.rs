//! `WriteApi` extensions: channels.
use anyhow::{anyhow, Result};
use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Create a channel in a guild. Returns the created channel object.
    #[allow(clippy::too_many_arguments)]
    pub async fn channel_create(
        &self,
        guild_id: &str,
        name: &str,
        channel_type: u8,
        topic: Option<&str>,
        parent_id: Option<&str>,
        nsfw: bool,
        slowmode: Option<u64>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        if let Some(pid) = parent_id {
            crate::cli::api::parse_snowflake(pid)?;
        }
        let mut body = json!({
            "name": name,
            "type": channel_type,
        });
        if let Some(t) = topic {
            body["topic"] = json!(t);
        }
        if let Some(pid) = parent_id {
            body["parent_id"] = json!(pid);
        }
        if nsfw {
            body["nsfw"] = json!(true);
        }
        if let Some(s) = slowmode {
            body["rate_limit_per_user"] = json!(s);
        }
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/channels", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit an existing channel. Only fields with `Some` values are sent.
    pub async fn channel_edit(&self, channel_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("channels/{}", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a channel. Optionally records an audit-log reason.
    pub async fn channel_delete(&self, channel_id: &str, reason: Option<&str>) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .request_with_reason_no_response(
                Method::DELETE,
                Route::Custom(format!("channels/{}", channel_id).into()),
                None::<serde_json::Value>,
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Set the slowmode (rate_limit_per_user) on a channel.
    pub async fn channel_set_slowmode(&self, channel_id: &str, seconds: u64) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let body = json!({ "rate_limit_per_user": seconds });
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("channels/{}", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Create a group DM with the listed recipient user IDs. Returns the
    /// created channel object. Requires at least 2 recipients.
    pub async fn create_group_dm(&self, recipient_ids: &[String]) -> Result<Value> {
        for id in recipient_ids {
            crate::cli::api::parse_snowflake(id)?;
        }
        let body = json!({ "recipients": recipient_ids });
        self.raw()
            .post::<Value, _>(Route::Custom("users/@me/channels".into()), body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Add a recipient to a group DM.
    pub async fn group_dm_add_recipient(&self, channel_id: &str, user_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        self.raw()
            .put_empty(Route::Custom(
                format!("channels/{}/recipients/{}", channel_id, user_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Remove a recipient from a group DM.
    pub async fn group_dm_remove_recipient(&self, channel_id: &str, user_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("channels/{}/recipients/{}", channel_id, user_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Rename a group DM. Returns the updated channel object.
    pub async fn group_dm_set_name(&self, channel_id: &str, name: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let body = json!({ "name": name });
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("channels/{}", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
