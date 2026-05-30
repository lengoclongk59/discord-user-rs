//! `discord dc guild ...` — guild settings & own membership.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum GuildCmd {
    /// Edit guild settings. Only provided flags are sent.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// New guild name.
        #[arg(long)]
        name: Option<String>,
        /// New guild description.
        #[arg(long)]
        description: Option<String>,
        /// AFK channel ID (snowflake).
        #[arg(long)]
        afk_channel: Option<String>,
        /// AFK timeout in seconds.
        #[arg(long)]
        afk_timeout: Option<u32>,
    },
    /// Set your own nickname in a guild.
    SetNick {
        /// Guild ID (snowflake).
        guild: String,
        /// New nickname.
        nick: String,
    },
    /// Leave a guild (confirmation prompt unless -y).
    Leave {
        /// Guild ID (snowflake).
        guild: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: GuildCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        GuildCmd::Edit {
            guild,
            name,
            description,
            afk_channel,
            afk_timeout,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(d) = description {
                body.insert("description".into(), serde_json::json!(d));
            }
            if let Some(c) = afk_channel {
                body.insert("afk_channel_id".into(), serde_json::json!(c));
            }
            if let Some(t) = afk_timeout {
                body.insert("afk_timeout".into(), serde_json::json!(t));
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api.guild_edit(&guild, serde_json::Value::Object(body)).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let gname = result["name"].as_str().unwrap_or("?");
                output::success(&format!("Updated guild '{}' ({})", gname, guild));
            }
        }

        GuildCmd::SetNick { guild, nick } => {
            let result = api.guild_set_nick(&guild, &nick).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Nickname set to '{}' in guild {}", nick, guild));
            }
        }

        GuildCmd::Leave { guild, yes } => {
            if !yes {
                print!("Leave guild {}? Type 'yes' to confirm: ", guild);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.guild_leave(&guild).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "left": guild }));
            } else {
                output::success(&format!("Left guild {}", guild));
            }
        }
    }

    Ok(())
}
