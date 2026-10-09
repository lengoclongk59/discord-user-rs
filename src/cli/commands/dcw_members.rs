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
    /// Dry-run: count members who would be removed by a prune.
    PruneCount {
        /// Guild snowflake ID.
        guild: String,
        /// Days of inactivity (1–30).
        #[arg(long, default_value_t = 7)]
        days: u32,
        /// Comma-separated role IDs to include in the count (optional).
        #[arg(long)]
        include_roles: Option<String>,
    },
    /// Prune (remove) members inactive for N days.
    Prune {
        /// Guild snowflake ID.
        guild: String,
        /// Days of inactivity (1–30).
        #[arg(long, default_value_t = 7)]
        days: u32,
        /// Comma-separated role IDs to include in the prune (optional).
        #[arg(long)]
        include_roles: Option<String>,
        /// Compute and return the pruned count (slower on large guilds).
        #[arg(long)]
        count: bool,
        /// Reason recorded in the audit log.
        #[arg(long)]
        reason: Option<String>,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Search members by username/nickname prefix.
    Search {
        /// Guild snowflake ID.
        guild: String,
        /// Query string (username/nickname prefix).
        query: String,
        /// Maximum results (1–1000).
        #[arg(short = 'n', long, default_value_t = 25)]
        limit: u32,
    },
}

/// Parse an optional comma-separated snowflake list into a `Vec<String>`.
fn parse_id_list(s: Option<&str>) -> Vec<String> {
    s.map(|v| {
        v.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()
    })
    .unwrap_or_default()
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

        MemberCmd::Timeout {
            guild,
            user,
            until,
            reason,
        } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_timeout(&guild, &user, &until, reason.as_deref())
                .await?;
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

        MemberCmd::Untimeout {
            guild,
            user,
            reason,
        } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.member_untimeout(&guild, &user, reason.as_deref())
                .await?;
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

        MemberCmd::Kick {
            guild,
            user,
            yes,
            reason,
        } => {
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

        MemberCmd::PruneCount {
            guild,
            days,
            include_roles,
        } => {
            let roles = parse_id_list(include_roles.as_deref());
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            let result = api.member_prune_count(&guild, days, &roles).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let pruned = result["pruned"].as_u64().unwrap_or(0);
                output::success(&format!(
                    "{} member(s) inactive for {}+ days would be pruned in guild {}",
                    pruned, days, guild
                ));
            }
        }

        MemberCmd::Prune {
            guild,
            days,
            include_roles,
            count,
            reason,
            yes,
        } => {
            let roles = parse_id_list(include_roles.as_deref());
            if !yes {
                print!(
                    "Prune members inactive {}+ days from guild {}? Type 'yes' to confirm: ",
                    days, guild
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
            let result = api
                .member_prune_begin(&guild, days, count, &roles, reason.as_deref())
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                match result["pruned"].as_u64() {
                    Some(n) => {
                        output::success(&format!("Pruned {} member(s) from guild {}", n, guild))
                    }
                    None => output::success(&format!("Prune started in guild {}", guild)),
                }
            }
        }

        MemberCmd::Search {
            guild,
            query,
            limit,
        } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            let result = api.member_search(&guild, &query, limit).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let count = result.as_array().map(|a| a.len()).unwrap_or(0);
                output::success(&format!(
                    "{} member(s) matching '{}' in guild {}",
                    count, query, guild
                ));
            }
        }
    }

    Ok(())
}
