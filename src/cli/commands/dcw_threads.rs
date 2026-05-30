//! `discord dc thread ...` — thread management.

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum ThreadCmd {
    /// Create a standalone thread in a channel
    Create {
        /// Channel ID to create the thread in
        channel: String,
        /// Thread name
        name: String,
        /// Auto-archive duration in minutes (60, 1440, 4320, 10080)
        #[arg(long, default_value_t = 1440)]
        auto_archive: u32,
        /// Thread type: public | private | announcement
        #[arg(long = "type", default_value = "public")]
        thread_type: String,
    },

    /// Create a forum/media post (a thread with an initial message)
    ForumPost {
        /// Forum/media channel ID
        channel: String,
        /// Post title (thread name)
        name: String,
        /// Initial message content
        message: String,
        /// Auto-archive duration in minutes (60, 1440, 4320, 10080)
        #[arg(long, default_value_t = 1440)]
        auto_archive: u32,
        /// Apply a forum tag by ID (repeatable: --tag ID --tag ID)
        #[arg(long = "tag", value_name = "TAG_ID")]
        tags: Vec<String>,
    },

    /// Create a thread from an existing message
    CreateFrom {
        /// Channel ID containing the message
        channel: String,
        /// Message ID to create the thread from
        message: String,
        /// Thread name
        name: String,
        /// Auto-archive duration in minutes (60, 1440, 4320, 10080)
        #[arg(long, default_value_t = 1440)]
        auto_archive: u32,
    },

    /// Join a thread (@me)
    Join {
        /// Thread (channel) ID to join
        thread: String,
    },

    /// Leave a thread (@me)
    Leave {
        /// Thread (channel) ID to leave
        thread: String,
    },

    /// Add a user to a thread
    AddMember {
        /// Thread (channel) ID
        thread: String,
        /// User ID to add
        user: String,
    },

    /// Remove a user from a thread
    RemoveMember {
        /// Thread (channel) ID
        thread: String,
        /// User ID to remove
        user: String,
    },

    /// Archive a thread (optionally lock it)
    Archive {
        /// Thread (channel) ID to archive
        thread: String,
        /// Also lock the thread so only moderators can unarchive it
        #[arg(long)]
        locked: bool,
    },

    /// Unarchive (re-open) a thread
    Unarchive {
        /// Thread (channel) ID to unarchive
        thread: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: ThreadCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        ThreadCmd::Create {
            channel,
            name,
            auto_archive,
            thread_type,
        } => {
            if ![60, 1440, 4320, 10080].contains(&auto_archive) {
                return Err(anyhow::anyhow!("--auto-archive must be one of 60, 1440, 4320, 10080 (minutes); got {}", auto_archive));
            }
            let type_id = match thread_type.to_ascii_lowercase().as_str() {
                "public" => 11,
                "private" => 12,
                "announcement" | "news" => 10,
                other => {
                    return Err(anyhow::anyhow!(
                        "invalid --type {:?}; expected one of: public, private, announcement",
                        other
                    ))
                }
            };
            let v = api
                .thread_create_standalone(&channel, &name, auto_archive, type_id)
                .await?;
            if ctx.json {
                output::print_json(&v);
            } else {
                let id = v["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created {} thread '{}' (id {}) in channel {}",
                    thread_type, name, id, channel
                ));
            }
        }

        ThreadCmd::ForumPost {
            channel,
            name,
            message,
            auto_archive,
            tags,
        } => {
            if ![60, 1440, 4320, 10080].contains(&auto_archive) {
                return Err(anyhow::anyhow!("--auto-archive must be one of 60, 1440, 4320, 10080 (minutes); got {}", auto_archive));
            }
            let v = api
                .thread_create_forum_post(&channel, &name, &message, auto_archive, &tags)
                .await?;
            if ctx.json {
                output::print_json(&v);
            } else {
                let id = v["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created forum post '{}' (id {}) in channel {}",
                    name, id, channel
                ));
            }
        }

        ThreadCmd::CreateFrom {
            channel,
            message,
            name,
            auto_archive,
        } => {
            if ![60, 1440, 4320, 10080].contains(&auto_archive) {
                return Err(anyhow::anyhow!("--auto-archive must be one of 60, 1440, 4320, 10080 (minutes); got {}", auto_archive));
            }
            let v = api
                .thread_create_from_message(&channel, &message, &name, auto_archive)
                .await?;
            if ctx.json {
                output::print_json(&v);
            } else {
                let id = v["id"].as_str().unwrap_or("?");
                output::success(&format!(
                    "Created thread '{}' (id {}) from message {} in channel {}",
                    name, id, message, channel
                ));
            }
        }

        ThreadCmd::Join { thread } => {
            api.thread_join(&thread).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "joined": thread }));
            } else {
                output::success(&format!("Joined thread {}", thread));
            }
        }

        ThreadCmd::Leave { thread } => {
            api.thread_leave(&thread).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "left": thread }));
            } else {
                output::success(&format!("Left thread {}", thread));
            }
        }

        ThreadCmd::AddMember { thread, user } => {
            api.thread_add_member(&thread, &user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "thread": thread, "added_user": user }));
            } else {
                output::success(&format!("Added user {} to thread {}", user, thread));
            }
        }

        ThreadCmd::RemoveMember { thread, user } => {
            api.thread_remove_member(&thread, &user).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({ "thread": thread, "removed_user": user }));
            } else {
                output::success(&format!("Removed user {} from thread {}", user, thread));
            }
        }

        ThreadCmd::Archive { thread, locked } => {
            let v = api.thread_archive(&thread, locked).await?;
            if ctx.json {
                output::print_json(&v);
            } else {
                if locked {
                    output::success(&format!("Archived and locked thread {}", thread));
                } else {
                    output::success(&format!("Archived thread {}", thread));
                }
            }
        }

        ThreadCmd::Unarchive { thread } => {
            let v = api.thread_unarchive(&thread).await?;
            if ctx.json {
                output::print_json(&v);
            } else {
                output::success(&format!("Unarchived thread {}", thread));
            }
        }
    }

    Ok(())
}
