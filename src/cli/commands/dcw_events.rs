//! `discord dc event ...` — guild scheduled event management.

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum EventCmd {
    /// Create a guild scheduled event.
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Event name.
        name: String,
        /// Scheduled start time in ISO-8601 format (e.g. 2026-06-01T18:00:00Z).
        start_iso: String,
        /// Event type: voice, stage, or external (default: external).
        #[arg(long, default_value = "external")]
        r#type: String,
        /// Voice/stage channel ID (required for voice and stage types).
        #[arg(long)]
        channel: Option<String>,
        /// Location string (required for external type).
        #[arg(long)]
        location: Option<String>,
        /// Scheduled end time in ISO-8601 format (required for external type).
        #[arg(long)]
        end_iso: Option<String>,
        /// Event description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Edit an existing guild scheduled event. Only provided flags are sent.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// Event ID (snowflake).
        event_id: String,
        /// New event name.
        #[arg(long)]
        name: Option<String>,
        /// New scheduled start time in ISO-8601 format.
        #[arg(long)]
        start_iso: Option<String>,
        /// New scheduled end time in ISO-8601 format.
        #[arg(long)]
        end_iso: Option<String>,
        /// New event status: scheduled, active, completed, or canceled.
        #[arg(long)]
        status: Option<String>,
        /// New event description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a guild scheduled event (with confirmation prompt unless -y).
    Delete {
        /// Guild ID (snowflake).
        guild: String,
        /// Event ID (snowflake).
        event_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: EventCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        EventCmd::Create {
            guild,
            name,
            start_iso,
            r#type,
            channel,
            location,
            end_iso,
            description,
        } => {
            let entity_type = match r#type.to_lowercase().as_str() {
                "stage" => 1u32,
                "voice" => 2u32,
                "external" => 3u32,
                other => {
                    return Err(anyhow::anyhow!(
                        "Unknown event type '{}'. Use voice, stage, or external.",
                        other
                    ));
                }
            };

            let mut body = serde_json::Map::new();
            body.insert("name".into(), serde_json::json!(name));
            body.insert("scheduled_start_time".into(), serde_json::json!(start_iso));
            body.insert("privacy_level".into(), serde_json::json!(2));
            body.insert("entity_type".into(), serde_json::json!(entity_type));

            match entity_type {
                1 | 2 => {
                    // STAGE or VOICE: channel_id required
                    match &channel {
                        Some(ch) => {
                            body.insert("channel_id".into(), serde_json::json!(ch));
                        }
                        None => {
                            return Err(anyhow::anyhow!(
                                "--channel is required for voice and stage event types."
                            ));
                        }
                    }
                }
                3 => {
                    // EXTERNAL: location and end time required
                    match &location {
                        Some(loc) => {
                            body.insert(
                                "entity_metadata".into(),
                                serde_json::json!({ "location": loc }),
                            );
                        }
                        None => {
                            return Err(anyhow::anyhow!(
                                "--location is required for external event type."
                            ));
                        }
                    }
                    match &end_iso {
                        Some(end) => {
                            body.insert("scheduled_end_time".into(), serde_json::json!(end));
                        }
                        None => {
                            return Err(anyhow::anyhow!(
                                "--end-iso is required for external event type."
                            ));
                        }
                    }
                }
                _ => {}
            }

            if let Some(desc) = description {
                body.insert("description".into(), serde_json::json!(desc));
            }

            let result = api
                .event_create(&guild, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                let ename = result["name"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created event '{}' ({}) in guild {}",
                    ename, id, guild
                ));
            }
        }

        EventCmd::Edit {
            guild,
            event_id,
            name,
            start_iso,
            end_iso,
            status,
            description,
        } => {
            let mut body = serde_json::Map::new();

            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(s) = start_iso {
                body.insert("scheduled_start_time".into(), serde_json::json!(s));
            }
            if let Some(e) = end_iso {
                body.insert("scheduled_end_time".into(), serde_json::json!(e));
            }
            if let Some(st) = status {
                let code = match st.to_lowercase().as_str() {
                    "scheduled" => 1u32,
                    "active" => 2u32,
                    "completed" => 3u32,
                    "canceled" | "cancelled" => 4u32,
                    other => {
                        return Err(anyhow::anyhow!(
                            "Unknown status '{}'. Use scheduled, active, completed, or canceled.",
                            other
                        ));
                    }
                };
                body.insert("status".into(), serde_json::json!(code));
            }
            if let Some(d) = description {
                body.insert("description".into(), serde_json::json!(d));
            }

            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }

            let result = api
                .event_edit(&guild, &event_id, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let ename = result["name"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Updated event '{}' ({}) in guild {}",
                    ename, event_id, guild
                ));
            }
        }

        EventCmd::Delete {
            guild,
            event_id,
            yes,
        } => {
            if !yes && !output::confirm(&format!("Delete event {} in guild {}?", event_id, guild))?
            {
                return Ok(());
            }
            api.event_delete(&guild, &event_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "deleted": event_id,
                    "guild_id": guild,
                }));
            } else {
                output::success(&format!("Deleted event {} in guild {}", event_id, guild));
            }
        }
    }

    Ok(())
}
