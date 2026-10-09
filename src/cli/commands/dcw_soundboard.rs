//! `discord dc soundboard ...` — soundboard sound management.
//!
//! Covers Discord's Soundboard REST API: the built-in default sounds, per-guild
//! sound CRUD under `/guilds/{guild_id}/soundboard-sounds`, and sending a sound
//! into a voice channel via `/channels/{channel_id}/send-soundboard-sound`.

use std::io::{self, Write};

use anyhow::{anyhow, Result};
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum SoundboardCmd {
    /// List Discord's built-in default soundboard sounds.
    ListDefault,
    /// List soundboard sounds uploaded to a guild.
    List {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Get a single guild soundboard sound by ID.
    Get {
        /// Guild ID (snowflake).
        guild: String,
        /// Soundboard sound ID (snowflake).
        sound_id: String,
    },
    /// Upload a new soundboard sound to a guild from an audio file (mp3/ogg).
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Sound name (2–32 chars).
        name: String,
        /// Path to the audio file (mp3 or ogg, max 512 KB).
        file: String,
        /// Playback volume, 0.0–1.0.
        #[arg(long, default_value_t = 1.0)]
        volume: f64,
        /// Custom emoji ID to associate (snowflake).
        #[arg(long)]
        emoji_id: Option<String>,
        /// Unicode emoji to associate.
        #[arg(long)]
        emoji_name: Option<String>,
    },
    /// Edit a guild soundboard sound. Only provided flags are sent.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// Soundboard sound ID (snowflake).
        sound_id: String,
        /// New sound name.
        #[arg(long)]
        name: Option<String>,
        /// New playback volume, 0.0–1.0.
        #[arg(long)]
        volume: Option<f64>,
        /// Custom emoji ID to associate (snowflake).
        #[arg(long)]
        emoji_id: Option<String>,
        /// Unicode emoji to associate.
        #[arg(long)]
        emoji_name: Option<String>,
    },
    /// Delete a guild soundboard sound (with confirmation prompt unless -y).
    Delete {
        /// Guild ID (snowflake).
        guild: String,
        /// Soundboard sound ID (snowflake).
        sound_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Send a soundboard sound into a voice channel you are connected to.
    Send {
        /// Voice channel ID (snowflake).
        channel: String,
        /// Soundboard sound ID (snowflake).
        sound_id: String,
        /// Source guild ID for a guild sound (omit for default sounds).
        #[arg(long)]
        source_guild: Option<String>,
    },
}

/// Validate a playback volume is within Discord's 0.0–1.0 range.
fn validate_volume(volume: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&volume) {
        return Err(anyhow!(
            "volume must be between 0.0 and 1.0 (got {})",
            volume
        ));
    }
    Ok(())
}

pub async fn dispatch(ctx: &Ctx, cmd: SoundboardCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        SoundboardCmd::ListDefault => {
            let result = api.soundboard_list_default().await?;
            output::print_json(&result);
        }

        SoundboardCmd::List { guild } => {
            let result = api.soundboard_list(&guild).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let count = result
                    .get("items")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                output::success(&format!("{} soundboard sound(s) in guild {}", count, guild));
            }
        }

        SoundboardCmd::Get { guild, sound_id } => {
            let result = api.soundboard_get(&guild, &sound_id).await?;
            output::print_json(&result);
        }

        SoundboardCmd::Create {
            guild,
            name,
            file,
            volume,
            emoji_id,
            emoji_name,
        } => {
            validate_volume(volume)?;
            if !std::path::Path::new(&file).is_file() {
                return Err(anyhow!("audio file not found: {}", file));
            }
            let result = api
                .soundboard_create(
                    &guild,
                    &name,
                    &file,
                    volume,
                    emoji_id.as_deref(),
                    emoji_name.as_deref(),
                )
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result
                    .get("sound_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                output::success(&format!(
                    "Created soundboard sound '{}' (id={}) in guild {}",
                    name, id, guild
                ));
            }
        }

        SoundboardCmd::Edit {
            guild,
            sound_id,
            name,
            volume,
            emoji_id,
            emoji_name,
        } => {
            if let Some(v) = volume {
                validate_volume(v)?;
            }
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(v) = volume {
                body.insert("volume".into(), serde_json::json!(v));
            }
            if let Some(eid) = emoji_id {
                crate::cli::api::parse_snowflake(&eid)?;
                body.insert("emoji_id".into(), serde_json::json!(eid));
            }
            if let Some(ename) = emoji_name {
                body.insert("emoji_name".into(), serde_json::json!(ename));
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api
                .soundboard_edit(&guild, &sound_id, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Updated soundboard sound {} in guild {}",
                    sound_id, guild
                ));
            }
        }

        SoundboardCmd::Delete {
            guild,
            sound_id,
            yes,
        } => {
            if !yes {
                print!(
                    "Delete soundboard sound {} from guild {}? Type 'yes' to confirm: ",
                    sound_id, guild
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.soundboard_delete(&guild, &sound_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "deleted": sound_id,
                    "guild_id": guild,
                }));
            } else {
                output::success(&format!(
                    "Deleted soundboard sound {} from guild {}",
                    sound_id, guild
                ));
            }
        }

        SoundboardCmd::Send {
            channel,
            sound_id,
            source_guild,
        } => {
            api.soundboard_send(&channel, &sound_id, source_guild.as_deref())
                .await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "sent": sound_id,
                    "channel_id": channel,
                    "source_guild_id": source_guild,
                }));
            } else {
                output::success(&format!(
                    "Sent soundboard sound {} to channel {}",
                    sound_id, channel
                ));
            }
        }
    }

    Ok(())
}
