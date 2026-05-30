//! `discord dc invite ...` — invite management.

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum InviteCmd {
    /// Create an invite for a channel.
    Create {
        /// Channel ID (snowflake).
        channel: String,
        /// Duration in seconds before invite expires (0 = never). Default: 86400.
        #[arg(long, default_value_t = 86400)]
        max_age: u32,
        /// Maximum number of uses (0 = unlimited). Default: 0.
        #[arg(long, default_value_t = 0)]
        max_uses: u32,
        /// Grant temporary membership (kicked when they disconnect unless given a role).
        #[arg(long)]
        temporary: bool,
        /// Guarantee a unique invite code.
        #[arg(long)]
        unique: bool,
    },
    /// Delete an invite by code.
    Delete {
        /// Invite code (e.g. `abc123`).
        code: String,
    },
    /// List all invites for a channel.
    ListChannel {
        /// Channel ID (snowflake).
        channel: String,
    },
    /// List all invites for a guild.
    ListGuild {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Get a single invite by code (with member counts).
    Get {
        /// Invite code (e.g. `abc123`).
        code: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: InviteCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        InviteCmd::Create {
            channel,
            max_age,
            max_uses,
            temporary,
            unique,
        } => {
            let result = api
                .invite_create(&channel, max_age, max_uses, temporary, unique)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let code = result["code"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created invite: https://discord.gg/{}",
                    code
                ));
            }
        }

        InviteCmd::Delete { code } => {
            let result = api.invite_delete(&code).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let deleted_code = result["code"].as_str().unwrap_or(&code);
                output::success(&format!("Deleted invite {}", deleted_code));
            }
        }

        InviteCmd::ListChannel { channel } => {
            let result = api.invite_list_channel(&channel).await?;
            output::print_json(&result);
        }

        InviteCmd::ListGuild { guild } => {
            let result = api.invite_list_guild(&guild).await?;
            output::print_json(&result);
        }

        InviteCmd::Get { code } => {
            let result = api.invite_get(&code).await?;
            output::print_json(&result);
        }
    }

    Ok(())
}
