//! `discord dc react / unreact <CHANNEL> <MESSAGE> <EMOJI>` — add/remove a
//! reaction.

use anyhow::Result;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(
    ctx: &Ctx,
    channel: &str,
    message: &str,
    emoji: &str,
    remove: bool,
    burst: bool,
) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    if remove {
        api.remove_reaction(channel, message, emoji).await?;
    } else {
        api.add_reaction(channel, message, emoji, burst).await?;
    }

    let verb = if remove { "Removed" } else { "Added" };
    if ctx.json {
        output::print_json(&serde_json::json!({
            "action": if remove { "unreact" } else { "react" },
            "channel_id": channel,
            "message_id": message,
            "emoji": emoji,
        }));
    } else {
        output::success(&format!(
            "{} reaction {} on message {}",
            verb, emoji, message
        ));
    }
    Ok(())
}
