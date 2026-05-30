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
}

pub async fn dispatch(ctx: &Ctx, cmd: AccountCmd) -> Result<()> {
    match cmd {
        AccountCmd::Edit {
            username,
            bio,
            pronouns,
            accent_color,
            avatar,
            banner,
        } => {
            // Fail fast if nothing was supplied.
            if username.is_none()
                && bio.is_none()
                && pronouns.is_none()
                && accent_color.is_none()
                && avatar.is_none()
                && banner.is_none()
            {
                return Err(anyhow!(
                    "nothing to edit: pass at least one of --username, --bio, --pronouns, --accent-color, --avatar, --banner"
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
    }

    Ok(())
}
