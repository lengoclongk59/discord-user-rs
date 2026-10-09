//! `discord dc notify ...` — user notification settings & read-state
//! acknowledgement (mute guilds/channels, set notification level, mark read).

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::api_usersettings::parse_notification_level;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum NotifyCmd {
    /// Set guild-level notification settings.
    Guild {
        /// Guild ID (snowflake).
        guild: String,
        /// Mute (true) or unmute (false) the whole guild.
        #[arg(long)]
        muted: Option<bool>,
        /// Notification level: all | mentions | nothing.
        #[arg(long)]
        level: Option<String>,
        /// Suppress @everyone / @here pings.
        #[arg(long = "suppress-everyone")]
        suppress_everyone: Option<bool>,
        /// Suppress role mentions.
        #[arg(long = "suppress-roles")]
        suppress_roles: Option<bool>,
    },
    /// Set a per-channel notification override. Use guild `@me` for DMs.
    Channel {
        /// Guild ID (snowflake), or `@me` for a DM / group DM.
        guild: String,
        /// Channel ID (snowflake).
        channel: String,
        /// Mute (true) or unmute (false) this channel.
        #[arg(long)]
        muted: Option<bool>,
        /// Notification level: all | mentions | nothing.
        #[arg(long)]
        level: Option<String>,
    },
    /// Mark a channel read up to a given message.
    ReadChannel {
        /// Channel ID (snowflake).
        channel: String,
        /// Message ID to mark read up to (snowflake).
        message: String,
    },
    /// Mark every channel in a guild read.
    ReadGuild {
        /// Guild ID (snowflake).
        guild: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: NotifyCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        NotifyCmd::Guild {
            guild,
            muted,
            level,
            suppress_everyone,
            suppress_roles,
        } => {
            let level = level.as_deref().map(parse_notification_level).transpose()?;
            let result = api
                .notif_guild_settings(&guild, muted, level, suppress_everyone, suppress_roles)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Updated notification settings for guild {}",
                    guild
                ));
            }
        }

        NotifyCmd::Channel {
            guild,
            channel,
            muted,
            level,
        } => {
            let level = level.as_deref().map(parse_notification_level).transpose()?;
            let result = api
                .notif_channel_override(&guild, &channel, muted, level)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Updated notification override for channel {}",
                    channel
                ));
            }
        }

        NotifyCmd::ReadChannel { channel, message } => {
            api.ack_channel(&channel, &message).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "channel_id": channel,
                    "message_id": message,
                    "acked": true,
                }));
            } else {
                output::success(&format!(
                    "Marked channel {} read up to message {}",
                    channel, message
                ));
            }
        }

        NotifyCmd::ReadGuild { guild } => {
            api.ack_guild(&guild).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "guild_id": guild, "acked": true }));
            } else {
                output::success(&format!("Marked all channels in guild {} read", guild));
            }
        }
    }

    Ok(())
}
