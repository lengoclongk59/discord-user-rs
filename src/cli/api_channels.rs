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
}
