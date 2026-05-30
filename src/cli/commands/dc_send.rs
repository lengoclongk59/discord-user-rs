//! `discord dc send <CHANNEL> <CONTENT> [--reply <MSG>]` — send a message.

use anyhow::Result;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx, channel: &str, content: &str, reply: Option<&str>) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    let msg = api.send_message(channel, content, reply).await?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "id": msg.id,
            "channel_id": channel,
            "content": msg.content,
            "reply_to": reply,
        }));
    } else {
        let what = if reply.is_some() { "Replied" } else { "Sent" };
        output::success(&format!("{} message {} in channel {}", what, msg.id, channel));
    }
    Ok(())
}
