//! `discord dc ban ...` — guild ban management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum BanCmd {
    /// Add a ban for a user in a guild.
    Add {
        /// Guild ID (snowflake).
        guild: String,
        /// User ID (snowflake).
        user: String,
        /// Number of seconds of messages to delete (0–604800).
        #[arg(long, default_value_t = 0)]
        delete_seconds: u32,
        /// Audit log reason.
        #[arg(long)]
        reason: Option<String>,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Remove a ban for a user in a guild.
    Remove {
        /// Guild ID (snowflake).
        guild: String,
        /// User ID (snowflake).
        user: String,
        /// Audit log reason.
        #[arg(long)]
        reason: Option<String>,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// List all bans for a guild.
    List {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Get a specific ban entry for a user in a guild.
    Get {
        /// Guild ID (snowflake).
        guild: String,
        /// User ID (snowflake).
        user: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: BanCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        BanCmd::Add {
            guild,
            user,
            delete_seconds,
            reason,
            yes,
        } => {
            if !yes {
                print!(
                    "Ban user {} from guild {}? Type 'yes' to confirm: ",
                    user, guild
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.ban_add(&guild, &user, delete_seconds, reason.as_deref())
                .await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "banned": user,
                    "guild_id": guild,
                    "delete_message_seconds": delete_seconds,
                }));
            } else {
                output::success(&format!("Banned user {} from guild {}", user, guild));
            }
        }

        BanCmd::Remove {
            guild,
            user,
            reason,
            yes,
        } => {
            if !yes {
                print!(
                    "Unban user {} from guild {}? Type 'yes' to confirm: ",
                    user, guild
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.ban_remove(&guild, &user, reason.as_deref()).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "unbanned": user,
                    "guild_id": guild,
                }));
            } else {
                output::success(&format!("Unbanned user {} from guild {}", user, guild));
            }
        }

        BanCmd::List { guild } => {
            let result = api.ban_list(&guild).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let count = result.as_array().map(|a| a.len()).unwrap_or(0);
                output::success(&format!("{} ban(s) in guild {}", count, guild));
            }
        }

        BanCmd::Get { guild, user } => {
            let result = api.ban_get(&guild, &user).await?;
            output::print_json(&result);
        }
    }

    Ok(())
}
