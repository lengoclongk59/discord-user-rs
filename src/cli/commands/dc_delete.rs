//! `discord dc delete <CHANNEL> <MESSAGE> [-y]` — delete a message.

use std::io::{self, Write};

use anyhow::Result;

use crate::cli::api::{parse_snowflake, WriteApi};
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx, channel: &str, message: &str, yes: bool) -> Result<()> {
    // Validate inputs before making any network requests
    parse_snowflake(channel)?;
    parse_snowflake(message)?;
    
    if !yes {
        print!(
            "Delete message {} in channel {}? Type 'yes' to confirm: ",
            message, channel
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() != "yes" {
            output::dim("Aborted.");
            return Ok(());
        }
    }

    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    api.delete_message(channel, message).await?;

    if ctx.json {
        output::print_json(&serde_json::json!({
            "deleted": message,
            "channel_id": channel,
        }));
    } else {
        output::success(&format!("Deleted message {} in channel {}", message, channel));
    }
    Ok(())
}
