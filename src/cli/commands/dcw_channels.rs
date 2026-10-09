//! `discord dc channel ...` — channel management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum ChannelCmd {
    /// Create a channel in a guild.
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Channel name.
        name: String,
        /// Channel type (0=text, 2=voice, 4=category, 5=announcement, 15=forum). Default: 0.
        #[arg(long, default_value_t = 0)]
        r#type: u8,
        /// Channel topic.
        #[arg(long)]
        topic: Option<String>,
        /// Parent category channel ID.
        #[arg(long)]
        parent: Option<String>,
        /// Mark channel as NSFW.
        #[arg(long)]
        nsfw: bool,
        /// Slowmode delay in seconds (rate_limit_per_user).
        #[arg(long)]
        slowmode: Option<u64>,
    },
    /// Edit an existing channel. Only provided flags are sent.
    Edit {
        /// Channel ID (snowflake).
        channel: String,
        /// New channel name.
        #[arg(long)]
        name: Option<String>,
        /// New channel topic.
        #[arg(long)]
        topic: Option<String>,
        /// Set NSFW flag (true/false).
        #[arg(long)]
        nsfw: Option<bool>,
        /// Slowmode delay in seconds.
        #[arg(long)]
        slowmode: Option<u64>,
        /// Voice channel bitrate (bits/s).
        #[arg(long)]
        bitrate: Option<u64>,
        /// Channel position in the channel list.
        #[arg(long)]
        position: Option<u64>,
    },
    /// Delete a channel (with confirmation prompt unless -y).
    Delete {
        /// Channel ID (snowflake).
        channel: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Audit-log reason for the deletion.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Set slowmode delay on a channel.
    Slowmode {
        /// Channel ID (snowflake).
        channel: String,
        /// Delay in seconds (0 to disable).
        seconds: u64,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: ChannelCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        ChannelCmd::Create {
            guild,
            name,
            r#type,
            topic,
            parent,
            nsfw,
            slowmode,
        } => {
            let result = api
                .channel_create(
                    &guild,
                    &name,
                    r#type,
                    topic.as_deref(),
                    parent.as_deref(),
                    nsfw,
                    slowmode,
                )
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!("Created channel '{}' ({})", name, id));
            }
        }

        ChannelCmd::Edit {
            channel,
            name,
            topic,
            nsfw,
            slowmode,
            bitrate,
            position,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(t) = topic {
                body.insert("topic".into(), serde_json::json!(t));
            }
            if let Some(b) = nsfw {
                body.insert("nsfw".into(), serde_json::json!(b));
            }
            if let Some(s) = slowmode {
                body.insert("rate_limit_per_user".into(), serde_json::json!(s));
            }
            if let Some(b) = bitrate {
                body.insert("bitrate".into(), serde_json::json!(b));
            }
            if let Some(p) = position {
                body.insert("position".into(), serde_json::json!(p));
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api
                .channel_edit(&channel, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let name = result["name"].as_str().unwrap_or("?");
                output::success(&format!("Updated channel '{}' ({})", name, channel));
            }
        }

        ChannelCmd::Delete {
            channel,
            yes,
            reason,
        } => {
            if !yes {
                print!("Delete channel {}? Type 'yes' to confirm: ", channel);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.channel_delete(&channel, reason.as_deref()).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "deleted": channel }));
            } else {
                output::success(&format!("Deleted channel {}", channel));
            }
        }

        ChannelCmd::Slowmode { channel, seconds } => {
            let result = api.channel_set_slowmode(&channel, seconds).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Set slowmode to {}s on channel {}",
                    seconds, channel
                ));
            }
        }
    }

    Ok(())
}
