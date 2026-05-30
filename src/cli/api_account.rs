//! `WriteApi` extensions: edit the authenticated user's own account/profile.
//!
//! Account fields (username, avatar, banner, accent_color) live on
//! `PATCH /users/@me`; profile fields (bio, pronouns) on
//! `PATCH /users/@me/profile`. Only keys for flags the caller actually passed
//! are included, so unset fields are never null-ed out.
use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use serde_json::{json, Map, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

/// Read an image file and encode it as a `data:<mime>;base64,<...>` URI,
/// guessing the MIME type from the file extension (png/jpg/gif/webp).
fn image_data_uri(path: &str) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path))?;
    let lower = path.to_ascii_lowercase();
    let mime = if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}

impl WriteApi {
    /// `PATCH /users/@me` — edit account-level fields. Builds the body from
    /// only the supplied options; returns `None` (issuing no request) when
    /// every option is absent.
    pub async fn edit_account(
        &self,
        username: Option<&str>,
        avatar_path: Option<&str>,
        banner_path: Option<&str>,
        accent_color: Option<u32>,
    ) -> Result<Option<Value>> {
        let mut body = Map::new();
        if let Some(u) = username {
            body.insert("username".into(), json!(u));
        }
        if let Some(p) = avatar_path {
            body.insert("avatar".into(), json!(image_data_uri(p)?));
        }
        if let Some(p) = banner_path {
            body.insert("banner".into(), json!(image_data_uri(p)?));
        }
        if let Some(c) = accent_color {
            body.insert("accent_color".into(), json!(c));
        }
        if body.is_empty() {
            return Ok(None);
        }
        let result = self
            .raw()
            .patch::<Value, _>(Route::Custom("users/@me".into()), Value::Object(body))
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        Ok(Some(result))
    }

    /// `PATCH /users/@me/profile` — edit profile-level fields. Builds the body
    /// from only the supplied options; returns `None` when all are absent.
    pub async fn edit_profile(
        &self,
        bio: Option<&str>,
        pronouns: Option<&str>,
    ) -> Result<Option<Value>> {
        let mut body = Map::new();
        if let Some(b) = bio {
            body.insert("bio".into(), json!(b));
        }
        if let Some(p) = pronouns {
            body.insert("pronouns".into(), json!(p));
        }
        if body.is_empty() {
            return Ok(None);
        }
        let result = self
            .raw()
            .patch::<Value, _>(
                Route::Custom("users/@me/profile".into()),
                Value::Object(body),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        Ok(Some(result))
    }
}
