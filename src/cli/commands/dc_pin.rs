//! `discord dc pin / unpin <CHANNEL> <MESSAGE>` — pin/unpin a message.

use anyhow::Result;

use crate::cli::api::{parse_snowflake, WriteApi};
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx, channel: &str, message: &str, unpin: bool) -> Result<()> {
    // Validate inputs before making any network requests
    parse_snowflake(channel)?;
    parse_snowflake(message)?;
    
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    if unpin {
        api.unpin_message(channel, message).await?;
    } else {
        api.pin_message(channel, message).await?;
    }

    let verb = if unpin { "Unpinned" } else { "Pinned" };
    if ctx.json {
        output::print_json(&serde_json::json!({
            "action": if unpin { "unpin" } else { "pin" },
            "channel_id": channel,
            "message_id": message,
        }));
    } else {
        output::success(&format!("{} message {} in channel {}", verb, message, channel));
    }
    Ok(())
}
