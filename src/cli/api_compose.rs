//! `WriteApi` extensions: rich message composition.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// POST channels/{channel_id}/messages — message with an embed.
    pub async fn compose_embed(
        &self,
        channel_id: &str,
        content: Option<&str>,
        title: Option<&str>,
        description: Option<&str>,
        color: Option<u32>,
        url: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let mut embed = serde_json::Map::new();
        if let Some(t) = title {
            embed.insert("title".into(), json!(t));
        }
        if let Some(d) = description {
            embed.insert("description".into(), json!(d));
        }
        if let Some(c) = color {
            embed.insert("color".into(), json!(c));
        }
        if let Some(u) = url {
            embed.insert("url".into(), json!(u));
        }
        let mut body = json!({ "flags": 0, "embeds": [Value::Object(embed)] });
        if let Some(c) = content {
            body["content"] = json!(c);
        }
        if !self.is_app_token() {
            body["mobile_network_type"] = json!("unknown");
        }
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("channels/{}/messages", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST channels/{channel_id}/messages — TTS message.
    pub async fn compose_tts(&self, channel_id: &str, content: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let mut body = json!({ "content": content, "tts": true, "flags": 0 });
        if !self.is_app_token() {
            body["mobile_network_type"] = json!("unknown");
        }
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("channels/{}/messages", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST channels/{channel_id}/messages — message with a poll.
    pub async fn compose_poll(
        &self,
        channel_id: &str,
        question: &str,
        answers: &[String],
        duration: u32,
        allow_multiselect: bool,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let answer_values: Vec<Value> = answers
            .iter()
            .map(|a| json!({ "poll_media": { "text": a } }))
            .collect();
        let body = json!({
            "poll": {
                "question": { "text": question },
                "answers": answer_values,
                "duration": duration,
                "allow_multiselect": allow_multiselect,
            }
        });
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("channels/{}/messages", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST channels/{channel_id}/messages/{message_id}/ack — mark a message as read.
    pub async fn compose_mark_read(&self, channel_id: &str, message_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        let body = json!({ "token": Value::Null });
        self.raw()
            .post_no_response(
                Route::Custom(
                    format!("channels/{}/messages/{}/ack", channel_id, message_id).into(),
                ),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
