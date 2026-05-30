//! `discord compose ...` — rich message composition (embeds, polls, TTS, mark-read).

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum ComposeCmd {
    /// Send a message with an embed.
    Embed {
        /// Channel ID (snowflake).
        channel: String,
        /// Optional message text above the embed.
        #[arg(long)]
        content: Option<String>,
        /// Embed title.
        #[arg(long)]
        title: Option<String>,
        /// Embed description.
        #[arg(long)]
        description: Option<String>,
        /// Embed colour as a decimal integer (e.g. 16711680 for red).
        #[arg(long)]
        color: Option<u32>,
        /// Embed URL (makes the title a hyperlink).
        #[arg(long)]
        url: Option<String>,
    },
    /// Send a text-to-speech message.
    Tts {
        /// Channel ID (snowflake).
        channel: String,
        /// Message content to speak.
        content: String,
    },
    /// Send a poll.
    Poll {
        /// Channel ID (snowflake).
        channel: String,
        /// Poll question text.
        question: String,
        /// Comma-separated list of answer choices.
        answers_comma_separated: String,
        /// Poll duration in hours (default: 24).
        #[arg(long, default_value_t = 24)]
        duration: u32,
        /// Allow voters to select multiple answers.
        #[arg(long)]
        multiselect: bool,
    },
    /// Mark a message as read (POST ack).
    MarkRead {
        /// Channel ID (snowflake).
        channel: String,
        /// Message ID (snowflake).
        message: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: ComposeCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        ComposeCmd::Embed {
            channel,
            content,
            title,
            description,
            color,
            url,
        } => {
            let result = api
                .compose_embed(
                    &channel,
                    content.as_deref(),
                    title.as_deref(),
                    description.as_deref(),
                    color,
                    url.as_deref(),
                )
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!("Sent embed message {} in channel {}", id, channel));
            }
        }

        ComposeCmd::Tts { channel, content } => {
            let result = api.compose_tts(&channel, &content).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!("Sent TTS message {} in channel {}", id, channel));
            }
        }

        ComposeCmd::Poll {
            channel,
            question,
            answers_comma_separated,
            duration,
            multiselect,
        } => {
            let answers: Vec<String> = answers_comma_separated
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if answers.is_empty() {
                output::warn("No answers provided for poll.");
                return Ok(());
            }
            let result = api
                .compose_poll(&channel, &question, &answers, duration, multiselect)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                output::success(&format!("Sent poll message {} in channel {}", id, channel));
            }
        }

        ComposeCmd::MarkRead { channel, message } => {
            api.compose_mark_read(&channel, &message).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "channel_id": channel,
                    "message_id": message,
                    "acked": true,
                }));
            } else {
                output::success(&format!(
                    "Marked message {} in channel {} as read",
                    message, channel
                ));
            }
        }
    }

    Ok(())
}
