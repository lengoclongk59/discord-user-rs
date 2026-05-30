//! `WriteApi` extensions: application (slash) command management.
//!
//! Wraps Discord's application command REST API under
//! `/applications/{app_id}/commands` (global) and
//! `/applications/{app_id}/guilds/{guild_id}/commands` (guild-scoped).
//! The list/get reads live here alongside the create/delete verbs since the
//! whole group is application-command management — mirroring `api_automod.rs`.
//!
//! Command *registration* works cleanly over REST for bot/application tokens.
//! INVOKING a slash command as a user requires the undocumented interactions
//! gateway flow and is intentionally not implemented here (see the `run`
//! subcommand in `dcw_commands.rs`).
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Resolve the current application id.
    ///
    /// Targets `GET /oauth2/applications/@me`, which returns the application
    /// object (including `id`) for bot/application tokens. This is the
    /// canonical way to discover the app id before hitting the
    /// `/applications/{app_id}/commands` endpoints.
    pub async fn app_id(&self) -> Result<String> {
        let app: Value = self
            .raw()
            .get::<Value>(Route::Custom("oauth2/applications/@me".into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        app["id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("could not determine application id from /oauth2/applications/@me"))
    }

    /// List application commands — global, or guild-scoped when `guild_id`
    /// is set. Returns the raw JSON array.
    pub async fn commands_list(&self, app_id: &str, guild_id: Option<&str>) -> Result<Value> {
        crate::cli::api::parse_snowflake(app_id)?;
        let path = match guild_id {
            Some(gid) => {
                crate::cli::api::parse_snowflake(gid)?;
                format!("applications/{}/guilds/{}/commands", app_id, gid)
            }
            None => format!("applications/{}/commands", app_id),
        };
        self.raw()
            .get::<Value>(Route::Custom(path.into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a single application command by ID — global or guild-scoped.
    pub async fn command_get(
        &self,
        app_id: &str,
        guild_id: Option<&str>,
        command_id: &str,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(app_id)?;
        crate::cli::api::parse_snowflake(command_id)?;
        let path = match guild_id {
            Some(gid) => {
                crate::cli::api::parse_snowflake(gid)?;
                format!(
                    "applications/{}/guilds/{}/commands/{}",
                    app_id, gid, command_id
                )
            }
            None => format!("applications/{}/commands/{}", app_id, command_id),
        };
        self.raw()
            .get::<Value>(Route::Custom(path.into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Create a `CHAT_INPUT`-style application command (type defaults to 1).
    /// Posts to the global or guild endpoint depending on `guild_id`.
    pub async fn command_create(
        &self,
        app_id: &str,
        guild_id: Option<&str>,
        name: &str,
        description: &str,
        command_type: u8,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(app_id)?;
        let path = match guild_id {
            Some(gid) => {
                crate::cli::api::parse_snowflake(gid)?;
                format!("applications/{}/guilds/{}/commands", app_id, gid)
            }
            None => format!("applications/{}/commands", app_id),
        };
        let body = json!({
            "name": name,
            "description": description,
            "type": command_type,
        });
        self.raw()
            .post::<Value, _>(Route::Custom(path.into()), body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete an application command — global or guild-scoped.
    pub async fn command_delete(
        &self,
        app_id: &str,
        guild_id: Option<&str>,
        command_id: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(app_id)?;
        crate::cli::api::parse_snowflake(command_id)?;
        let path = match guild_id {
            Some(gid) => {
                crate::cli::api::parse_snowflake(gid)?;
                format!(
                    "applications/{}/guilds/{}/commands/{}",
                    app_id, gid, command_id
                )
            }
            None => format!("applications/{}/commands/{}", app_id, command_id),
        };
        self.raw()
            .delete(Route::Custom(path.into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
