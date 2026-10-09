//! `WriteApi` extensions: custom guild stickers.
use anyhow::{anyhow, Context, Result};
use reqwest::multipart::{Form, Part};
use serde_json::Value;

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    pub async fn sticker_create(
        &self,
        guild_id: &str,
        name: &str,
        description: &str,
        tags: &str,
        file_path: &str,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let bytes = std::fs::read(file_path).with_context(|| format!("reading {}", file_path))?;
        let mime = if file_path.ends_with(".json") {
            "application/json"
        } else if file_path.ends_with(".apng") {
            "image/apng"
        } else if file_path.ends_with(".gif") {
            "image/gif"
        } else {
            "image/png"
        };
        let fname = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sticker.png")
            .to_string();
        let part = Part::bytes(bytes)
            .file_name(fname)
            .mime_str(mime)
            .map_err(|e| anyhow!(e.to_string()))?;
        let form = Form::new()
            .text("name", name.to_string())
            .text("description", description.to_string())
            .text("tags", tags.to_string())
            .part("file", part);
        self.raw()
            .post_raw_multipart::<Value>(format!("guilds/{}/stickers", guild_id), form)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn sticker_edit(
        &self,
        guild_id: &str,
        sticker_id: &str,
        body: Value,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(sticker_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/stickers/{}", guild_id, sticker_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn sticker_delete(&self, guild_id: &str, sticker_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(sticker_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/stickers/{}", guild_id, sticker_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
