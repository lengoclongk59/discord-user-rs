//! `discord dc dm-group ...` — group DM management.

use anyhow::{anyhow, Result};
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum DmGroupCmd {
    /// Create a group DM with a comma-separated list of recipient user IDs (2+).
    Create {
        /// Recipient user IDs, comma-separated (e.g. `123,456`). At least 2.
        users: String,
    },
    /// Add a recipient to a group DM.
    Add {
        /// Group DM channel ID (snowflake).
        channel: String,
        /// Recipient user ID (snowflake).
        user: String,
    },
    /// Remove a recipient from a group DM.
    Remove {
        /// Group DM channel ID (snowflake).
        channel: String,
        /// Recipient user ID (snowflake).
        user: String,
    },
    /// Rename a group DM.
    SetName {
        /// Group DM channel ID (snowflake).
        channel: String,
        /// New group DM name.
        name: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: DmGroupCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        DmGroupCmd::Create { users } => {
            let recipients: Vec<String> = users
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if recipients.len() < 2 {
                return Err(anyhow!(
                    "a group DM needs at least 2 recipients (got {})",
                    recipients.len()
                ));
            }
            let result = api.create_group_dm(&recipients).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created group DM {} with {} recipients",
                    id,
                    recipients.len()
                ));
            }
        }

        DmGroupCmd::Add { channel, user } => {
            api.group_dm_add_recipient(&channel, &user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "channel": channel, "added": user }));
            } else {
                output::success(&format!("Added {} to group DM {}", user, channel));
            }
        }

        DmGroupCmd::Remove { channel, user } => {
            api.group_dm_remove_recipient(&channel, &user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "channel": channel, "removed": user }));
            } else {
                output::success(&format!("Removed {} from group DM {}", user, channel));
            }
        }

        DmGroupCmd::SetName { channel, name } => {
            let result = api.group_dm_set_name(&channel, &name).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Renamed group DM {} to '{}'", channel, name));
            }
        }
    }

    Ok(())
}
