//! `discord dc automod ...` — guild auto-moderation rule management.
//!
//! Covers Discord's Auto Moderation REST API under
//! `/guilds/{guild_id}/auto-moderation/rules`. Trigger and event type numbers
//! are documented inline in the `--help` text.

use std::io::{self, Write};

use anyhow::{anyhow, Result};
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum AutomodCmd {
    /// List all auto-moderation rules in a guild.
    List {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Get a single auto-moderation rule by ID.
    Get {
        /// Guild ID (snowflake).
        guild: String,
        /// Rule ID (snowflake).
        rule_id: String,
    },
    /// Create an auto-moderation rule.
    ///
    /// trigger-type: 1=KEYWORD, 3=SPAM, 4=KEYWORD_PRESET, 5=MENTION_SPAM.
    /// event-type: 1=MESSAGE_SEND. A default BLOCK_MESSAGE action (type 1) is
    /// always included.
    Create {
        /// Guild ID (snowflake).
        guild: String,
        /// Rule name.
        name: String,
        /// Event type: 1=MESSAGE_SEND.
        #[arg(long)]
        event_type: u8,
        /// Trigger type: 1=KEYWORD, 3=SPAM, 4=KEYWORD_PRESET, 5=MENTION_SPAM.
        #[arg(long)]
        trigger_type: u8,
        /// Comma-separated keyword filters (trigger type 1).
        #[arg(long)]
        keyword: Option<String>,
        /// Comma-separated preset numbers, e.g. 1,2,3 (trigger type 4:
        /// 1=PROFANITY, 2=SEXUAL_CONTENT, 3=SLURS).
        #[arg(long)]
        preset: Option<String>,
        /// Maximum mentions allowed per message (trigger type 5).
        #[arg(long)]
        mention_limit: Option<u32>,
        /// Enable the rule on creation.
        #[arg(long)]
        enabled: bool,
        /// Comma-separated role IDs exempt from this rule.
        #[arg(long)]
        exempt_roles: Option<String>,
        /// Comma-separated channel IDs exempt from this rule.
        #[arg(long)]
        exempt_channels: Option<String>,
    },
    /// Edit an existing auto-moderation rule. Only provided flags are sent.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// Rule ID (snowflake).
        rule_id: String,
        /// New rule name.
        #[arg(long)]
        name: Option<String>,
        /// New event type: 1=MESSAGE_SEND.
        #[arg(long)]
        event_type: Option<u8>,
        /// Comma-separated keyword filters (trigger type 1).
        #[arg(long)]
        keyword: Option<String>,
        /// Comma-separated preset numbers (trigger type 4).
        #[arg(long)]
        preset: Option<String>,
        /// Maximum mentions allowed per message (trigger type 5).
        #[arg(long)]
        mention_limit: Option<u32>,
        /// Set enabled flag (true/false).
        #[arg(long)]
        enabled: Option<bool>,
        /// Comma-separated role IDs exempt from this rule.
        #[arg(long)]
        exempt_roles: Option<String>,
        /// Comma-separated channel IDs exempt from this rule.
        #[arg(long)]
        exempt_channels: Option<String>,
    },
    /// Delete an auto-moderation rule (with confirmation prompt unless -y).
    Delete {
        /// Guild ID (snowflake).
        guild: String,
        /// Rule ID (snowflake).
        rule_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

/// Parse a comma-separated string into trimmed, non-empty parts.
fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Parse a comma-separated list of snowflake IDs, validating each.
fn parse_id_list(s: &str) -> Result<Vec<String>> {
    let ids = split_csv(s);
    for id in &ids {
        crate::cli::api::parse_snowflake(id)?;
    }
    Ok(ids)
}

/// Build `trigger_metadata` from the keyword/preset/mention-limit flags.
/// Returns `None` when no metadata flags were provided.
fn build_trigger_metadata(
    keyword: Option<&str>,
    preset: Option<&str>,
    mention_limit: Option<u32>,
) -> Result<Option<serde_json::Value>> {
    let mut meta = serde_json::Map::new();
    if let Some(k) = keyword {
        meta.insert("keyword_filter".into(), serde_json::json!(split_csv(k)));
    }
    if let Some(p) = preset {
        let presets: Vec<u32> = split_csv(p)
            .iter()
            .map(|n| {
                n.parse::<u32>()
                    .map_err(|_| anyhow!("invalid preset number: {:?}", n))
            })
            .collect::<Result<_>>()?;
        meta.insert("presets".into(), serde_json::json!(presets));
    }
    if let Some(limit) = mention_limit {
        meta.insert("mention_total_limit".into(), serde_json::json!(limit));
    }
    if meta.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(meta)))
    }
}

pub async fn dispatch(ctx: &Ctx, cmd: AutomodCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        AutomodCmd::List { guild } => {
            let result = api.automod_list(&guild).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let count = result.as_array().map(|a| a.len()).unwrap_or(0);
                output::success(&format!("{} auto-mod rule(s) in guild {}", count, guild));
            }
        }

        AutomodCmd::Get { guild, rule_id } => {
            let result = api.automod_get(&guild, &rule_id).await?;
            output::print_json(&result);
        }

        AutomodCmd::Create {
            guild,
            name,
            event_type,
            trigger_type,
            keyword,
            preset,
            mention_limit,
            enabled,
            exempt_roles,
            exempt_channels,
        } => {
            let mut body = serde_json::Map::new();
            body.insert("name".into(), serde_json::json!(name));
            body.insert("event_type".into(), serde_json::json!(event_type));
            body.insert("trigger_type".into(), serde_json::json!(trigger_type));
            // Minimal default action: BLOCK_MESSAGE (type 1).
            body.insert("actions".into(), serde_json::json!([{ "type": 1 }]));
            body.insert("enabled".into(), serde_json::json!(enabled));
            if let Some(meta) =
                build_trigger_metadata(keyword.as_deref(), preset.as_deref(), mention_limit)?
            {
                body.insert("trigger_metadata".into(), meta);
            }
            if let Some(roles) = exempt_roles {
                body.insert(
                    "exempt_roles".into(),
                    serde_json::json!(parse_id_list(&roles)?),
                );
            }
            if let Some(channels) = exempt_channels {
                body.insert(
                    "exempt_channels".into(),
                    serde_json::json!(parse_id_list(&channels)?),
                );
            }
            let result = api
                .automod_create(&guild, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let id = result["id"].as_str().unwrap_or("?");
                let rname = result["name"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created auto-mod rule '{}' ({}) in guild {}",
                    rname, id, guild
                ));
            }
        }

        AutomodCmd::Edit {
            guild,
            rule_id,
            name,
            event_type,
            keyword,
            preset,
            mention_limit,
            enabled,
            exempt_roles,
            exempt_channels,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(e) = event_type {
                body.insert("event_type".into(), serde_json::json!(e));
            }
            if let Some(e) = enabled {
                body.insert("enabled".into(), serde_json::json!(e));
            }
            if let Some(meta) =
                build_trigger_metadata(keyword.as_deref(), preset.as_deref(), mention_limit)?
            {
                body.insert("trigger_metadata".into(), meta);
            }
            if let Some(roles) = exempt_roles {
                body.insert(
                    "exempt_roles".into(),
                    serde_json::json!(parse_id_list(&roles)?),
                );
            }
            if let Some(channels) = exempt_channels {
                body.insert(
                    "exempt_channels".into(),
                    serde_json::json!(parse_id_list(&channels)?),
                );
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api
                .automod_edit(&guild, &rule_id, serde_json::Value::Object(body))
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let rname = result["name"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Updated auto-mod rule '{}' ({}) in guild {}",
                    rname, rule_id, guild
                ));
            }
        }

        AutomodCmd::Delete {
            guild,
            rule_id,
            yes,
        } => {
            if !yes {
                print!(
                    "Delete auto-mod rule {} in guild {}? Type 'yes' to confirm: ",
                    rule_id, guild
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.automod_delete(&guild, &rule_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "deleted": rule_id, "guild_id": guild }));
            } else {
                output::success(&format!(
                    "Deleted auto-mod rule {} in guild {}",
                    rule_id, guild
                ));
            }
        }
    }

    Ok(())
}
