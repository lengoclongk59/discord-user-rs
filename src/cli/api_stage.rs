//! `WriteApi` extensions: stage instances.
//!
//! Wraps `/stage-instances`. The get read lives here (alongside the mutating
//! verbs) since the whole group is stage-instance management — mirroring
//! `api_automod.rs`. A stage instance is keyed by its stage channel id.
use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::cli::api::WriteApi;
use crate::route::Route;

/// Validate a stage instance topic (1-120 characters).
fn validate_topic(topic: &str) -> Result<()> {
    let len = topic.chars().count();
    if !(1..=120).contains(&len) {
        return Err(anyhow!(
            "topic must be 1-120 characters (got {})",
            len
        ));
    }
    Ok(())
}

impl WriteApi {
    /// Create (start) a stage instance. Returns the created stage instance.
    pub async fn stage_start(&self, body: Value) -> Result<Value> {
        self.raw()
            .post::<Value, _>(Route::Custom("stage-instances".into()), body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get the active stage instance for a stage channel.
    pub async fn stage_get(&self, channel_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("stage-instances/{}", channel_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit an active stage instance. Only fields present in `body` are sent.
    pub async fn stage_edit(&self, channel_id: &str, body: Value) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("stage-instances/{}", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Stop (delete) an active stage instance.
    pub async fn stage_stop(&self, channel_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("stage-instances/{}", channel_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}

/// Build the POST body for `stage start`, validating inputs.
pub fn build_start_body(
    channel: &str,
    topic: &str,
    privacy: u8,
    send_start_notification: bool,
) -> Result<Value> {
    crate::cli::api::parse_snowflake(channel)?;
    validate_topic(topic)?;
    if !(1..=2).contains(&privacy) {
        return Err(anyhow!(
            "privacy must be 1 (PUBLIC) or 2 (GUILD_ONLY), got {}",
            privacy
        ));
    }
    let mut body = serde_json::Map::new();
    body.insert("channel_id".into(), serde_json::json!(channel));
    body.insert("topic".into(), serde_json::json!(topic));
    body.insert("privacy_level".into(), serde_json::json!(privacy));
    body.insert(
        "send_start_notification".into(),
        serde_json::json!(send_start_notification),
    );
    Ok(Value::Object(body))
}

/// Build the PATCH body for `stage edit`, validating provided fields.
pub fn build_edit_body(topic: Option<&str>, privacy: Option<u8>) -> Result<Value> {
    let mut body = serde_json::Map::new();
    if let Some(t) = topic {
        validate_topic(t)?;
        body.insert("topic".into(), serde_json::json!(t));
    }
    if let Some(p) = privacy {
        if !(1..=2).contains(&p) {
            return Err(anyhow!(
                "privacy must be 1 (PUBLIC) or 2 (GUILD_ONLY), got {}",
                p
            ));
        }
        body.insert("privacy_level".into(), serde_json::json!(p));
    }
    Ok(Value::Object(body))
}
