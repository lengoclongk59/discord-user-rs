//! `WriteApi` extensions: guild settings & membership.
use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Edit guild settings. Only fields present in `body` are sent.
    pub async fn guild_edit(&self, guild_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Set the current user's nickname in a guild.
    pub async fn guild_set_nick(&self, guild_id: &str, nick: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/members/@me", guild_id).into()),
                serde_json::json!({ "nick": nick }),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Leave a guild (current user).
    pub async fn guild_leave(&self, guild_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("users/@me/guilds/{}", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
