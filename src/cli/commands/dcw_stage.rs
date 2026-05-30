//! `discord dc stage ...` — stage instance management.

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::api_stage::{build_edit_body, build_start_body};
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum StageCmd {
    /// Start a stage instance in a stage channel.
    Start {
        /// Stage channel ID (snowflake).
        channel: String,
        /// Stage topic (1-120 characters).
        topic: String,
        /// Privacy level: 1 = PUBLIC, 2 = GUILD_ONLY (default).
        #[arg(long, default_value_t = 2)]
        privacy: u8,
        /// Notify @everyone that the stage has started (requires MENTION_EVERYONE).
        #[arg(long)]
        send_start_notification: bool,
    },
    /// Get the active stage instance for a stage channel.
    Get {
        /// Stage channel ID (snowflake).
        channel: String,
    },
    /// Edit an active stage instance. Only provided flags are sent.
    Edit {
        /// Stage channel ID (snowflake).
        channel: String,
        /// New topic (1-120 characters).
        #[arg(long)]
        topic: Option<String>,
        /// New privacy level: 1 = PUBLIC, 2 = GUILD_ONLY.
        #[arg(long)]
        privacy: Option<u8>,
    },
    /// Stop (end) an active stage instance (with confirmation prompt unless -y).
    Stop {
        /// Stage channel ID (snowflake).
        channel: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: StageCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        StageCmd::Start {
            channel,
            topic,
            privacy,
            send_start_notification,
        } => {
            let body = build_start_body(&channel, &topic, privacy, send_start_notification)?;
            let result = api.stage_start(body).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Started stage instance {} in channel {}",
                    id, channel
                ));
            }
        }

        StageCmd::Get { channel } => {
            let result = api.stage_get(&channel).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let topic = result["topic"].as_str().unwrap_or("?");
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Stage instance {} in channel {}: '{}'",
                    id, channel, topic
                ));
            }
        }

        StageCmd::Edit {
            channel,
            topic,
            privacy,
        } => {
            let body = build_edit_body(topic.as_deref(), privacy)?;
            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api.stage_edit(&channel, body).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let new_topic = result["topic"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Updated stage instance in channel {}: '{}'",
                    channel, new_topic
                ));
            }
        }

        StageCmd::Stop { channel, yes } => {
            if !yes
                && !output::confirm(&format!("Stop stage instance in channel {}?", channel))?
            {
                return Ok(());
            }
            api.stage_stop(&channel).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "stopped": channel,
                }));
            } else {
                output::success(&format!("Stopped stage instance in channel {}", channel));
            }
        }
    }

    Ok(())
}
