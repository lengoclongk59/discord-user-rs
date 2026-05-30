//! `discord dc emoji ...` — custom guild emoji management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum EmojiCmd {
    /// Upload a new custom emoji to a guild.
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Emoji name (alphanumeric + underscores, 2–32 chars).
        name: String,
        /// Path to the image file (PNG, JPEG, or GIF).
        image_path: String,
    },
    /// Rename an existing custom emoji.
    Rename {
        /// Guild ID (snowflake).
        guild: String,
        /// Emoji ID (snowflake).
        emoji_id: String,
        /// New emoji name.
        name: String,
    },
    /// Delete a custom emoji from a guild.
    Delete {
        /// Guild ID (snowflake).
        guild: String,
        /// Emoji ID (snowflake).
        emoji_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: EmojiCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        EmojiCmd::Create {
            guild,
            name,
            image_path,
        } => {
            let result = api.emoji_create(&guild, &name, &image_path).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                output::success(&format!(
                    "Created emoji '{}' (id={}) in guild {}",
                    name, id, guild
                ));
            }
        }

        EmojiCmd::Rename {
            guild,
            emoji_id,
            name,
        } => {
            let result = api.emoji_rename(&guild, &emoji_id, &name).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Renamed emoji {} to '{}' in guild {}",
                    emoji_id, name, guild
                ));
            }
        }

        EmojiCmd::Delete { guild, emoji_id, yes } => {
            if !yes {
                print!(
                    "Delete emoji {} from guild {}? Type 'yes' to confirm: ",
                    emoji_id, guild
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.emoji_delete(&guild, &emoji_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "deleted": emoji_id,
                    "guild_id": guild,
                }));
            } else {
                output::success(&format!(
                    "Deleted emoji {} from guild {}",
                    emoji_id, guild
                ));
            }
        }
    }

    Ok(())
}
