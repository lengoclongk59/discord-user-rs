//! `discord dc presence ...` — set online status & custom activity over the
//! Gateway.
//!
//! UpdatePresence (op 3) is only honored by Discord while the gateway
//! connection that sent it stays open: as soon as the socket closes, the
//! presence reverts. So, like `dc tail`, this command connects, IDENTIFYs,
//! sends the presence update, and then holds the connection alive (the gateway
//! heartbeats automatically) until the user presses Ctrl+C. There is no
//! fire-and-forget mode because a one-shot send would be undone the instant we
//! disconnected.

use anyhow::{anyhow, Result};
use clap::Subcommand;

use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;
use crate::operations::{ActivityData, StatusOps};
use crate::types::UserStatus;

#[derive(Subcommand, Debug)]
pub enum PresenceCmd {
    /// Set your online status (and optionally a custom activity), holding the
    /// Gateway connection open until Ctrl+C.
    Set {
        /// Online status: online | idle | dnd | invisible
        status: String,
        /// Optional activity name (e.g. "Chess").
        #[arg(long)]
        activity: Option<String>,
        /// Activity type: playing | streaming | listening | watching | custom
        #[arg(long = "type", default_value = "playing")]
        activity_type: String,
        /// Activity URL (only meaningful for streaming type).
        #[arg(long)]
        url: Option<String>,
    },
}

/// Parse a status string into [`UserStatus`], failing fast on invalid input.
fn parse_status(s: &str) -> Result<UserStatus> {
    match s.to_ascii_lowercase().as_str() {
        "online" => Ok(UserStatus::Online),
        "idle" => Ok(UserStatus::Idle),
        "dnd" => Ok(UserStatus::DoNotDisturb),
        "invisible" => Ok(UserStatus::Invisible),
        other => Err(anyhow!(
            "invalid status {:?}; expected one of: online, idle, dnd, invisible",
            other
        )),
    }
}

pub async fn dispatch(ctx: &Ctx, cmd: PresenceCmd) -> Result<()> {
    match cmd {
        PresenceCmd::Set {
            status,
            activity,
            activity_type,
            url,
        } => {
            let status = parse_status(&status)?;
            let activity_type = activity_type.to_ascii_lowercase();

            // Validate the activity type before connecting so we fail fast.
            match activity_type.as_str() {
                "playing" | "streaming" | "listening" | "watching" | "custom" => {}
                other => {
                    return Err(anyhow!(
                        "invalid --type {:?}; expected one of: playing, streaming, listening, watching, custom",
                        other
                    ))
                }
            }

            let token = config::resolve_token(ctx.token_flag.clone())?;

            output::dim("Connecting to Gateway... press Ctrl+C to stop");

            // `token` is `Zeroizing<String>`; `as_str()` exposes a `&str` that
            // `DiscordUser::new`'s `impl Into<String>` bound accepts.
            let mut user = crate::DiscordUser::new(token.as_str());
            user.init()
                .await
                .map_err(|e| anyhow!("Gateway connection failed: {}", e))?;

            // Send the presence update once connected. A "custom" activity uses
            // activity type 4 (state text), which the library exposes via
            // `set_custom_status`; the rest map onto `ActivityData`.
            match activity.as_deref() {
                None => {
                    user.set_status(status)
                        .await
                        .map_err(|e| anyhow!("failed to set status: {}", e))?;
                    output::success(&format!("Status set to {}", status.as_str()));
                }
                Some(name) => match activity_type.as_str() {
                    "custom" => {
                        user.set_custom_status(status, Some(name), None)
                            .await
                            .map_err(|e| anyhow!("failed to set custom status: {}", e))?;
                        output::success(&format!(
                            "Status set to {} with custom status {:?}",
                            status.as_str(),
                            name
                        ));
                    }
                    other => {
                        let activity = match other {
                            "playing" => ActivityData::playing(name),
                            "listening" => ActivityData::listening(name),
                            "watching" => ActivityData::watching(name),
                            "streaming" => {
                                let url = url.as_deref().ok_or_else(|| {
                                    anyhow!("--url is required for --type streaming")
                                })?;
                                ActivityData::streaming(name, url)
                            }
                            // Unreachable: validated above.
                            _ => unreachable!(),
                        };
                        user.set_activity(activity, status)
                            .await
                            .map_err(|e| anyhow!("failed to set activity: {}", e))?;
                        output::success(&format!(
                            "Status set to {} ({} {:?})",
                            status.as_str(),
                            other,
                            name
                        ));
                    }
                },
            }

            output::dim("Presence is live. Holding the Gateway connection (Ctrl+C to stop)...");

            tokio::signal::ctrl_c()
                .await
                .map_err(|e| anyhow!("failed to install Ctrl+C handler: {}", e))?;
            output::dim("\nDisconnecting...");
            user.disconnect().await;
            output::dim("Stopped.");
        }
    }

    Ok(())
}
