//! `discord dc account ...` — edit the authenticated user's own account/profile.
//!
//! Account fields (username, avatar, banner, accent_color) go to
//! `PATCH /users/@me`; profile fields (bio, pronouns) to
//! `PATCH /users/@me/profile`. Each request is skipped entirely when none of
//! its fields were supplied.

use anyhow::{anyhow, Result};
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum AccountCmd {
    /// Edit your own profile/account. Pass at least one field to change.
    Edit {
        /// New username.
        #[arg(long)]
        username: Option<String>,
        /// New global display name (the non-unique name shown in chat).
        #[arg(long = "global-name")]
        global_name: Option<String>,
        /// New "about me" bio.
        #[arg(long)]
        bio: Option<String>,
        /// New pronouns.
        #[arg(long)]
        pronouns: Option<String>,
        /// New accent color as a decimal integer (e.g. 16711680 for red).
        #[arg(long = "accent-color")]
        accent_color: Option<u32>,
        /// Path to a new avatar image (png/jpg/gif/webp).
        #[arg(long, value_name = "PATH")]
        avatar: Option<String>,
        /// Path to a new banner image (png/jpg/gif/webp).
        #[arg(long, value_name = "PATH")]
        banner: Option<String>,
    },
    /// List your third-party linked accounts (Steam, Spotify, …).
    Connections,
    /// Edit a connection's visibility / activity sharing.
    ConnectionEdit {
        /// Connection type (e.g. steam, spotify, github).
        conn_type: String,
        /// Connection account ID.
        conn_id: String,
        /// Show this connection on your profile.
        #[arg(long)]
        visible: Option<bool>,
        /// Share this connection's activity as rich presence.
        #[arg(long = "show-activity")]
        show_activity: Option<bool>,
    },
    /// Remove a third-party linked account.
    ConnectionRemove {
        /// Connection type (e.g. steam, spotify, github).
        conn_type: String,
        /// Connection account ID.
        conn_id: String,
    },
    /// Get your private note on a user.
    NoteGet {
        /// User ID (snowflake).
        user: String,
    },
    /// Set (or clear, with empty string) your private note on a user.
    NoteSet {
        /// User ID (snowflake).
        user: String,
        /// Note text (pass "" to clear).
        note: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: AccountCmd) -> Result<()> {
    match cmd {
        AccountCmd::Edit {
            username,
            global_name,
            bio,
            pronouns,
            accent_color,
            avatar,
            banner,
        } => {
            // Fail fast if nothing was supplied.
            if username.is_none()
                && global_name.is_none()
                && bio.is_none()
                && pronouns.is_none()
                && accent_color.is_none()
                && avatar.is_none()
                && banner.is_none()
            {
                return Err(anyhow!(
                    "nothing to edit: pass at least one of --username, --global-name, --bio, --pronouns, --accent-color, --avatar, --banner"
                ));
            }

            // Validate image files exist before any network call.
            for (flag, path) in [("--avatar", &avatar), ("--banner", &banner)] {
                if let Some(p) = path {
                    if !std::path::Path::new(p).is_file() {
                        return Err(anyhow!("{} file not found: {}", flag, p));
                    }
                }
            }

            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);

            let account = api
                .edit_account(
                    username.as_deref(),
                    global_name.as_deref(),
                    avatar.as_deref(),
                    banner.as_deref(),
                    accent_color,
                )
                .await?;
            let profile = api
                .edit_profile(bio.as_deref(), pronouns.as_deref())
                .await?;

            if ctx.json {
                output::print_json(&serde_json::json!({
                    "account": account,
                    "profile": profile,
                }));
            } else {
                let mut changed: Vec<&str> = Vec::new();
                if username.is_some() {
                    changed.push("username");
                }
                if global_name.is_some() {
                    changed.push("global_name");
                }
                if avatar.is_some() {
                    changed.push("avatar");
                }
                if banner.is_some() {
                    changed.push("banner");
                }
                if accent_color.is_some() {
                    changed.push("accent_color");
                }
                if bio.is_some() {
                    changed.push("bio");
                }
                if pronouns.is_some() {
                    changed.push("pronouns");
                }
                output::success(&format!("Updated profile: {}", changed.join(", ")));
            }
        }

        AccountCmd::Connections => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            let result = api.get_connections().await?;
            if ctx.json {
                output::print_json(&result);
            } else if let Some(arr) = result.as_array() {
                if arr.is_empty() {
                    output::dim("No linked accounts.");
                } else {
                    let rows: Vec<Vec<String>> = arr
                        .iter()
                        .map(|c| {
                            vec![
                                c["type"].as_str().unwrap_or("").to_string(),
                                c["name"].as_str().unwrap_or("").to_string(),
                                c["id"].as_str().unwrap_or("").to_string(),
                                c["verified"].as_bool().unwrap_or(false).to_string(),
                            ]
                        })
                        .collect();
                    output::print_table(&["type", "name", "id", "verified"], &rows);
                }
            } else {
                output::print_json(&result);
            }
        }

        AccountCmd::ConnectionEdit {
            conn_type,
            conn_id,
            visible,
            show_activity,
        } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            let result = api
                .edit_connection(&conn_type, &conn_id, visible, show_activity)
                .await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                output::success(&format!("Updated connection {}/{}", conn_type, conn_id));
            }
        }

        AccountCmd::ConnectionRemove { conn_type, conn_id } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.delete_connection(&conn_type, &conn_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "removed": format!("{}/{}", conn_type, conn_id) }));
            } else {
                output::success(&format!("Removed connection {}/{}", conn_type, conn_id));
            }
        }

        AccountCmd::NoteGet { user } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            let result = api.get_note(&user).await?;
            if ctx.json {
                output::print_json(&result);
            } else {
                let note = result["note"].as_str().unwrap_or("");
                if note.is_empty() {
                    output::dim(&format!("No note on user {}", user));
                } else {
                    println!("{}", note);
                }
            }
        }

        AccountCmd::NoteSet { user, note } => {
            let token = config::resolve_token(ctx.token_flag.clone())?;
            let api = WriteApi::new(&token);
            api.set_note(&user, &note).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "user_id": user, "note": note }));
            } else if note.is_empty() {
                output::success(&format!("Cleared note on user {}", user));
            } else {
                output::success(&format!("Set note on user {}", user));
            }
        }
    }

    Ok(())
}
