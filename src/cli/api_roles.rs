//! `WriteApi` extensions: guild roles.
use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Create a guild role. Returns the created role object.
    pub async fn role_create(&self, guild_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/roles", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit an existing guild role. Only fields present in `body` are sent.
    pub async fn role_edit(&self, guild_id: &str, role_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(role_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/roles/{}", guild_id, role_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a guild role.
    pub async fn role_delete(&self, guild_id: &str, role_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(role_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/roles/{}", guild_id, role_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List all roles in a guild. Returns a JSON array.
    pub async fn role_list(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/roles", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
