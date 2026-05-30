//! `WriteApi` extensions: invites.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Create an invite for a channel. Returns the created invite object.
    pub async fn invite_create(
        &self,
        channel_id: &str,
        max_age: u32,
        max_uses: u32,
        temporary: bool,
        unique: bool,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let body = json!({
            "max_age": max_age,
            "max_uses": max_uses,
            "temporary": temporary,
            "unique": unique,
        });
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("channels/{}/invites", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete an invite by code. Returns the deleted invite object.
    pub async fn invite_delete(&self, code: &str) -> Result<Value> {
        self.raw()
            .delete_with_response::<Value>(Route::Custom(format!("invites/{}", code).into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List invites for a channel. Returns an array of invite objects.
    pub async fn invite_list_channel(&self, channel_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .get::<Value>(Route::Custom(format!("channels/{}/invites", channel_id).into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List invites for a guild. Returns an array of invite objects.
    pub async fn invite_list_guild(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(format!("guilds/{}/invites", guild_id).into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a single invite by code (with approximate member counts).
    pub async fn invite_get(&self, code: &str) -> Result<Value> {
        self.raw()
            .get::<Value>(Route::Custom(
                format!("invites/{}?with_counts=true", code).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
