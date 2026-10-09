//! `WriteApi` extensions: custom guild emojis.
use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    pub async fn emoji_create(
        &self,
        guild_id: &str,
        name: &str,
        image_path: &str,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        let bytes = std::fs::read(image_path).with_context(|| format!("reading {}", image_path))?;
        let mime = if image_path.ends_with(".gif") {
            "image/gif"
        } else if image_path.ends_with(".jpg") || image_path.ends_with(".jpeg") {
            "image/jpeg"
        } else {
            "image/png"
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let data_uri = format!("data:{};base64,{}", mime, b64);
        let body = json!({ "name": name, "image": data_uri });
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/emojis", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn emoji_rename(&self, guild_id: &str, emoji_id: &str, name: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(emoji_id)?;
        let body = json!({ "name": name });
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/emojis/{}", guild_id, emoji_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    pub async fn emoji_delete(&self, guild_id: &str, emoji_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(emoji_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/emojis/{}", guild_id, emoji_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
