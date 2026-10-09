//! `discord dc tail <CHANNEL>` — stream messages in real time via the
//! Gateway, persisting MESSAGE_CREATE/UPDATE/DELETE into the local archive.
//!
//! Snapshot policy: the initial REST fetch starts from `last_msg_id` so a
//! restart no longer re-prints (and re-stores) the same `--limit` window.
//! When the DB is empty we fall back to the most recent `--limit`.
//!
//! Persistence is funnelled through a single mpsc channel into a dedicated
//! writer task that owns the SQLite `Db` handle. This replaces an earlier
//! `Arc<tokio::sync::Mutex<Db>>` that held the lock across awaits and
//! serialized live events one row at a time behind every snapshot insert.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::DateTime;
use colored::Colorize;
use tokio::sync::mpsc;

use crate::cli::api::Api;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::db::Db;
use crate::cli::output;
use crate::cli::types::{ChannelContext, StoredMessage};

enum DbOp {
    Insert(Vec<StoredMessage>),
    Edit {
        msg_id: String,
        content: String,
        edited_at: Option<String>,
    },
    Delete(String),
}

pub async fn run(ctx: &Ctx, channel: &str, limit: u32, once: bool) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = Api::new(&token);

    if once {
        let messages = api.fetch_messages(channel, None, limit).await?;
        for m in &messages {
            println!("{}", output::format_message(m));
        }
        return Ok(());
    }

    let meta = api
        .resolve_channel_context(channel)
        .await
        .unwrap_or_else(|e| {
            output::warn(&format!("could not resolve channel context: {}", e));
            ChannelContext::default()
        });

    // Read the resume cursor synchronously, before spawning the writer task,
    // so there is at most one `Db` handle alive at any moment.
    let last = {
        let db = Db::open(&ctx.db_path)?;
        db.last_msg_id(channel)?
    };

    // All persistence flows through this channel into a single writer task.
    let (db_tx, mut db_rx) = mpsc::unbounded_channel::<DbOp>();
    let db_path = ctx.db_path.clone();
    let writer_handle = tokio::spawn(async move {
        let mut db = match Db::open(&db_path) {
            Ok(d) => d,
            Err(e) => {
                output::err(&format!("DB writer init failed: {}", e));
                return;
            }
        };
        while let Some(op) = db_rx.recv().await {
            match op {
                DbOp::Insert(msgs) => {
                    if let Err(e) = db.insert_batch(&msgs) {
                        output::err(&format!("persist insert failed: {}", e));
                    }
                }
                DbOp::Edit {
                    msg_id,
                    content,
                    edited_at,
                } => {
                    if let Err(e) = db.apply_edit(&msg_id, Some(&content), edited_at.as_deref()) {
                        output::err(&format!("persist edit failed: {}", e));
                    }
                }
                DbOp::Delete(msg_id) => {
                    if let Err(e) = db.apply_delete(&msg_id) {
                        output::err(&format!("persist delete failed: {}", e));
                    }
                }
            }
        }
    });

    output::dim(&format!(
        "Connecting to Gateway for channel {}... press Ctrl+C to stop",
        channel
    ));

    let target_channel = Arc::new(channel.to_string());
    // `token` is `Zeroizing<String>`; `as_str()` exposes a `&str` that
    // `DiscordUser::new`'s `impl Into<String>` bound accepts (deref coercion
    // does not fire through generic `Into` parameters).
    let mut user = crate::DiscordUser::new(token.as_str());

    // MESSAGE_CREATE — print and persist.
    let _g_create = {
        let ch = Arc::clone(&target_channel);
        let db_tx = db_tx.clone();
        let meta = meta.clone();
        user.on_message_create(move |event| {
            let msg = &event.message;
            if msg.channel_id != *ch {
                return;
            }
            let ts = DateTime::parse_from_rfc3339(&msg.timestamp)
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|_| msg.timestamp.clone());
            let sender = msg
                .author
                .global_name
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&msg.author.username);
            let display = msg.content.replace('\n', " ");
            println!("{} {}: {}", ts.dimmed(), sender.bold(), display);

            // Persist via the same DTO path the REST sync uses, so the
            // schema and edit semantics stay consistent.
            let raw = crate::cli::types::MessageRaw {
                id: msg.id.clone(),
                channel_id: Some(msg.channel_id.clone()),
                content: msg.content.clone(),
                timestamp: msg.timestamp.clone(),
                edited_timestamp: msg.edited_timestamp.clone(),
                author: crate::cli::types::AuthorDto {
                    id: msg.author.id.clone(),
                    username: msg.author.username.clone(),
                    global_name: msg.author.global_name.clone(),
                },
                attachments: msg
                    .attachments
                    .iter()
                    .map(|a| crate::cli::types::AttachmentDto {
                        id: a.id.clone(),
                        filename: a.filename.clone(),
                        url: Some(a.url.clone()),
                        content_type: a.content_type.clone(),
                        size: a.size,
                    })
                    .collect(),
                embeds: Vec::new(),
            };
            let stored = StoredMessage::from_raw_with_ctx(&raw, &msg.channel_id, &meta);
            let _ = db_tx.send(DbOp::Insert(vec![stored]));
        })
        .await
    };

    // MESSAGE_UPDATE — print and persist edit.
    let _g_update = {
        let ch = Arc::clone(&target_channel);
        let db_tx = db_tx.clone();
        user.on_message_update(move |event| {
            if event.channel_id != *ch {
                return;
            }
            let Some(content) = event.content.as_deref() else {
                return;
            };
            let display = content.replace('\n', " ");
            println!(
                "{} {} {}: {}",
                "          ".dimmed(),
                event.id.dimmed(),
                "[edited]".yellow(),
                display
            );
            let _ = db_tx.send(DbOp::Edit {
                msg_id: event.id.clone(),
                content: content.to_string(),
                edited_at: event.edited_timestamp.clone(),
            });
        })
        .await
    };

    // MESSAGE_DELETE — annotate the stored row.
    let _g_delete = {
        let ch = Arc::clone(&target_channel);
        let db_tx = db_tx.clone();
        user.on_message_delete(move |event| {
            if event.channel_id != *ch {
                return;
            }
            println!(
                "{} {} {}",
                "          ".dimmed(),
                event.id.dimmed(),
                "[deleted]".red()
            );
            let _ = db_tx.send(DbOp::Delete(event.id.clone()));
        })
        .await
    };

    user.init()
        .await
        .map_err(|e| anyhow!("Gateway connection failed: {}", e))?;

    let snapshot_label = if last.is_some() {
        "since last sync"
    } else {
        "recent"
    };
    let initial = api
        .fetch_messages_page(channel, last.as_deref(), None, limit, &meta)
        .await?;
    if !initial.messages.is_empty() {
        output::dim(&format!(
            "--- {} ({} messages) ---",
            snapshot_label,
            initial.messages.len()
        ));
        for m in &initial.messages {
            println!("{}", output::format_message(m));
        }
        // Hand the snapshot to the writer as a single batched insert. The
        // channel preserves order, and the table's ON CONFLICT clause makes
        // any overlap with already-arrived live events a no-op.
        let _ = db_tx.send(DbOp::Insert(initial.messages));
        output::dim("--- live stream ---");
    }

    output::dim("Connected. Streaming messages, edits, and deletes...");

    tokio::signal::ctrl_c()
        .await
        .map_err(|e| anyhow!("failed to install Ctrl+C handler: {}", e))?;
    output::dim("\nDisconnecting...");
    user.disconnect().await;

    // Drop the listener guards (each holds a cloned db_tx) and our own
    // sender so the writer task's `recv()` returns None and the task exits
    // cleanly after draining every queued op.
    drop(_g_create);
    drop(_g_update);
    drop(_g_delete);
    drop(db_tx);
    let _ = writer_handle.await;

    output::dim("Stopped.");
    Ok(())
}
