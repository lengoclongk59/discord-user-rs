//! `WriteApi` extensions: rich message composition.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

/// Full set of embed fields the CLI can populate, mirroring the embed editor
/// in the official client. Every field is optional; `fields` is a list of
/// `(name, value, inline)` tuples.
#[derive(Default, Debug)]
pub struct EmbedSpec {
    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<u32>,
    pub url: Option<String>,
    pub footer: Option<String>,
    pub footer_icon: Option<String>,
    pub author: Option<String>,
    pub author_url: Option<String>,
    pub author_icon: Option<String>,
    pub image: Option<String>,
    pub thumbnail: Option<String>,
    pub timestamp: Option<String>,
    pub fields: Vec<(String, String, bool)>,
}

impl EmbedSpec {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.color.is_none()
            && self.url.is_none()
            && self.footer.is_none()
            && self.author.is_none()
            && self.image.is_none()
            && self.thumbnail.is_none()
            && self.timestamp.is_none()
            && self.fields.is_empty()
    }

    fn to_json(&self) -> Value {
        let mut embed = serde_json::Map::new();
        if let Some(t) = &self.title {
            embed.insert("title".into(), json!(t));
        }
        if let Some(d) = &self.description {
            embed.insert("description".into(), json!(d));
        }
        if let Some(c) = self.color {
            embed.insert("color".into(), json!(c));
        }
        if let Some(u) = &self.url {
            embed.insert("url".into(), json!(u));
        }
        if let Some(ts) = &self.timestamp {
            embed.insert("timestamp".into(), json!(ts));
        }
        if let Some(f) = &self.footer {
            let mut footer = json!({ "text": f });
            if let Some(icon) = &self.footer_icon {
                footer["icon_url"] = json!(icon);
            }
            embed.insert("footer".into(), footer);
        }
        if let Some(a) = &self.author {
            let mut author = json!({ "name": a });
            if let Some(u) = &self.author_url {
                author["url"] = json!(u);
            }
            if let Some(icon) = &self.author_icon {
                author["icon_url"] = json!(icon);
            }
            embed.insert("author".into(), author);
        }
        if let Some(img) = &self.image {
            embed.insert("image".into(), json!({ "url": img }));
        }
        if let Some(th) = &self.thumbnail {
            embed.insert("thumbnail".into(), json!({ "url": th }));
        }
        if !self.fields.is_empty() {
            let fields: Vec<Value> = self
                .fields
                .iter()
                .map(|(name, value, inline)| json!({ "name": name, "value": value, "inline": inline }))
                .collect();
            embed.insert("fields".into(), Value::Array(fields));
        }
        Value::Object(embed)
    }
}

impl WriteApi {
    /// POST channels/{channel_id}/messages — message with an embed.
    ///
    /// When `files` is non-empty the request is sent as `multipart/form-data`
    /// with the embed JSON in `payload_json` and the files as `files[i]` parts;
    /// otherwise the JSON-only path is used.
    pub async fn compose_embed(
        &self,
        channel_id: &str,
        content: Option<&str>,
        spec: &EmbedSpec,
        files: &[std::path::PathBuf],
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        if spec.is_empty() && content.is_none() {
            return Err(anyhow!(
                "nothing to send: provide --content and/or at least one embed field"
            ));
        }
        let mut body = json!({ "flags": 0, "embeds": [spec.to_json()] });
        if let Some(c) = content {
            body["content"] = json!(c);
        }
        if !self.is_application_token() {
            body["mobile_network_type"] = json!("unknown");
        }
        if !files.is_empty() {
            return self.post_message_with_files(channel_id, body, files).await;
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
        if !self.is_application_token() {
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
