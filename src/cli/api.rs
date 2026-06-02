//! Discord REST wrappers — read (`Api`) and write (`WriteApi`).
//!
//! Reads go through [`Api`], which wraps `DiscordHttpClient` in a
//! [`ReadOnlyHttp`] newtype exposing only `.get()`. Writes (send, edit,
//! delete, react, pin, typing, DMs) go through [`WriteApi`], which wraps a
//! [`WriteHttp`] newtype exposing the mutating verbs. Keeping the two
//! surfaces in separate types means read-only call sites still cannot reach
//! a write verb by accident — you have to construct a `WriteApi` on purpose.
//!
//! `auth::windows::validate_token` still uses raw `reqwest` directly (NOT
//! through this module) because it must run before a token exists in the
//! resolution chain.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use rand::Rng;
use serde::de::DeserializeOwned;
use tracing::debug;

use crate::client::DiscordHttpClient;
use crate::route::Route;

use crate::cli::types::{
    ActiveThreadsResponse, AuditLogResponse, ChannelContext, ChannelDto, EmojiDto, GuildDetailDto,
    GuildMemberDto, GuildSummary, MeResponse, MessageRaw, RelationshipDto, RoleDto,
    ScheduledEventDto, SearchResponse, StickerDto, StoredMessage, UserProfileDto,
};

/// Newtype that exposes ONLY `.get<T>()` from `DiscordHttpClient`.
/// Makes the read-only invariant a compile-time guarantee.
pub struct ReadOnlyHttp {
    inner: DiscordHttpClient,
}

impl ReadOnlyHttp {
    pub fn new(token: &str) -> Self {
        Self {
            inner: DiscordHttpClient::new(token, None, false),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, route: Route<'_>) -> Result<T> {
        self.inner
            .get::<T>(route)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}

/// Parse a Discord snowflake (string of digits) into a u64. Used at the
/// SQLite-TEXT ↔ `Route::GetMessages`-u64 boundary.
pub fn parse_snowflake(s: &str) -> Result<u64> {
    s.parse::<u64>()
        .with_context(|| format!("invalid snowflake id: {:?}", s))
}

/// Best-effort MIME type from a file extension, used when attaching files to
/// outgoing messages. Falls back to `application/octet-stream` for anything
/// unrecognized (Discord still accepts the upload).
fn guess_mime(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("apng") => "image/apng",
        Some("bmp") => "image/bmp",
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("pdf") => "application/pdf",
        Some("json") => "application/json",
        Some("txt") | Some("log") => "text/plain",
        Some("csv") => "text/csv",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
}

/// Read a file and build its `attachments[]` metadata entry plus the matching
/// `files[i]` multipart part. Returns `(metadata, part_name, part)`. Validates
/// the file exists/readable and has a usable filename, failing fast otherwise.
fn build_file_part(
    index: usize,
    path: &std::path::Path,
) -> Result<(serde_json::Value, String, reqwest::multipart::Part)> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading attachment {}", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("invalid attachment filename: {}", path.display()))?
        .to_string();
    let meta = serde_json::json!({ "id": index, "filename": filename });
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(guess_mime(path))
        .map_err(|e| anyhow!(e.to_string()))?;
    Ok((meta, format!("files[{}]", index), part))
}

/// Optional send-time message options, mirroring the controls the official
/// client exposes (TTS, silent/suppress-notifications, suppress-embeds,
/// idempotency nonce, and disabling all mention pings).
#[derive(Default, Clone, Debug)]
pub struct MessageOptions {
    pub tts: bool,
    /// SUPPRESS_NOTIFICATIONS (1 << 12) — "silent" message.
    pub silent: bool,
    /// SUPPRESS_EMBEDS (1 << 2) — hide link/embed unfurls.
    pub suppress_embeds: bool,
    pub nonce: Option<String>,
    /// Send `allowed_mentions: { parse: [] }` so the message pings no one.
    pub suppress_mentions: bool,
}

impl MessageOptions {
    /// Compute the message `flags` bitfield from the boolean options.
    pub fn flags(&self) -> u64 {
        let mut flags = 0u64;
        if self.suppress_embeds {
            flags |= 1 << 2;
        }
        if self.silent {
            flags |= 1 << 12;
        }
        flags
    }

    /// Apply tts/flags/nonce/allowed_mentions to an already-built message body.
    pub fn apply(&self, body: &mut serde_json::Value) {
        body["tts"] = serde_json::json!(self.tts);
        body["flags"] = serde_json::json!(self.flags());
        if let Some(n) = &self.nonce {
            body["nonce"] = serde_json::json!(n);
        }
        if self.suppress_mentions {
            body["allowed_mentions"] = serde_json::json!({ "parse": [] });
        }
    }
}

/// Result of a paginated message fetch.
///
/// `hit_limit` is true when the loop exited because `limit` was reached
/// rather than because Discord returned a short page — callers should
/// surface this so users know to re-run with a larger `-n`.
pub struct FetchedPage {
    pub messages: Vec<StoredMessage>,
    pub hit_limit: bool,
    pub oldest_msg_id: Option<String>,
}

pub struct Api {
    http: ReadOnlyHttp,
}

impl Api {
    pub fn new(token: &str) -> Self {
        Self {
            http: ReadOnlyHttp::new(token),
        }
    }

    pub async fn get_me(&self) -> Result<MeResponse> {
        self.http.get::<MeResponse>(Route::GetMe).await
    }

    pub async fn list_guilds(&self) -> Result<Vec<GuildSummary>> {
        self.http
            .get::<Vec<GuildSummary>>(Route::GetCurrentUserGuilds)
            .await
    }

    /// Lists text-class channels (types 0/5/15) sorted by `position`.
    pub async fn list_text_channels(&self, guild_id: &str) -> Result<Vec<ChannelDto>> {
        let gid = parse_snowflake(guild_id)?;
        let mut chans: Vec<ChannelDto> = self
            .http
            .get::<Vec<ChannelDto>>(Route::GetGuildChannels { guild_id: gid })
            .await?;
        chans.retain(|c| matches!(c.type_, 0 | 5 | 15));
        chans.sort_by_key(|c| c.position);
        Ok(chans)
    }

    /// Paginated message fetch with jittered sleep between pages.
    /// `after` is a snowflake string; converted to u64 at the boundary.
    pub async fn fetch_messages(
        &self,
        channel_id: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<StoredMessage>> {
        let page = self
            .fetch_messages_page(channel_id, after, None, limit, &ChannelContext::default())
            .await?;
        Ok(page.messages)
    }

    /// Paginated fetch with optional `before` cursor (for backfill) and
    /// caller-supplied channel context (so rows arrive pre-annotated).
    pub async fn fetch_messages_page(
        &self,
        channel_id: &str,
        after: Option<&str>,
        before: Option<&str>,
        limit: u32,
        ctx: &ChannelContext,
    ) -> Result<FetchedPage> {
        let cid = parse_snowflake(channel_id)?;
        let mut all: Vec<StoredMessage> = Vec::new();
        let mut remaining = limit;
        let mut after_cursor: Option<u64> = after.map(parse_snowflake).transpose()?;
        let mut before_cursor: Option<u64> = before.map(parse_snowflake).transpose()?;
        let use_after = after_cursor.is_some();
        let mut hit_limit = false;
        let mut oldest_id: Option<u64> = None;

        while remaining > 0 {
            let batch_limit = remaining.min(100);
            let route = Route::GetMessages {
                channel_id: cid,
                limit: Some(batch_limit),
                before: if use_after { None } else { before_cursor },
                after: after_cursor,
            };
            debug!(
                "fetch_messages: channel={} after={:?} before={:?} limit={}",
                channel_id, after_cursor, before_cursor, batch_limit
            );
            let page: Vec<MessageRaw> = self.http.get::<Vec<MessageRaw>>(route).await?;
            if page.is_empty() {
                break;
            }
            for raw in &page {
                all.push(StoredMessage::from_raw_with_ctx(raw, channel_id, ctx));
            }
            let page_len = page.len() as u32;
            remaining = remaining.saturating_sub(page_len);

            // Discord returns newest-first by default, but when `after=` is
            // supplied it returns oldest-first. Pick numeric extremes
            // regardless of order — these are the safe cursors.
            let max_id_in_page = page.iter().filter_map(|m| m.id.parse::<u64>().ok()).max();
            let min_id_in_page = page.iter().filter_map(|m| m.id.parse::<u64>().ok()).min();
            // `oldest_msg_id` is the resume cursor for *backward* pagination
            // (`dc history`). Only populate it when we are walking backward —
            // a forward sync's minimum is the *start* of its window, not the
            // tail to resume from. Mixing them poisoned the history cursor.
            if !use_after {
                if let Some(min) = min_id_in_page {
                    oldest_id = Some(oldest_id.map_or(min, |o| o.min(min)));
                }
            }

            if (page_len as u32) < batch_limit {
                break;
            }

            if remaining == 0 {
                // We exited because `limit` ran out; tell the caller.
                hit_limit = true;
                break;
            }

            if use_after {
                after_cursor = max_id_in_page;
            } else {
                before_cursor = min_id_in_page;
            }

            // Polite jittered backoff between pages.
            let sleep_ms = rand::rng().random_range(300u64..1000u64);
            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
        }

        // Sort ascending by snowflake numeric value before returning.
        all.sort_by_key(|m| m.msg_id.parse::<u128>().unwrap_or(0));
        Ok(FetchedPage {
            messages: all,
            hit_limit,
            oldest_msg_id: oldest_id.map(|n| n.to_string()),
        })
    }

    /// One-shot single-channel fetch — used to populate channel name when
    /// resolving metadata.
    pub async fn get_channel(&self, channel_id: &str) -> Result<ChannelDto> {
        let cid = parse_snowflake(channel_id)?;
        self.http
            .get::<ChannelDto>(Route::GetChannel { channel_id: cid })
            .await
    }

    /// Look up a channel's parent guild + names in one shot. Returns
    /// `Ok(ChannelContext { guild_id: None, .. })` for DMs/group DMs (i.e.
    /// no enclosing guild was found). Errors propagate — callers must NOT
    /// silently `unwrap_or_default()`, otherwise rows lose all metadata.
    ///
    /// Cost is O(guilds × channels). Cache the result per CLI invocation
    /// when calling more than once.
    pub async fn resolve_channel_context(&self, channel_id: &str) -> Result<ChannelContext> {
        let chan = self.get_channel(channel_id).await?;
        let guilds = self.list_guilds().await?;
        for g in &guilds {
            let gid = match parse_snowflake(&g.id) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // Don't swallow the per-guild error silently — log it. A
            // missing-permission guild is fine to skip; transport errors
            // should bubble.
            let chans: Vec<ChannelDto> = match self
                .http
                .get::<Vec<ChannelDto>>(Route::GetGuildChannels { guild_id: gid })
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    debug!("skipping guild {} for channel lookup: {}", g.id, e);
                    continue;
                }
            };
            if chans.iter().any(|c| c.id == channel_id) {
                return Ok(ChannelContext {
                    guild_id: Some(g.id.clone()),
                    guild_name: Some(g.name.clone()),
                    channel_name: chan.name.clone(),
                });
            }
        }
        // Found channel but no enclosing guild — DM, group DM, or a guild
        // we no longer have access to.
        Ok(ChannelContext {
            guild_id: None,
            guild_name: None,
            channel_name: chan.name,
        })
    }

    pub async fn get_guild_info(&self, guild_id: &str) -> Result<GuildDetailDto> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<GuildDetailDto>(Route::GetGuild {
                guild_id: gid,
                with_counts: true,
            })
            .await
    }

    pub async fn list_guild_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<GuildMemberDto>> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<Vec<GuildMemberDto>>(Route::GetGuildMembers {
                guild_id: gid,
                limit: limit.min(1000),
            })
            .await
    }

    pub async fn search_guild_messages(
        &self,
        guild_id: &str,
        content: &str,
        channel_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<StoredMessage>> {
        let gid = parse_snowflake(guild_id)?;
        let cid = channel_id.map(parse_snowflake).transpose()?;
        let resp: SearchResponse = self
            .http
            .get::<SearchResponse>(Route::SearchGuildMessages {
                guild_id: gid,
                content,
                channel_id: cid,
                limit: Some(limit.min(25)),
            })
            .await?;

        let mut results = Vec::new();
        for group in &resp.messages {
            for raw in group {
                let ch = raw.channel_id.as_deref().unwrap_or("");
                results.push(StoredMessage::from_raw(raw, ch));
            }
        }
        Ok(results)
    }

    /// Remote guild search with the full filter set the official client
    /// exposes: `content`, `author_id`, `mentions`, `has:` (link/embed/file/
    /// image/video/sound), `channel_id`, `pinned`, and `before`/`after`
    /// snowflake cursors. All are optional. Builds the query manually since the
    /// typed `SearchGuildMessages` route only carries `content`.
    #[allow(clippy::too_many_arguments)]
    pub async fn search_guild_messages_advanced(
        &self,
        guild_id: &str,
        content: Option<&str>,
        channel_id: Option<&str>,
        author_id: Option<&str>,
        mentions: Option<&str>,
        has: Option<&str>,
        pinned: Option<bool>,
        before: Option<&str>,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<StoredMessage>> {
        let gid = parse_snowflake(guild_id)?;
        let mut params: Vec<String> = Vec::new();
        if let Some(c) = content {
            params.push(format!("content={}", urlencoding::encode(c)));
        }
        if let Some(cid) = channel_id {
            parse_snowflake(cid)?;
            params.push(format!("channel_id={}", cid));
        }
        if let Some(a) = author_id {
            parse_snowflake(a)?;
            params.push(format!("author_id={}", a));
        }
        if let Some(m) = mentions {
            parse_snowflake(m)?;
            params.push(format!("mentions={}", m));
        }
        if let Some(h) = has {
            params.push(format!("has={}", urlencoding::encode(h)));
        }
        if let Some(p) = pinned {
            params.push(format!("pinned={}", p));
        }
        if let Some(b) = before {
            parse_snowflake(b)?;
            params.push(format!("max_id={}", b));
        }
        if let Some(a) = after {
            parse_snowflake(a)?;
            params.push(format!("min_id={}", a));
        }
        params.push(format!("limit={}", limit.min(25)));
        let url = format!("guilds/{}/messages/search?{}", gid, params.join("&"));

        let resp: SearchResponse = self
            .http
            .get::<SearchResponse>(Route::Custom(url.into()))
            .await?;
        let mut results = Vec::new();
        for group in &resp.messages {
            for raw in group {
                let ch = raw.channel_id.as_deref().unwrap_or("");
                results.push(StoredMessage::from_raw(raw, ch));
            }
        }
        Ok(results)
    }

    pub async fn get_pins(&self, channel_id: &str) -> Result<Vec<MessageRaw>> {
        let cid = parse_snowflake(channel_id)?;
        self.http
            .get::<Vec<MessageRaw>>(Route::GetPins { channel_id: cid })
            .await
    }

    pub async fn get_active_threads(&self, guild_id: &str) -> Result<ActiveThreadsResponse> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<ActiveThreadsResponse>(Route::GetActiveThreads { guild_id: gid })
            .await
    }

    pub async fn get_relationships(&self) -> Result<Vec<RelationshipDto>> {
        self.http
            .get::<Vec<RelationshipDto>>(Route::GetRelationships)
            .await
    }

    pub async fn get_guild_roles(&self, guild_id: &str) -> Result<Vec<RoleDto>> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<Vec<RoleDto>>(Route::GetGuildRoles { guild_id: gid })
            .await
    }

    pub async fn get_guild_emojis(&self, guild_id: &str) -> Result<Vec<EmojiDto>> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<Vec<EmojiDto>>(Route::GetGuildEmojis { guild_id: gid })
            .await
    }

    pub async fn get_user_profile(&self, user_id: &str) -> Result<UserProfileDto> {
        let uid = parse_snowflake(user_id)?;
        self.http
            .get::<UserProfileDto>(Route::GetUserProfile { user_id: uid, guild_id: None })
            .await
    }

    pub async fn get_guild_stickers(&self, guild_id: &str) -> Result<Vec<StickerDto>> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<Vec<StickerDto>>(Route::GetGuildStickers { guild_id: gid })
            .await
    }

    pub async fn get_guild_audit_logs(&self, guild_id: &str, limit: u8) -> Result<AuditLogResponse> {
        self.get_guild_audit_logs_filtered(guild_id, limit, None, None, None, None)
            .await
    }

    /// Audit log with optional `user_id`, `action_type`, and `before`/`after`
    /// snowflake cursors. All filters map to documented query params.
    pub async fn get_guild_audit_logs_filtered(
        &self,
        guild_id: &str,
        limit: u8,
        user_id: Option<&str>,
        action_type: Option<u32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> Result<AuditLogResponse> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<AuditLogResponse>(Route::GetGuildAuditLogs {
                guild_id: gid,
                user_id: user_id.map(parse_snowflake).transpose()?,
                action_type,
                before: before.map(parse_snowflake).transpose()?,
                after: after.map(parse_snowflake).transpose()?,
                limit: Some(limit.min(100)),
            })
            .await
    }

    pub async fn get_scheduled_events(&self, guild_id: &str) -> Result<Vec<ScheduledEventDto>> {
        let gid = parse_snowflake(guild_id)?;
        self.http
            .get::<Vec<ScheduledEventDto>>(Route::GetGuildScheduledEvents { guild_id: gid })
            .await
    }

    pub async fn resolve_guild_id(&self, guild: &str) -> Result<String> {
        if guild.chars().all(|c| c.is_ascii_digit()) {
            return Ok(guild.to_string());
        }
        let guilds = self.list_guilds().await?;
        let needle = guild.to_lowercase();
        let matches: Vec<_> = guilds
            .iter()
            .filter(|g| g.name.to_lowercase().contains(&needle))
            .collect();
        match matches.len() {
            0 => Err(anyhow!("Guild '{}' not found.", guild)),
            1 => Ok(matches[0].id.clone()),
            n => {
                let names: Vec<String> = matches.iter().map(|g| g.name.clone()).collect();
                Err(anyhow!(
                    "{} guilds match '{}': {}. Use a guild ID instead.",
                    n,
                    guild,
                    names.join(", ")
                ))
            }
        }
    }
}

/// Newtype that exposes the mutating verbs of `DiscordHttpClient`.
///
/// Separate from [`ReadOnlyHttp`] so read-only command code can't reach a
/// write verb by accident — a `WriteApi` has to be constructed deliberately.
pub struct WriteHttp {
    inner: DiscordHttpClient,
}

impl WriteHttp {
    pub fn new(token: &str) -> Self {
        Self {
            inner: DiscordHttpClient::new(token, None, false),
        }
    }

    /// `true` for `Bot `/`Bearer ` tokens. User-account fingerprint fields
    /// (e.g. `mobile_network_type`) must be omitted under these.
    fn is_application_token(&self) -> bool {
        self.inner.is_application_token()
    }

    async fn post<T: DeserializeOwned>(
        &self,
        route: Route<'_>,
        body: serde_json::Value,
    ) -> Result<T> {
        self.inner
            .post::<T, _>(route, body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn patch<T: DeserializeOwned>(
        &self,
        route: Route<'_>,
        body: serde_json::Value,
    ) -> Result<T> {
        self.inner
            .patch::<T, _>(route, body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn delete(&self, route: Route<'_>) -> Result<()> {
        self.inner
            .delete(route)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn put_empty(&self, route: Route<'_>) -> Result<()> {
        self.inner
            .put_empty(route)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    async fn post_empty(&self, route: Route<'_>) -> Result<()> {
        self.inner
            .post_empty(route)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}

/// Write-capable Discord REST wrapper used by the interactive client
/// commands (send, edit, delete, react, pin, typing, DMs).
pub struct WriteApi {
    http: WriteHttp,
}

impl WriteApi {
    pub fn new(token: &str) -> Self {
        Self {
            http: WriteHttp::new(token),
        }
    }

    /// Send a message to a channel. `reply_to`, when set, makes this a reply
    /// (Discord `message_reference`). `sticker_ids`, when non-empty, attaches
    /// up to 3 stickers (caller must pre-validate count and snowflakes).
    /// Returns the created message.
    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
        reply_to: Option<&str>,
        sticker_ids: &[String],
        opts: &MessageOptions,
    ) -> Result<MessageRaw> {
        let cid = parse_snowflake(channel_id)?;
        let mut body = serde_json::json!({ "content": content });
        opts.apply(&mut body);
        // User-account tokens mimic the mobile client; bot endpoints reject
        // this field (HTTP 403 code 40333). Mirror the library's split.
        if !self.is_application_token() {
            body["mobile_network_type"] = serde_json::json!("unknown");
        }
        if let Some(mid) = reply_to {
            parse_snowflake(mid)?; // validate
            body["message_reference"] = serde_json::json!({ "message_id": mid });
        }
        if !sticker_ids.is_empty() {
            body["sticker_ids"] = serde_json::json!(sticker_ids);
        }
        self.http
            .post::<MessageRaw>(Route::CreateMessage { channel_id: cid }, body)
            .await
    }

    /// Send a message to a channel with one or more file attachments.
    ///
    /// Builds a `multipart/form-data` body: a `payload_json` part holding the
    /// message JSON (including an `attachments` array referencing each file by
    /// index + filename) plus `files[i]` parts carrying the raw bytes. Mirrors
    /// the user-token `mobile_network_type` handling of [`send_message`].
    /// `sticker_ids`, when non-empty, are included in the payload_json.
    pub async fn send_message_with_files(
        &self,
        channel_id: &str,
        content: &str,
        reply_to: Option<&str>,
        files: &[std::path::PathBuf],
        sticker_ids: &[String],
        opts: &MessageOptions,
    ) -> Result<serde_json::Value> {
        parse_snowflake(channel_id)?;
        let mut body = serde_json::json!({ "content": content });
        opts.apply(&mut body);
        if !self.is_application_token() {
            body["mobile_network_type"] = serde_json::json!("unknown");
        }
        if let Some(mid) = reply_to {
            parse_snowflake(mid)?; // validate
            body["message_reference"] = serde_json::json!({ "message_id": mid });
        }
        if !sticker_ids.is_empty() {
            body["sticker_ids"] = serde_json::json!(sticker_ids);
        }
        self.post_message_with_files(channel_id, body, files).await
    }

    /// Open a DM with a user and send `content` plus file attachments to it.
    pub async fn send_dm_with_files(
        &self,
        recipient_id: &str,
        content: &str,
        files: &[std::path::PathBuf],
    ) -> Result<serde_json::Value> {
        let dm = self.create_dm(recipient_id).await?;
        self.send_message_with_files(&dm.id, content, None, files, &[], &MessageOptions::default())
            .await
    }

    /// Build a `channels/{id}/messages` multipart request from an already-built
    /// message `body` and a list of files, injecting the `attachments` array.
    ///
    /// Shared by the file-aware send/DM/embed paths. Validates that every file
    /// exists and is readable, failing fast with a clear error otherwise.
    pub(crate) async fn post_message_with_files(
        &self,
        channel_id: &str,
        mut body: serde_json::Value,
        files: &[std::path::PathBuf],
    ) -> Result<serde_json::Value> {
        let mut attachments = Vec::with_capacity(files.len());
        let mut form = reqwest::multipart::Form::new();
        for (i, path) in files.iter().enumerate() {
            let (meta, name, part) = build_file_part(i, path)?;
            attachments.push(meta);
            form = form.part(name, part);
        }
        body["attachments"] = serde_json::Value::Array(attachments);
        self.send_message_multipart(channel_id, body, form).await
    }

    /// Send a voice message: a single audio attachment carrying `duration_secs`
    /// and a base64 `waveform`, with the message `flags` bit set to mark it a
    /// voice message (IS_VOICE_MESSAGE = 8192). Uploaded via the same multipart
    /// machinery as [`post_message_with_files`].
    ///
    /// `waveform` must already be base64-encoded (the CLI generates a short
    /// placeholder when the user does not supply one).
    pub async fn send_voice_message(
        &self,
        channel_id: &str,
        file: &std::path::Path,
        duration_secs: f64,
        waveform: &str,
    ) -> Result<serde_json::Value> {
        parse_snowflake(channel_id)?;
        let (mut meta, name, part) = build_file_part(0, file)?;
        meta["duration_secs"] = serde_json::json!(duration_secs);
        meta["waveform"] = serde_json::json!(waveform);

        let mut body = serde_json::json!({
            "content": "",
            "flags": 8192,
            "attachments": [meta],
        });
        if !self.is_application_token() {
            body["mobile_network_type"] = serde_json::json!("unknown");
        }

        let form = reqwest::multipart::Form::new().part(name, part);
        self.send_message_multipart(channel_id, body, form).await
    }

    /// Attach `payload_json` to a multipart `form` and POST it to the channel's
    /// messages endpoint. Shared tail of the file/voice send paths.
    async fn send_message_multipart(
        &self,
        channel_id: &str,
        body: serde_json::Value,
        form: reqwest::multipart::Form,
    ) -> Result<serde_json::Value> {
        let form = form.text(
            "payload_json",
            serde_json::to_string(&body).map_err(|e| anyhow!(e.to_string()))?,
        );
        self.raw()
            .post_raw_multipart::<serde_json::Value>(format!("channels/{}/messages", channel_id), form)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Edit a message you authored. Returns the updated message.
    pub async fn edit_message(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<MessageRaw> {
        let cid = parse_snowflake(channel_id)?;
        let mid = parse_snowflake(message_id)?;
        let body = serde_json::json!({ "content": content });
        self.http
            .patch::<MessageRaw>(
                Route::EditMessage {
                    channel_id: cid,
                    message_id: mid,
                },
                body,
            )
            .await
    }

    /// Delete a message.
    pub async fn delete_message(&self, channel_id: &str, message_id: &str) -> Result<()> {
        let cid = parse_snowflake(channel_id)?;
        let mid = parse_snowflake(message_id)?;
        self.http
            .delete(Route::DeleteMessage {
                channel_id: cid,
                message_id: mid,
            })
            .await
    }

    /// Add a reaction (`@me`). `emoji` is a raw unicode emoji (e.g. `👍`) or a
    /// custom-emoji `name:id`; it is percent-encoded for the URL path.
    /// When `burst` is `true`, sends a super (burst) reaction (`type=1`).
    pub async fn add_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
        burst: bool,
    ) -> Result<()> {
        let cid = parse_snowflake(channel_id)?;
        let mid = parse_snowflake(message_id)?;
        let encoded = urlencoding::encode(emoji).into_owned();
        if burst {
            self.http
                .put_empty(Route::Custom(
                    format!(
                        "channels/{}/messages/{}/reactions/{}/@me?location=Message&type=1",
                        cid, mid, encoded
                    )
                    .into(),
                ))
                .await
        } else {
            self.http
                .put_empty(Route::AddReaction {
                    channel_id: cid,
                    message_id: mid,
                    emoji: &encoded,
                })
                .await
        }
    }

    /// Remove your own reaction.
    pub async fn remove_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> Result<()> {
        let cid = parse_snowflake(channel_id)?;
        let mid = parse_snowflake(message_id)?;
        let encoded = urlencoding::encode(emoji).into_owned();
        self.http
            .delete(Route::RemoveOwnReaction {
                channel_id: cid,
                message_id: mid,
                emoji: &encoded,
            })
            .await
    }

    /// Pin a message in a channel.
    pub async fn pin_message(&self, channel_id: &str, message_id: &str) -> Result<()> {
        let cid = parse_snowflake(channel_id)?;
        let mid = parse_snowflake(message_id)?;
        self.http
            .put_empty(Route::PinMessage {
                channel_id: cid,
                message_id: mid,
            })
            .await
    }

    /// Unpin a message.
    pub async fn unpin_message(&self, channel_id: &str, message_id: &str) -> Result<()> {
        let cid = parse_snowflake(channel_id)?;
        let mid = parse_snowflake(message_id)?;
        self.http
            .delete(Route::UnpinMessage {
                channel_id: cid,
                message_id: mid,
            })
            .await
    }

    /// Fire the typing indicator for a channel (lasts ~10s server-side).
    pub async fn trigger_typing(&self, channel_id: &str) -> Result<()> {
        let cid = parse_snowflake(channel_id)?;
        self.http
            .post_empty(Route::TriggerTyping { channel_id: cid })
            .await
    }

    /// Open (or fetch) the DM channel with a user. Returns the DM channel.
    pub async fn create_dm(&self, recipient_id: &str) -> Result<ChannelDto> {
        parse_snowflake(recipient_id)?; // validate
        let body = serde_json::json!({ "recipients": [recipient_id] });
        self.http.post::<ChannelDto>(Route::CreateDm, body).await
    }

    /// Open a DM with a user and send `content` to it. Returns the message.
    pub async fn send_dm(&self, recipient_id: &str, content: &str) -> Result<MessageRaw> {
        let dm = self.create_dm(recipient_id).await?;
        self.send_message(&dm.id, content, None, &[], &MessageOptions::default())
            .await
    }

    /// Raw HTTP handle for the per-domain write modules in `cli/api_*.rs`.
    ///
    /// These modules add `impl WriteApi` blocks that reach arbitrary endpoints
    /// via [`Route::Custom`]. Returns `crate::error::Result` (DiscordError);
    /// map to `anyhow` at the call site with `.map_err(|e| anyhow!(e.to_string()))`.
    /// `cli/api_*.rs` is the only place (besides this file) where raw write
    /// verbs are allowed — see `tests/readonly.rs`.
    pub(crate) fn raw(&self) -> &DiscordHttpClient {
        &self.http.inner
    }

    /// `true` for `Bot `/`Bearer ` tokens — omit user-account fingerprint
    /// fields (e.g. `mobile_network_type`) under these. `pub(crate)` so the
    /// per-domain write modules in `cli/api_*.rs` can reach it (the underlying
    /// [`WriteHttp`] method is private to this module).
    pub(crate) fn is_application_token(&self) -> bool {
        self.http.is_application_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_snowflake_ok() {
        assert_eq!(
            parse_snowflake("123456789012345678").unwrap(),
            123456789012345678u64
        );
    }

    #[test]
    fn parse_snowflake_err() {
        assert!(parse_snowflake("not-a-number").is_err());
        assert!(parse_snowflake("").is_err());
    }
}
