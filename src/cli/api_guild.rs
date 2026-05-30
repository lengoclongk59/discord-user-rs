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

    /// Get a guild's vanity URL (code + uses).
    pub async fn guild_vanity(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/vanity-url", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a guild's widget settings object.
    pub async fn guild_widget_get(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/widget", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit a guild's widget settings. Only provided fields are sent.
    pub async fn guild_widget_edit(
        &self,
        guild_id: &str,
        enabled: Option<bool>,
        channel_id: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let mut body = serde_json::Map::new();
        if let Some(e) = enabled {
            body.insert("enabled".into(), serde_json::json!(e));
        }
        if let Some(c) = channel_id {
            crate::cli::api::parse_snowflake(c)?;
            body.insert("channel_id".into(), serde_json::json!(c));
        }
        if body.is_empty() {
            return Err(anyhow!("nothing to edit"));
        }
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/widget", guild_id).into()),
                Value::Object(body),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a guild's welcome screen.
    pub async fn guild_welcome_get(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/welcome-screen", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit a guild's welcome screen. Only provided fields are sent.
    pub async fn guild_welcome_edit(
        &self,
        guild_id: &str,
        enabled: Option<bool>,
        description: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let mut body = serde_json::Map::new();
        if let Some(e) = enabled {
            body.insert("enabled".into(), serde_json::json!(e));
        }
        if let Some(d) = description {
            body.insert("description".into(), serde_json::json!(d));
        }
        if body.is_empty() {
            return Err(anyhow!("nothing to edit"));
        }
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/welcome-screen", guild_id).into()),
                Value::Object(body),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a guild's onboarding configuration.
    pub async fn guild_onboarding_get(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/onboarding", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List a guild's integrations.
    pub async fn guild_integrations(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/integrations", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a guild integration by ID.
    pub async fn guild_integration_delete(
        &self,
        guild_id: &str,
        integration_id: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(integration_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/integrations/{}", guild_id, integration_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List a guild's templates.
    pub async fn guild_templates(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/templates", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Create a guild template. Only provided fields are sent.
    pub async fn guild_template_create(
        &self,
        guild_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let mut body = serde_json::Map::new();
        body.insert("name".into(), serde_json::json!(name));
        if let Some(d) = description {
            body.insert("description".into(), serde_json::json!(d));
        }
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/templates", guild_id).into()),
                Value::Object(body),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Sync a guild template to the guild's current state.
    pub async fn guild_template_sync(&self, guild_id: &str, code: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .put::<Value, _>(
                Route::Custom(format!("guilds/{}/templates/{}", guild_id, code).into()),
                serde_json::json!({}),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a guild template by code. Returns the deleted template object.
    pub async fn guild_template_delete(&self, guild_id: &str, code: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .delete_with_response::<Value>(Route::Custom(
                format!("guilds/{}/templates/{}", guild_id, code).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
