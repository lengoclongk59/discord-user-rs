//! `discord dc message ...` — advanced message operations.

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum MessageCmd {
    /// Bulk-delete up to 100 messages by ID (bot/managed tokens only)
    BulkDelete {
        /// Channel ID containing the messages
        channel: String,
        /// Comma-separated message IDs to delete
        message_ids: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Publish a message to announcement-channel followers (crosspost)
    Crosspost {
        /// Channel ID (must be an announcement channel)
        channel: String,
        /// Message ID to crosspost
        message: String,
    },

    /// Remove all reactions from a message
    ClearReactions {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Remove all reactions for a specific emoji from a message
    ClearEmoji {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
        /// Emoji (unicode or name:id for custom)
        emoji: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Remove a specific user's reaction from a message
    RemoveUserReaction {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
        /// User ID whose reaction to remove
        user: String,
        /// Emoji (unicode or name:id for custom)
        emoji: String,
    },

    /// List users who reacted with a given emoji
    Reactors {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
        /// Emoji (unicode or name:id for custom)
        emoji: String,
        /// Maximum number of reactors to return (1-100, default 25)
        #[arg(short = 'n', long, default_value_t = 25)]
        limit: u32,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: MessageCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        MessageCmd::BulkDelete {
            channel,
            message_ids,
            yes,
        } => {
            let ids: Vec<String> = message_ids
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            // Discord's bulk-delete endpoint requires 2..=100 messages and
            // rejects any message older than 2 weeks. Enforce the count
            // locally so we fail fast with a clear message instead of a 400.
            if ids.len() < 2 {
                return Err(anyhow::anyhow!(
                    "bulk-delete needs at least 2 message IDs (got {}). Use `dc delete` for a single message.",
                    ids.len()
                ));
            }
            if ids.len() > 100 {
                return Err(anyhow::anyhow!(
                    "bulk-delete accepts at most 100 message IDs (got {}).",
                    ids.len()
                ));
            }

            if !yes
                && !output::confirm(&format!(
                    "Bulk-delete {} message(s) from channel {}?",
                    ids.len(),
                    channel
                ))?
            {
                return Ok(());
            }

            api.msg_bulk_delete(&channel, &ids).await?;

            if ctx.json {
                output::print_json(&serde_json::json!({
                    "action": "bulk_delete",
                    "channel_id": channel,
                    "deleted": ids,
                }));
            } else {
                output::success(&format!(
                    "Bulk-deleted {} message(s) from channel {}.",
                    ids.len(),
                    channel
                ));
            }
        }

        MessageCmd::Crosspost { channel, message } => {
            let v = api.msg_crosspost(&channel, &message).await?;
            if ctx.json {
                output::print_json(&v);
            } else {
                output::success(&format!(
                    "Crossposted message {} in channel {}.",
                    message, channel
                ));
            }
        }

        MessageCmd::ClearReactions {
            channel,
            message,
            yes,
        } => {
            if !yes
                && !output::confirm(&format!(
                    "Clear ALL reactions from message {} in channel {}?",
                    message, channel
                ))?
            {
                return Ok(());
            }
            api.msg_remove_all_reactions(&channel, &message).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "action": "clear_reactions",
                    "channel_id": channel,
                    "message_id": message,
                }));
            } else {
                output::success(&format!(
                    "Cleared all reactions from message {} in channel {}.",
                    message, channel
                ));
            }
        }

        MessageCmd::ClearEmoji {
            channel,
            message,
            emoji,
            yes,
        } => {
            if !yes
                && !output::confirm(&format!(
                    "Clear all {} reactions from message {} in channel {}?",
                    emoji, message, channel
                ))?
            {
                return Ok(());
            }
            api.msg_clear_emoji(&channel, &message, &emoji).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "action": "clear_emoji",
                    "channel_id": channel,
                    "message_id": message,
                    "emoji": emoji,
                }));
            } else {
                output::success(&format!(
                    "Cleared {} reactions from message {} in channel {}.",
                    emoji, message, channel
                ));
            }
        }

        MessageCmd::RemoveUserReaction {
            channel,
            message,
            user,
            emoji,
        } => {
            api.msg_remove_user_reaction(&channel, &message, &user, &emoji)
                .await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "action": "remove_user_reaction",
                    "channel_id": channel,
                    "message_id": message,
                    "user_id": user,
                    "emoji": emoji,
                }));
            } else {
                output::success(&format!(
                    "Removed {} reaction by user {} from message {} in channel {}.",
                    emoji, user, message, channel
                ));
            }
        }

        MessageCmd::Reactors {
            channel,
            message,
            emoji,
            limit,
        } => {
            // Discord caps the reactor list at 100 per request.
            let limit = limit.clamp(1, 100);
            let v = api.msg_get_reactors(&channel, &message, &emoji, limit).await?;
            output::print_json(&v);
        }
    }

    Ok(())
}
