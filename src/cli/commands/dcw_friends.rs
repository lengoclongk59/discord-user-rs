//! `discord dc friend ...` — relationship management (friends / block).

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum FriendCmd {
    /// Add a friend or accept a pending request by user ID
    Add {
        /// User ID (snowflake) to add or accept
        user: String,
    },

    /// Send a friend request by global username (pomelo)
    Request {
        /// Global username (e.g. `alice` — no discriminator for pomelo accounts)
        username: String,
    },

    /// Accept a pending incoming friend request by user ID
    Accept {
        /// User ID (snowflake) of the pending request to accept
        user: String,
    },

    /// Remove a friend or cancel an outgoing friend request by user ID
    Remove {
        /// User ID (snowflake) to unfriend or cancel request for
        user: String,
    },

    /// Block a user by user ID
    Block {
        /// User ID (snowflake) to block
        user: String,
    },

    /// Unblock a user by user ID
    Unblock {
        /// User ID (snowflake) to unblock
        user: String,
    },

    /// Set (or clear) a friend nickname
    Nickname {
        /// User ID (snowflake) of the friend
        user: String,
        /// Nickname text (pass "" to clear)
        nickname: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: FriendCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        FriendCmd::Add { user } => {
            api.friend_add(&user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "added": user }));
            } else {
                output::success(&format!("Friend request sent / accepted for user {}", user));
            }
        }

        FriendCmd::Request { username } => {
            api.friend_request(&username).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "requested": username }));
            } else {
                output::success(&format!("Friend request sent to {}", username));
            }
        }

        FriendCmd::Accept { user } => {
            api.friend_add(&user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "accepted": user }));
            } else {
                output::success(&format!("Accepted friend request from user {}", user));
            }
        }

        FriendCmd::Remove { user } => {
            api.friend_remove(&user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "removed": user }));
            } else {
                output::success(&format!("Removed friend / cancelled request for user {}", user));
            }
        }

        FriendCmd::Block { user } => {
            api.friend_block(&user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "blocked": user }));
            } else {
                output::success(&format!("Blocked user {}", user));
            }
        }

        FriendCmd::Unblock { user } => {
            api.friend_unblock(&user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "unblocked": user }));
            } else {
                output::success(&format!("Unblocked user {}", user));
            }
        }

        FriendCmd::Nickname { user, nickname } => {
            api.friend_set_nickname(&user, &nickname).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "user_id": user, "nickname": nickname }));
            } else if nickname.is_empty() {
                output::success(&format!("Cleared nickname for user {}", user));
            } else {
                output::success(&format!("Set nickname for user {} to {:?}", user, nickname));
            }
        }
    }

    Ok(())
}
