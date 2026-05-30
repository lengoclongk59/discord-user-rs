//! `WriteApi` extensions: guild auto-moderation rules.
//!
//! Wraps `/guilds/{guild_id}/auto-moderation/rules`. The list/get reads live
//! here (alongside the mutating verbs) since the whole group is rule
//! management — mirroring `api_bans.rs` / `api_roles.rs`.
use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// List all auto-moderation rules in a guild. Returns a JSON array.
    pub async fn automod_list(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/auto-moderation/rules", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a single auto-moderation rule by ID.
    pub async fn automod_get(&self, guild_id: &str, rule_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(rule_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/auto-moderation/rules/{}", guild_id, rule_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Create an auto-moderation rule. Returns the created rule object.
    pub async fn automod_create(&self, guild_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/auto-moderation/rules", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit an existing auto-moderation rule. Only fields present in `body`
    /// are sent.
    pub async fn automod_edit(&self, guild_id: &str, rule_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(rule_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(
                    format!("guilds/{}/auto-moderation/rules/{}", guild_id, rule_id).into(),
                ),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete an auto-moderation rule.
    pub async fn automod_delete(&self, guild_id: &str, rule_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(rule_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/auto-moderation/rules/{}", guild_id, rule_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
