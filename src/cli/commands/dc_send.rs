//! `discord dc send <CHANNEL> <CONTENT> [--reply <MSG>] [--file <PATH>...] [--sticker <ID>...]` — send a message.

use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::cli::api::{
    parse_snowflake, validate_content, validate_file_size, MessageOptions, WriteApi,
};
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(
    ctx: &Ctx,
    channel: &str,
    content: &str,
    reply: Option<&str>,
    files: &[PathBuf],
    stickers: &[String],
    opts: MessageOptions,
) -> Result<()> {
    // Validate inputs before making any network requests
    parse_snowflake(channel)?;
    if let Some(reply_id) = reply {
        parse_snowflake(reply_id)?;
    }
    validate_content(content)?;

    // Validate file sizes
    for file in files {
        validate_file_size(file)?;
    }

    // Validate sticker constraints before making any network requests.
    if stickers.len() > 3 {
        bail!(
            "at most 3 stickers may be attached per message (got {})",
            stickers.len()
        );
    }
    for id in stickers {
        parse_snowflake(id)?;
    }

    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    if !files.is_empty() {
        let result = api
            .send_message_with_files(channel, content, reply, files, stickers, &opts)
            .await?;
        if ctx.json {
            output::print_json(&result);
        } else {
            let id = result["id"].as_str().unwrap_or("?");
            let what = if reply.is_some() { "Replied" } else { "Sent" };
            output::success(&format!(
                "{} message {} ({} file(s)) in channel {}",
                what,
                id,
                files.len(),
                channel
            ));
        }
        return Ok(());
    }

    let msg = api
        .send_message(channel, content, reply, stickers, &opts)
        .await?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "id": msg.id,
            "channel_id": channel,
            "content": msg.content,
            "reply_to": reply,
        }));
    } else {
        let what = if reply.is_some() { "Replied" } else { "Sent" };
        output::success(&format!(
            "{} message {} in channel {}",
            what, msg.id, channel
        ));
    }
    Ok(())
}
