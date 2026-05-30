//! `discord dc member <CMD>` — guild member management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum MemberCmd {
    /// Set a member's nickname in a guild.
    SetNick {
        /// Guild snowflake ID.
        guild: String,
        /// User snowflake ID.
        user: String,
        /// New nickname (empty string to clear).
        nick: String,
    },
    /// Add a role to a guild member.
    AddRole {
        /// Guild snowflake ID.
        guild: String,
        /// User snowflake ID.
        user: String,
        /// Role snowflake ID.
        role: String,
    },
    /// Remove a role from a guild member.
    RemoveRole {
        /// Guild snowflake ID.
        guild: String,
        /// User snowflake ID.
        user: String,
        /// Role snowflake ID.
        role: String,
    },
    /// Timeout a guild member until an ISO-8601 timestamp.
    Timeout {
        /// Guild snowflake ID.
        guild: String,
        /// User snowflake ID.
        user: String,
        /// ISO-8601 datetime string (e.g. 2025-01-01T00:00:00Z).
        until: String,
        /// Reason recorded in the audit log.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Remove a timeout from a guild member.
    Untimeout {
        /// Guild snowflake ID.
        guild: String,
        /// User snowflake ID.
        user: String,
        /// Reason recorded in the audit log.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Kick a member from a guild.
    Kick {
        /// Guild snowflake ID.
        guild: String,
        /// User snowflake ID.
        user: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Reason recorded in the audit log.
        #[arg(long)]
        reason: Option<String>,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: MemberCmd) -> Result<()> {
    match cmd {
        MemberCmd::SetNick { guild, user, nick } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            let result = api.member_set_nick(&guild, &user, &nick).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Set nickname of {} in guild {} to {:?}",
                    user, guild, nick
                ));
            }
        }

        MemberCmd::AddRole { guild, user, role } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_add_role(&guild, &user, &role).await?;
            if ctx.json {
                output::print_json(&json!({
                    "guild_id": guild,
                    "user_id": user,
                    "role_id": role,
                    "action": "add_role",
                }));
            } else {
                output::success(&format!(
                    "Added role {} to user {} in guild {}",
                    role, user, guild
                ));
            }
        }

        MemberCmd::RemoveRole { guild, user, role } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_remove_role(&guild, &user, &role).await?;
            if ctx.json {
                output::print_json(&json!({
                    "guild_id": guild,
                    "user_id": user,
                    "role_id": role,
                    "action": "remove_role",
                }));
            } else {
                output::success(&format!(
                    "Removed role {} from user {} in guild {}",
                    role, user, guild
                ));
            }
        }

        MemberCmd::Timeout { guild, user, until, reason } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_timeout(&guild, &user, &until, reason.as_deref()).await?;
            if ctx.json {
                output::print_json(&json!({
                    "guild_id": guild,
                    "user_id": user,
                    "until": until,
                    "action": "timeout",
                }));
            } else {
                output::success(&format!(
                    "Timed out user {} in guild {} until {}",
                    user, guild, until
                ));
            }
        }

        MemberCmd::Untimeout { guild, user, reason } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_untimeout(&guild, &user, reason.as_deref()).await?;
            if ctx.json {
                output::print_json(&json!({
                    "guild_id": guild,
                    "user_id": user,
                    "action": "untimeout",
                }));
            } else {
                output::success(&format!(
                    "Removed timeout from user {} in guild {}",
                    user, guild
                ));
            }
        }

        MemberCmd::Kick { guild, user, yes, reason } => {
            if !yes {
                print!(
                    "Kick user {} from guild {}? Type 'yes' to confirm: ",
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

            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_kick(&guild, &user, reason.as_deref()).await?;
            if ctx.json {
                output::print_json(&json!({
                    "guild_id": guild,
                    "user_id": user,
                    "action": "kick",
                }));
            } else {
                output::success(&format!("Kicked user {} from guild {}", user, guild));
            }
        }
    }

    Ok(())
}
