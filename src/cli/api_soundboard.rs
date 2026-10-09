//! `WriteApi` extensions: soundboard sounds.
//!
//! Wraps Discord's Soundboard REST API: the default-sound list, per-guild
//! sound CRUD (`/guilds/{guild_id}/soundboard-sounds`), and sending a sound
//! into a voice channel (`/channels/{channel_id}/send-soundboard-sound`). The
//! list/get reads live here alongside the mutating verbs since the whole
//! group is soundboard management — mirroring `api_automod.rs`.
use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// List Discord's built-in default soundboard sounds. Returns a JSON
    /// object with a `items` array.
    pub async fn soundboard_list_default(&self) -> Result<Value> {
        self.raw()
            .get::<Value>(Route::Custom("soundboard-default-sounds".into()))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// List all soundboard sounds uploaded to a guild.
    pub async fn soundboard_list(&self, guild_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/soundboard-sounds", guild_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Get a single guild soundboard sound by ID.
    pub async fn soundboard_get(&self, guild_id: &str, sound_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(sound_id)?;
        self.raw()
            .get::<Value>(Route::Custom(
                format!("guilds/{}/soundboard-sounds/{}", guild_id, sound_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Create a guild soundboard sound from a local audio file (mp3/ogg).
    ///
    /// The file is base64-encoded into a `data:audio/...;base64,...` URI for
    /// the `sound` field. `emoji_id` and `emoji_name` are sent only when
    /// provided.
    #[allow(clippy::too_many_arguments)]
    pub async fn soundboard_create(
        &self,
        guild_id: &str,
        name: &str,
        file_path: &str,
        volume: f64,
        emoji_id: Option<&str>,
        emoji_name: Option<&str>,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        if let Some(eid) = emoji_id {
            crate::cli::api::parse_snowflake(eid)?;
        }
        let bytes = std::fs::read(file_path).with_context(|| format!("reading {}", file_path))?;
        let mime = if file_path.ends_with(".ogg") {
            "audio/ogg"
        } else {
            "audio/mpeg"
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let data_uri = format!("data:{};base64,{}", mime, b64);
        let mut body = json!({
            "name": name,
            "sound": data_uri,
            "volume": volume,
        });
        if let Some(eid) = emoji_id {
            body["emoji_id"] = json!(eid);
        }
        if let Some(ename) = emoji_name {
            body["emoji_name"] = json!(ename);
        }
        self.raw()
            .post::<Value, _>(
                Route::Custom(format!("guilds/{}/soundboard-sounds", guild_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit a guild soundboard sound. Only fields present in `body` are sent.
    pub async fn soundboard_edit(
        &self,
        guild_id: &str,
        sound_id: &str,
        body: Value,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(sound_id)?;
        self.raw()
            .patch::<Value, _>(
                Route::Custom(format!("guilds/{}/soundboard-sounds/{}", guild_id, sound_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Delete a guild soundboard sound permanently.
    pub async fn soundboard_delete(&self, guild_id: &str, sound_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(guild_id)?;
        crate::cli::api::parse_snowflake(sound_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("guilds/{}/soundboard-sounds/{}", guild_id, sound_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Send a soundboard sound into a voice channel. The current user must be
    /// connected to that channel. `source_guild` is required for guild sounds
    /// (the guild the sound belongs to) and omitted for default sounds.
    pub async fn soundboard_send(
        &self,
        channel_id: &str,
        sound_id: &str,
        source_guild: Option<&str>,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(sound_id)?;
        if let Some(gid) = source_guild {
            crate::cli::api::parse_snowflake(gid)?;
        }
        let mut body = json!({ "sound_id": sound_id });
        if let Some(gid) = source_guild {
            body["source_guild_id"] = json!(gid);
        }
        self.raw()
            .post_no_response(
                Route::Custom(format!("channels/{}/send-soundboard-sound", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
