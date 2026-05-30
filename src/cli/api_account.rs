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
        global_name: Option<&str>,
        avatar_path: Option<&str>,
        banner_path: Option<&str>,
        accent_color: Option<u32>,
    ) -> Result<Option<Value>> {
        let mut body = Map::new();
        if let Some(u) = username {
            body.insert("username".into(), json!(u));
        }
        if let Some(g) = global_name {
            body.insert("global_name".into(), json!(g));
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

    /// GET /users/@me/connections — list third-party linked accounts.
    pub async fn get_connections(&self) -> Result<Value> {
        self.raw()
            .get::<Value>(Route::Custom("users/@me/connections".into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PATCH /users/@me/connections/{type}/{id} — toggle a connection's
    /// `visibility` (shown on profile) and `show_activity` (rich presence).
    pub async fn edit_connection(
        &self,
        conn_type: &str,
        conn_id: &str,
        visible: Option<bool>,
        show_activity: Option<bool>,
    ) -> Result<Value> {
        let mut body = Map::new();
        if let Some(v) = visible {
            body.insert("visibility".into(), json!(if v { 1 } else { 0 }));
        }
        if let Some(s) = show_activity {
            body.insert("show_activity".into(), json!(s));
        }
        if body.is_empty() {
            return Err(anyhow!("nothing to change: pass --visible and/or --show-activity"));
        }
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("users/@me/connections/{}/{}", conn_type, conn_id).into()),
                Value::Object(body),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /users/@me/connections/{type}/{id} — remove a linked account.
    pub async fn delete_connection(&self, conn_type: &str, conn_id: &str) -> Result<()> {
        self.raw()
            .delete(Route::Custom(
                format!("users/@me/connections/{}/{}", conn_type, conn_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// GET /users/@me/notes/{user_id} — fetch your private note on a user.
    /// Returns `{ "note_user_id", "user_id", "note" }` or 404 when no note.
    pub async fn get_note(&self, user_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(user_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("users/@me/notes/{}", user_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PUT /users/@me/notes/{user_id} — set (or clear, with empty string) your
    /// private note on a user.
    pub async fn set_note(&self, user_id: &str, note: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(user_id)?;
        let body = json!({ "note": note });
        self.raw()
            .put_optional::<Value, _>(
                Route::Custom(format!("users/@me/notes/{}", user_id).into()),
                body,
            )
            .await
            .map(|_| ())
            .map_err(|e| anyhow!(e.to_string()))
    }
}
