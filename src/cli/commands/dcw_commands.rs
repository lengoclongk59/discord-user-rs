//! `discord dc command ...` — application (slash) command management.
//!
//! Application-command *management* (list, get, create, delete) works cleanly
//! over REST for bot/application tokens. The application id is resolved up
//! front via `GET /oauth2/applications/@me`.
//!
//! Note: actually *invoking* a slash command as a user requires the
//! undocumented interactions gateway flow (session_id + nonce) and is not
//! supported over REST — `run` prints an explanatory error rather than faking
//! it.

use anyhow::{anyhow, Result};
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum AppCommandCmd {
    /// List application (slash) commands — global, or guild-scoped with --guild.
    List {
        /// Restrict to a guild's commands (snowflake).
        #[arg(long)]
        guild: Option<String>,
    },
    /// Get a single application command by ID.
    Get {
        /// Command ID (snowflake).
        command_id: String,
        /// Look up the command in a guild scope (snowflake).
        #[arg(long)]
        guild: Option<String>,
    },
    /// Register a CHAT_INPUT application command (works for bot/application tokens).
    Create {
        /// Command name (lowercase, 1-32 chars).
        name: String,
        /// Command description (1-100 chars).
        description: String,
        /// Register in a guild scope (snowflake); omit for a global command.
        #[arg(long)]
        guild: Option<String>,
        /// Command type: 1=CHAT_INPUT (default), 2=USER, 3=MESSAGE.
        #[arg(long = "type", default_value_t = 1)]
        command_type: u8,
    },
    /// Delete an application command (with confirmation prompt unless -y).
    Delete {
        /// Command ID (snowflake).
        command_id: String,
        /// Delete from a guild scope (snowflake).
        #[arg(long)]
        guild: Option<String>,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Invoke a slash command as a user — NOT supported over REST (see notes).
    Run {
        /// Channel ID to invoke the command in (snowflake).
        channel: String,
        /// Slash command name to invoke.
        command_name: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: AppCommandCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        AppCommandCmd::List { guild } => {
            let app_id = api.app_id().await?;
            let result = api.commands_list(&app_id, guild.as_deref()).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let count = result.as_array().map(|a| a.len()).unwrap_or(0);
                let scope = match &guild {
                    Some(g) => format!("guild {}", g),
                    None => "global".to_string(),
                };
                output::success(&format!(
                    "{} application command(s) [{}] for app {}",
                    count, scope, app_id
                ));
            }
        }

        AppCommandCmd::Get { command_id, guild } => {
            let app_id = api.app_id().await?;
            let result = api
                .command_get(&app_id, guild.as_deref(), &command_id)
                .await?;
            output::print_json(&result);
        }

        AppCommandCmd::Create {
            name,
            description,
            guild,
            command_type,
        } => {
            let app_id = api.app_id().await?;
            let result = api
                .command_create(&app_id, guild.as_deref(), &name, &description, command_type)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                let cname = result["name"].as_str().unwrap_or(&name);
                let scope = match &guild {
                    Some(g) => format!("guild {}", g),
                    None => "global".to_string(),
                };
                output::success(&format!(
                    "Created application command '{}' ({}) [{}]",
                    cname, id, scope
                ));
            }
        }

        AppCommandCmd::Delete {
            command_id,
            guild,
            yes,
        } => {
            let scope = match &guild {
                Some(g) => format!("guild {}", g),
                None => "global".to_string(),
            };
            if !yes
                && !output::confirm(&format!(
                    "Delete application command {} [{}]?",
                    command_id, scope
                ))?
            {
                return Ok(());
            }
            let app_id = api.app_id().await?;
            api.command_delete(&app_id, guild.as_deref(), &command_id)
                .await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "deleted": command_id,
                    "guild_id": guild,
                }));
            } else {
                output::success(&format!(
                    "Deleted application command {} [{}]",
                    command_id, scope
                ));
            }
        }

        AppCommandCmd::Run {
            channel,
            command_name,
        } => {
            // Invoking a slash command as a user is an INTERACTION, not a REST
            // resource. It requires the gateway flow (a live session_id plus a
            // per-invocation nonce posted to the undocumented `/interactions`
            // endpoint) and is fragile/unsupported here. Fail with a clear
            // explanation rather than faking a request.
            return Err(anyhow!(
                "Invoking slash command '{}' in channel {} is not supported over REST.\n\
                 Running a slash command as a user requires the interactions gateway flow \
                 (a live session_id + nonce posted to the undocumented /interactions endpoint), \
                 which this CLI does not implement. Use `dc command list` to inspect registered \
                 commands, or `dc command create` to register one (bot/application tokens).",
                command_name,
                channel
            ));
        }
    }

    Ok(())
}
