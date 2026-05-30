//! `discord dc guild ...` — guild settings & own membership.

use std::io::{self, Write};

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum GuildCmd {
    /// Edit guild settings. Only provided flags are sent.
    Edit {
        /// Guild ID (snowflake).
        guild: String,
        /// New guild name.
        #[arg(long)]
        name: Option<String>,
        /// New guild description.
        #[arg(long)]
        description: Option<String>,
        /// AFK channel ID (snowflake).
        #[arg(long)]
        afk_channel: Option<String>,
        /// AFK timeout in seconds.
        #[arg(long)]
        afk_timeout: Option<u32>,
    },
    /// Set your own nickname in a guild.
    SetNick {
        /// Guild ID (snowflake).
        guild: String,
        /// New nickname.
        nick: String,
    },
    /// Leave a guild (confirmation prompt unless -y).
    Leave {
        /// Guild ID (snowflake).
        guild: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Get a guild's vanity URL (code + uses).
    Vanity {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Get a guild's widget settings.
    #[command(name = "widget-get")]
    WidgetGet {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Edit a guild's widget settings. Only provided flags are sent.
    #[command(name = "widget-edit")]
    WidgetEdit {
        /// Guild ID (snowflake).
        guild: String,
        /// Enable or disable the widget.
        #[arg(long)]
        enabled: Option<bool>,
        /// Widget channel ID (snowflake).
        #[arg(long)]
        channel: Option<String>,
    },
    /// Get a guild's welcome screen.
    #[command(name = "welcome-get")]
    WelcomeGet {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Edit a guild's welcome screen. Only provided flags are sent.
    #[command(name = "welcome-edit")]
    WelcomeEdit {
        /// Guild ID (snowflake).
        guild: String,
        /// Enable or disable the welcome screen.
        #[arg(long)]
        enabled: Option<bool>,
        /// Welcome screen description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Get a guild's onboarding configuration.
    #[command(name = "onboarding-get")]
    OnboardingGet {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// List a guild's integrations.
    Integrations {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Delete a guild integration (confirmation prompt unless -y).
    #[command(name = "integration-delete")]
    IntegrationDelete {
        /// Guild ID (snowflake).
        guild: String,
        /// Integration ID (snowflake).
        integration_id: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// List a guild's templates.
    Templates {
        /// Guild ID (snowflake).
        guild: String,
    },
    /// Create a guild template.
    #[command(name = "template-create")]
    TemplateCreate {
        /// Guild ID (snowflake).
        guild: String,
        /// Template name.
        name: String,
        /// Template description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Sync a guild template to the guild's current state.
    #[command(name = "template-sync")]
    TemplateSync {
        /// Guild ID (snowflake).
        guild: String,
        /// Template code.
        code: String,
    },
    /// Delete a guild template (confirmation prompt unless -y).
    #[command(name = "template-delete")]
    TemplateDelete {
        /// Guild ID (snowflake).
        guild: String,
        /// Template code.
        code: String,
        /// Skip confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: GuildCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        GuildCmd::Edit {
            guild,
            name,
            description,
            afk_channel,
            afk_timeout,
        } => {
            let mut body = serde_json::Map::new();
            if let Some(n) = name {
                body.insert("name".into(), serde_json::json!(n));
            }
            if let Some(d) = description {
                body.insert("description".into(), serde_json::json!(d));
            }
            if let Some(c) = afk_channel {
                body.insert("afk_channel_id".into(), serde_json::json!(c));
            }
            if let Some(t) = afk_timeout {
                body.insert("afk_timeout".into(), serde_json::json!(t));
            }
            if body.is_empty() {
                output::warn("No fields provided — nothing to edit.");
                return Ok(());
            }
            let result = api.guild_edit(&guild, serde_json::Value::Object(body)).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let gname = result["name"].as_str().unwrap_or("?");
                output::success(&format!("Updated guild '{}' ({})", gname, guild));
            }
        }

        GuildCmd::SetNick { guild, nick } => {
            let result = api.guild_set_nick(&guild, &nick).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Nickname set to '{}' in guild {}", nick, guild));
            }
        }

        GuildCmd::Leave { guild, yes } => {
            if !yes {
                print!("Leave guild {}? Type 'yes' to confirm: ", guild);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() != "yes" {
                    output::dim("Aborted.");
                    return Ok(());
                }
            }
            api.guild_leave(&guild).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "left": guild }));
            } else {
                output::success(&format!("Left guild {}", guild));
            }
        }

        GuildCmd::Vanity { guild } => {
            let result = api.guild_vanity(&guild).await?;
            output::print_json(&result);
        }

        GuildCmd::WidgetGet { guild } => {
            let result = api.guild_widget_get(&guild).await?;
            output::print_json(&result);
        }

        GuildCmd::WidgetEdit {
            guild,
            enabled,
            channel,
        } => {
            let result = api
                .guild_widget_edit(&guild, enabled, channel.as_deref())
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Updated widget settings for guild {}", guild));
            }
        }

        GuildCmd::WelcomeGet { guild } => {
            let result = api.guild_welcome_get(&guild).await?;
            output::print_json(&result);
        }

        GuildCmd::WelcomeEdit {
            guild,
            enabled,
            description,
        } => {
            let result = api
                .guild_welcome_edit(&guild, enabled, description.as_deref())
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Updated welcome screen for guild {}", guild));
            }
        }

        GuildCmd::OnboardingGet { guild } => {
            let result = api.guild_onboarding_get(&guild).await?;
            output::print_json(&result);
        }

        GuildCmd::Integrations { guild } => {
            let result = api.guild_integrations(&guild).await?;
            output::print_json(&result);
        }

        GuildCmd::IntegrationDelete {
            guild,
            integration_id,
            yes,
        } => {
            if !yes
                && !output::confirm(&format!(
                    "Delete integration {} from guild {}?",
                    integration_id, guild
                ))?
            {
                return Ok(());
            }
            api.guild_integration_delete(&guild, &integration_id).await?;
            if ctx.json {
                output::print_json(
                    &serde_json::json!({ "deleted": integration_id, "guild": guild }),
                );
            } else {
                output::success(&format!(
                    "Deleted integration {} from guild {}",
                    integration_id, guild
                ));
            }
        }

        GuildCmd::Templates { guild } => {
            let result = api.guild_templates(&guild).await?;
            output::print_json(&result);
        }

        GuildCmd::TemplateCreate {
            guild,
            name,
            description,
        } => {
            let result = api
                .guild_template_create(&guild, &name, description.as_deref())
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let code = result["code"].as_str().unwrap_or("?");
                output::success(&format!("Created template '{}' ({})", name, code));
            }
        }

        GuildCmd::TemplateSync { guild, code } => {
            let result = api.guild_template_sync(&guild, &code).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Synced template {} in guild {}", code, guild));
            }
        }

        GuildCmd::TemplateDelete { guild, code, yes } => {
            if !yes
                && !output::confirm(&format!(
                    "Delete template {} from guild {}?",
                    code, guild
                ))?
            {
                return Ok(());
            }
            let result = api.guild_template_delete(&guild, &code).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Deleted template {} from guild {}", code, guild));
            }
        }
    }

    Ok(())
}
