//! `discord dc webhook ...` — webhook management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum WebhookCmd {
    /// Create a webhook in a channel.
    Create {
        /// Channel ID (snowflake).
        channel: String,
        /// Webhook name.
        name: String,
    },
    /// Edit a webhook. Only provided flags are sent.
    Edit {
        /// Webhook ID (snowflake).
        webhook_id: String,
        /// New webhook name.
        #[arg(long)]
        name: Option<String>,
        /// Move webhook to a different channel ID.
        #[arg(long)]
        channel: Option<String>,
    },
    /// Delete a webhook (with confirmation prompt unless -y).
    Delete {
        /// Webhook ID (snowflake).
        webhook_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Execute a webhook — post a message via its token.
    Execute {
        /// Webhook ID (snowflake).
        webhook_id: String,
        /// Webhook token string.
        token: String,
        /// Message content to post.
        content: String,
    },
    /// List webhooks in a channel.
    ListChannel {
        /// Channel ID (snowflake).
        channel: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: WebhookCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        WebhookCmd::Create { channel, name } => {
            let result = api.webhook_create(&channel, &name).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                let wname = result["name"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created webhook '{}' ({}) in channel {}",
                    wname, id, channel
                ));
            }
        }

        WebhookCmd::Edit {
            webhook_id,
            name,
            channel,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(c) = channel {
                body.insert("channel_id".into(), serde_json::json!(c));
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api
                .webhook_edit(&webhook_id, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let wname = result["name"].as_str().unwrap_or("?");
                output::success(&format!("Updated webhook '{}' ({})", wname, webhook_id));
            }
        }

        WebhookCmd::Delete { webhook_id, yes } => {
            if !yes {
                print!("Delete webhook {}? Type 'yes' to confirm: ", webhook_id);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.webhook_delete(&webhook_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "deleted": webhook_id }));
            } else {
                output::success(&format!("Deleted webhook {}", webhook_id));
            }
        }

        WebhookCmd::Execute {
            webhook_id,
            token,
            content,
        } => {
            api.webhook_execute(&webhook_id, &token, &content).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "webhook_id": webhook_id,
                    "executed": true,
                }));
            } else {
                output::success(&format!("Executed webhook {}", webhook_id));
            }
        }

        WebhookCmd::ListChannel { channel } => {
            let result = api.webhook_list_channel(&channel).await?;
            output::print_json(&result);
        }
    }

    Ok(())
}
