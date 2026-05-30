//! `discord dc sticker ...` — custom guild sticker management.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;
use serde_json::json;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum StickerCmd {
    /// Upload a new custom sticker to a guild.
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Sticker name (2–30 characters).
        name: String,
        /// Autocomplete/suggestion tags (comma-separated, max 200 chars).
        tags: String,
        /// Path to the sticker file (.png, .apng, .gif, or .json for Lottie).
        file_path: String,
        /// Sticker description (0–100 characters).
        #[arg(long, default_value = "")]
        description: String,
    },
    /// Edit a guild sticker's name, description, or tags.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// Sticker ID (snowflake).
        sticker_id: String,
        /// New sticker name.
        #[arg(long)]
        name: Option<String>,
        /// New sticker description.
        #[arg(long)]
        description: Option<String>,
        /// New autocomplete tags.
        #[arg(long)]
        tags: Option<String>,
    },
    /// Delete a guild sticker.
    Delete {
        /// Guild ID (snowflake).
        guild: String,
        /// Sticker ID (snowflake).
        sticker_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: StickerCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        StickerCmd::Create {
            guild,
            name,
            tags,
            file_path,
            description,
        } => {
            let result = api
                .sticker_create(&guild, &name, &description, &tags, &file_path)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created sticker '{}' (id {}) in guild {}",
                    name, id, guild
                ));
            }
        }

        StickerCmd::Edit {
            guild,
            sticker_id,
            name,
            description,
            tags,
        } => {
            if name.is_none() && description.is_none() && tags.is_none() {
                output::warn("No fields provided — nothing to update.");
                return Ok(());
            }
            let mut body = json!({});
            if let Some(n) = name {
                body["name"] = json!(n);
            }
            if let Some(d) = description {
                body["description"] = json!(d);
            }
            if let Some(t) = tags {
                body["tags"] = json!(t);
            }
            let result = api.sticker_edit(&guild, &sticker_id, body).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!(
                    "Updated sticker {} in guild {}",
                    sticker_id, guild
                ));
            }
        }

        StickerCmd::Delete {
            guild,
            sticker_id,
            yes,
        } => {
            if !yes {
                print!(
                    "Delete sticker {} from guild {}? Type 'yes' to confirm: ",
                    sticker_id, guild
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.sticker_delete(&guild, &sticker_id).await?;
            if ctx.json {
                output::print_json(&json!({
                    "deleted": sticker_id,
                    "guild_id": guild,
                }));
            } else {
                output::success(&format!(
                    "Deleted sticker {} from guild {}",
                    sticker_id, guild
                ));
            }
        }
    }

    Ok(())
}
