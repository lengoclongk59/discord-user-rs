//! `WriteApi` extensions: guild scheduled events.
use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Create a guild scheduled event. Returns the created event object.
    pub async fn event_create(&self, guild_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/scheduled-events", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit an existing guild scheduled event. Only fields present in `body` are sent.
    pub async fn event_edit(&self, guild_id: &str, event_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(event_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(
                    format!("guilds/{}/scheduled-events/{}", guild_id, event_id).into(),
                ),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a guild scheduled event.
    pub async fn event_delete(&self, guild_id: &str, event_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(event_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/scheduled-events/{}", guild_id, event_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
