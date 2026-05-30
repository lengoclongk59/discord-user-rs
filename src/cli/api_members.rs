//! `WriteApi` extensions: guild members.
use anyhow::{anyhow, Result};
use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    pub async fn member_set_nick(
        &self,
        guild_id: &str,
        user_id: &str,
        nick: &str,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        let body = json!({ "nick": nick });
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/members/{}", guild_id, user_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn member_add_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        crate::cli::api::parse_snowflake(role_id)?;
        self.raw()
            .put_empty(Route::Custom(
                format!("guilds/{}/members/{}/roles/{}", guild_id, user_id, role_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn member_remove_role(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        crate::cli::api::parse_snowflake(role_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/members/{}/roles/{}", guild_id, user_id, role_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn member_timeout(
        &self,
        guild_id: &str,
        user_id: &str,
        until: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        let body = json!({ "communication_disabled_until": until });
        self.raw()
            .request_with_reason_no_response(
                Method::PATCH,
                Route::Custom(format!("guilds/{}/members/{}", guild_id, user_id).into()),
                Some(body),
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn member_untimeout(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        let body = json!({ "communication_disabled_until": null });
        self.raw()
            .request_with_reason_no_response(
                Method::PATCH,
                Route::Custom(format!("guilds/{}/members/{}", guild_id, user_id).into()),
                Some(body),
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn member_kick(
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
                Route::Custom(format!("guilds/{}/members/{}", guild_id, user_id).into()),
                None::<serde_json::Value>,
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
