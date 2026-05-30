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
        /// Footer text.
        #[arg(long)]
        footer: Option<String>,
        /// Footer icon URL.
        #[arg(long = "footer-icon")]
        footer_icon: Option<String>,
        /// Author name.
        #[arg(long)]
        author: Option<String>,
        /// Author URL.
        #[arg(long = "author-url")]
        author_url: Option<String>,
        /// Author icon URL.
        #[arg(long = "author-icon")]
        author_icon: Option<String>,
        /// Large image URL.
        #[arg(long)]
        image: Option<String>,
        /// Thumbnail image URL.
        #[arg(long)]
        thumbnail: Option<String>,
        /// ISO-8601 timestamp shown in the footer (e.g. 2025-01-01T00:00:00Z).
        #[arg(long)]
        timestamp: Option<String>,
        /// Add a field as `name|value` or `name|value|inline` (repeatable).
        #[arg(long = "field", value_name = "NAME|VALUE[|inline]")]
        fields: Vec<String>,
        /// Attach a file (repeatable: --file a.png --file b.pdf)
        #[arg(long = "file", value_name = "PATH")]
        files: Vec<std::path::PathBuf>,
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
            footer,
            footer_icon,
            author,
            author_url,
            author_icon,
            image,
            thumbnail,
            timestamp,
            fields,
            files,
        } => {
            // Parse `name|value` or `name|value|inline` field specs.
            let mut parsed_fields = Vec::with_capacity(fields.len());
            for f in &fields {
                let parts: Vec<&str> = f.splitn(3, '|').collect();
                if parts.len() < 2 {
                    return Err(anyhow::anyhow!(
                        "invalid --field {:?}: expected 'name|value' or 'name|value|inline'",
                        f
                    ));
                }
                let inline = parts
                    .get(2)
                    .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "inline" | "true" | "1" | "yes"))
                    .unwrap_or(false);
                parsed_fields.push((parts[0].trim().to_string(), parts[1].trim().to_string(), inline));
            }
            let spec = crate::cli::api_compose::EmbedSpec {
                title,
                description,
                color,
                url,
                footer,
                footer_icon,
                author,
                author_url,
                author_icon,
                image,
                thumbnail,
                timestamp,
                fields: parsed_fields,
            };
            let result = api
                .compose_embed(&channel, content.as_deref(), &spec, &files)
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
