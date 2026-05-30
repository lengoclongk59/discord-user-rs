//! `discord dc role ...` — guild role management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum RoleCmd {
    /// Create a role in a guild.
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Role name.
        #[arg(long)]
        name: Option<String>,
        /// Role color as a decimal integer (e.g. 16711680 for red).
        #[arg(long)]
        color: Option<u32>,
        /// Display role members separately (hoist).
        #[arg(long)]
        hoist: bool,
        /// Allow anyone to @mention this role.
        #[arg(long)]
        mentionable: bool,
        /// Permission bitfield string (e.g. "8" for Administrator).
        #[arg(long)]
        permissions: Option<String>,
    },
    /// Edit an existing role. Only provided flags are sent.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// Role ID (snowflake).
        role: String,
        /// New role name.
        #[arg(long)]
        name: Option<String>,
        /// New role color as a decimal integer.
        #[arg(long)]
        color: Option<u32>,
        /// Set hoist flag (true/false).
        #[arg(long)]
        hoist: Option<bool>,
        /// Set mentionable flag (true/false).
        #[arg(long)]
        mentionable: Option<bool>,
        /// New permission bitfield string.
        #[arg(long)]
        permissions: Option<String>,
    },
    /// Delete a role (with confirmation prompt unless -y).
    Delete {
        /// Guild ID (snowflake).
        guild: String,
        /// Role ID (snowflake).
        role: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// List all roles in a guild.
    List {
        /// Guild ID (snowflake).
        guild: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: RoleCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        RoleCmd::Create {
            guild,
            name,
            color,
            hoist,
            mentionable,
            permissions,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(c) = color {
                body.insert("color".into(), serde_json::json!(c));
            }
            if hoist {
                body.insert("hoist".into(), serde_json::json!(true));
            }
            if mentionable {
                body.insert("mentionable".into(), serde_json::json!(true));
            }
            if let Some(p) = permissions {
                body.insert("permissions".into(), serde_json::json!(p));
            }
            let result = api.role_create(&guild, serde_json::Value::Object(body)).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                let rname = result["name"].as_str().unwrap_or("?");
                output::success(&format!("Created role '{}' ({}) in guild {}", rname, id, guild));
            }
        }

        RoleCmd::Edit {
            guild,
            role,
            name,
            color,
            hoist,
            mentionable,
            permissions,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(c) = color {
                body.insert("color".into(), serde_json::json!(c));
            }
            if let Some(h) = hoist {
                body.insert("hoist".into(), serde_json::json!(h));
            }
            if let Some(m) = mentionable {
                body.insert("mentionable".into(), serde_json::json!(m));
            }
            if let Some(p) = permissions {
                body.insert("permissions".into(), serde_json::json!(p));
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api.role_edit(&guild, &role, serde_json::Value::Object(body)).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let rname = result["name"].as_str().unwrap_or("?");
                output::success(&format!("Updated role '{}' ({}) in guild {}", rname, role, guild));
            }
        }

        RoleCmd::Delete { guild, role, yes } => {
            if !yes {
                print!("Delete role {} in guild {}? Type 'yes' to confirm: ", role, guild);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.role_delete(&guild, &role).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "deleted": role, "guild_id": guild }));
            } else {
                output::success(&format!("Deleted role {} in guild {}", role, guild));
            }
        }

        RoleCmd::List { guild } => {
            let result = api.role_list(&guild).await?;
            output::print_json(&result);
        }
    }

    Ok(())
}
