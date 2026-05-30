//! `discord dc edit <CHANNEL> <MESSAGE> <CONTENT>` — edit a message.

use anyhow::Result;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx, channel: &str, message: &str, content: &str) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    let msg = api.edit_message(channel, message, content).await?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "id": msg.id,
            "channel_id": channel,
            "content": msg.content,
        }));
    } else {
        output::success(&format!("Edited message {} in channel {}", message, channel));
    }
    Ok(())
}
