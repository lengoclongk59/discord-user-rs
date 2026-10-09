//! `WriteApi` extensions: webhooks.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// Create a webhook in a channel. Returns the created webhook object.
    pub async fn webhook_create(&self, channel_id: &str, name: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let body = json!({ "name": name });
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("channels/{}/webhooks", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit a webhook. Only fields present in `body` are sent.
    pub async fn webhook_edit(&self, webhook_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(webhook_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("webhooks/{}", webhook_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a webhook.
    pub async fn webhook_delete(&self, webhook_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(webhook_id)?;
        self.raw()
            .delete(Route::Custom(format!("webhooks/{}", webhook_id).into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Execute a webhook (fire and forget, no response body).
    pub async fn webhook_execute(
        &self,
        webhook_id: &str,
        token: &str,
        content: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(webhook_id)?;
        let body = json!({ "content": content });
        self.raw()
            .post_no_response(
                Route::Custom(format!("webhooks/{}/{}?wait=false", webhook_id, token).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List webhooks for a channel. Returns a JSON array.
    pub async fn webhook_list_channel(&self, channel_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("channels/{}/webhooks", channel_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
