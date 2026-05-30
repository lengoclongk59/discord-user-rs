//! `discord dc typing <CHANNEL>` — fire the typing indicator once.

use anyhow::Result;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx, channel: &str) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    api.trigger_typing(channel).await?;

    if ctx.json {
        output::print_json(&serde_json::json!({ "channel_id": channel, "typing": true }));
    } else {
        output::success(&format!("Triggered typing in channel {}", channel));
    }
    Ok(())
}
