//! `discord dc dm <USER_ID> <CONTENT>` — open a DM and send a message.

use anyhow::Result;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx, user_id: &str, content: &str) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    let msg = api.send_dm(user_id, content).await?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "id": msg.id,
            "recipient_id": user_id,
            "content": msg.content,
        }));
    } else {
        output::success(&format!("Sent DM {} to user {}", msg.id, user_id));
    }
    Ok(())
}
