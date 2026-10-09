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

    /// GET guilds/{id}/prune — dry-run count of members who would be removed by
    /// a prune of `days` inactivity. `include_roles` optionally counts members
    /// who only have those roles. Returns `{ "pruned": N }`.
    pub async fn member_prune_count(
        &self,
        guild_id: &str,
        days: u32,
        include_roles: &[String],
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let mut url = format!("guilds/{}/prune?days={}", guild_id, days);
        if !include_roles.is_empty() {
            for r in include_roles {
                crate::cli::api::parse_snowflake(r)?;
            }
            url.push_str(&format!("&include_roles={}", include_roles.join(",")));
        }
        self.raw()
            .get::<Value>(Route::Custom(url.into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST guilds/{id}/prune — actually remove members inactive for `days`.
    /// Returns `{ "pruned": N|null }` (null when `compute_count` is false).
    pub async fn member_prune_begin(
        &self,
        guild_id: &str,
        days: u32,
        compute_count: bool,
        include_roles: &[String],
        reason: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        for r in include_roles {
            crate::cli::api::parse_snowflake(r)?;
        }
        let body = json!({
            "days": days,
            "compute_prune_count": compute_count,
            "include_roles": include_roles,
        });
        self.raw()
            .request_with_reason::<Value, _>(
                Method::POST,
                Route::Custom(format!("guilds/{}/prune", guild_id).into()),
                Some(body),
                reason,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// GET guilds/{id}/members/search?query=&limit= — search members by
    /// username/nickname prefix. Returns an array of member objects.
    pub async fn member_search(&self, guild_id: &str, query: &str, limit: u32) -> Result<Value> {
        let gid = crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::GetGuildMembersByQuery {
                guild_id: gid,
                query: query.to_string(),
                limit: limit.clamp(1, 1000),
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
