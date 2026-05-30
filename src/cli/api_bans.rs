//! `WriteApi` extensions: guild bans.
use anyhow::{anyhow, Result};
use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    pub async fn ban_add(
        &self,
        guild_id: &str,
        user_id: &str,
        delete_seconds: u32,
        reason: Option<&str>,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        let body = json!({ "delete_message_seconds": delete_seconds });
        self.raw()
            .request_with_reason_no_response(
                Method::PUT,
                Route::Custom(format!("guilds/{}/bans/{}", guild_id, user_id).into()),
                Some(body),
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn ban_remove(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        self.raw()
            .request_with_reason_no_response(
                Method::DELETE,
                Route::Custom(format!("guilds/{}/bans/{}", guild_id, user_id).into()),
                None::<serde_json::Value>,
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn ban_list(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/bans", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn ban_get(&self, guild_id: &str, user_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/bans/{}", guild_id, user_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST guilds/{id}/bulk-ban — ban up to 200 users in a single request.
    /// Returns `{ banned_users: [...], failed_users: [...] }`.
    pub async fn ban_bulk(
        &self,
        guild_id: &str,
        user_ids: &[String],
        delete_seconds: u32,
        reason: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        for id in user_ids {
            crate::cli::api::parse_snowflake(id)?;
        }
        let body = json!({
            "user_ids": user_ids,
            "delete_message_seconds": delete_seconds,
        });
        self.raw()
            .request_with_reason::<Value, _>(
                Method::POST,
                Route::Custom(format!("guilds/{}/bulk-ban", guild_id).into()),
                Some(body),
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
