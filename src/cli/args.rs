//! clap CLI definitions.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
pub enum TimelineGrouping {
    Day,
    Hour,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ExportFormat {
    Text,
    Json,
    Yaml,
}

#[derive(Parser, Debug)]
#[command(name = "discord", version, about = "Local-first Discord archival CLI")]
pub struct Cli {
    /// Override token resolution (flag wins over env and .env)
    #[arg(long, global = true)]
    pub token: Option<String>,

    /// Override SQLite database path (default `$DISCORD_DB`, else `~/.discord/discord.db`)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Disable ANSI color output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Output results as JSON
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)] // parsed once per process
pub enum Cmd {
    /// Auto-discover and (optionally) save the Discord user token
    Auth(AuthArgs),
    /// Verify the configured token works
    Status,
    /// Show profile of the authenticated user
    Whoami,
    /// Discord API operations (guilds, channels, sync, ...)
    #[command(subcommand)]
    Dc(DcCmd),
    /// Search the local message archive
    Search(SearchArgs),
    /// Show newest stored messages
    Recent(RecentArgs),
    /// Per-channel message counts
    Stats,
    /// Show today's messages grouped by channel
    Today(TodayArgs),
    /// Show most active senders
    Top(TopArgs),
    /// Show message activity over time
    Timeline(TimelineArgs),
    /// Export messages to file
    Export(ExportArgs),
    /// Delete stored messages for a channel
    Purge(PurgeArgs),
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    /// Save the discovered token to `./.env`
    #[arg(long)]
    pub save: bool,
}

#[derive(Subcommand, Debug)]
pub enum DcCmd {
    /// List joined guilds
    Guilds,
    /// List text channels in a guild (id or name)
    Channels {
        /// Guild ID (digits) or partial guild name
        guild: String,
    },
    /// Incremental sync: fetch only new messages
    Sync {
        /// Channel ID
        channel: String,
        /// Maximum messages to fetch
        #[arg(short = 'n', long, default_value_t = 5000)]
        limit: u32,
    },
    /// Discover all guilds + text channels and sync each
    SyncAll {
        /// Maximum messages per channel
        #[arg(short = 'n', long, default_value_t = 200)]
        limit: u32,
    },
    /// Fetch historical messages backward into local DB
    History {
        /// Channel ID
        channel: String,
        /// Maximum messages to fetch
        #[arg(short = 'n', long, default_value_t = 1000)]
        limit: u32,
    },
    /// Stream a channel's messages in real time via Gateway WebSocket
    Tail {
        /// Channel ID
        channel: String,
        /// Initial snapshot size
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: u32,
        /// Fetch once then exit (no Gateway connection)
        #[arg(long)]
        once: bool,
    },
    /// Search messages on Discord's server (remote search)
    #[command(name = "search")]
    SearchRemote {
        /// Guild ID or partial guild name
        guild: String,
        /// Search keyword (optional when filtering by author/mentions/has)
        #[arg(default_value = "")]
        keyword: String,
        /// Filter by channel ID
        #[arg(short = 'c', long)]
        channel: Option<String>,
        /// Filter by author user ID
        #[arg(long)]
        author: Option<String>,
        /// Only messages mentioning this user ID
        #[arg(long)]
        mentions: Option<String>,
        /// Only messages containing: link | embed | file | image | video | sound
        #[arg(long)]
        has: Option<String>,
        /// Only pinned messages
        #[arg(long)]
        pinned: bool,
        /// Only messages before this message ID
        #[arg(long)]
        before: Option<String>,
        /// Only messages after this message ID
        #[arg(long)]
        after: Option<String>,
        /// Maximum results
        #[arg(short = 'n', long, default_value_t = 25)]
        limit: u32,
    },
    /// List guild members
    Members {
        /// Guild ID or partial guild name
        guild: String,
        /// Maximum members to fetch
        #[arg(short = 'n', long, default_value_t = 50)]
        limit: u32,
    },
    /// Show detailed guild information
    Info {
        /// Guild ID or partial guild name
        guild: String,
    },
    /// Show pinned messages in a channel
    Pins {
        /// Channel ID
        channel: String,
    },
    /// List active threads in a guild
    Threads {
        /// Guild ID or partial guild name
        guild: String,
    },
    /// Show relationships (friends, blocked, pending)
    Relationships,
    /// List roles in a guild
    Roles {
        /// Guild ID or partial guild name
        guild: String,
    },
    /// List custom emojis in a guild
    Emojis {
        /// Guild ID or partial guild name
        guild: String,
    },
    /// Show a user's profile
    Profile {
        /// User ID (defaults to self)
        user_id: Option<String>,
    },
    /// List custom stickers in a guild
    Stickers {
        /// Guild ID or partial guild name
        guild: String,
    },
    /// Show recent audit log entries
    Audit {
        /// Guild ID or partial guild name
        guild: String,
        /// Maximum entries
        #[arg(short = 'n', long, default_value_t = 50)]
        limit: u8,
        /// Filter by the user (moderator) who performed the action
        #[arg(long)]
        user: Option<String>,
        /// Filter by audit log action type (numeric, e.g. 22 = MEMBER_BAN_ADD)
        #[arg(long = "action-type")]
        action_type: Option<u32>,
        /// Only entries before this audit log entry ID
        #[arg(long)]
        before: Option<String>,
        /// Only entries after this audit log entry ID
        #[arg(long)]
        after: Option<String>,
    },
    /// List scheduled events in a guild
    Events {
        /// Guild ID or partial guild name
        guild: String,
    },
    /// Download attachments from stored messages
    Download {
        /// Channel ID or local channel name
        channel: String,
        /// Output directory
        #[arg(short = 'o', long, default_value = "downloads")]
        output: PathBuf,
        /// Only from the last N hours
        #[arg(long)]
        hours: Option<i64>,
    },
    /// Export full guild snapshot as JSON
    Snapshot {
        /// Guild ID or partial guild name
        guild: String,
        /// Output file (stdout if omitted)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    // ---- Write / interactive client commands ----
    /// Send a message to a channel
    Send {
        /// Channel ID
        channel: String,
        /// Message content
        content: String,
        /// Reply to a message ID (sets message_reference)
        #[arg(long)]
        reply: Option<String>,
        /// Attach a file (repeatable: --file a.png --file b.pdf)
        #[arg(long = "file", value_name = "PATH")]
        files: Vec<PathBuf>,
        /// Attach a sticker by ID (repeatable: --sticker ID, max 3)
        #[arg(long = "sticker", value_name = "STICKER_ID")]
        stickers: Vec<String>,
        /// Send as a text-to-speech message
        #[arg(long)]
        tts: bool,
        /// Send silently (no push/desktop notification)
        #[arg(long)]
        silent: bool,
        /// Suppress link/embed unfurls on this message
        #[arg(long = "suppress-embeds")]
        suppress_embeds: bool,
        /// Don't ping anyone, even if the content @-mentions users/roles
        #[arg(long = "no-mentions")]
        no_mentions: bool,
        /// Idempotency nonce (used by Discord to de-duplicate sends)
        #[arg(long)]
        nonce: Option<String>,
    },
    /// Edit a message you authored
    Edit {
        /// Channel ID
        channel: String,
        /// Message ID to edit
        message: String,
        /// New message content
        content: String,
    },
    /// Delete a message
    Delete {
        /// Channel ID
        channel: String,
        /// Message ID to delete
        message: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Add a reaction to a message
    React {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
        /// Emoji: unicode (👍) or custom `name:id`
        emoji: String,
        /// Send as a super (burst) reaction
        #[arg(long)]
        burst: bool,
    },
    /// Remove your own reaction from a message
    Unreact {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
        /// Emoji: unicode (👍) or custom `name:id`
        emoji: String,
    },
    /// Pin a message in a channel
    Pin {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
    },
    /// Unpin a message in a channel
    Unpin {
        /// Channel ID
        channel: String,
        /// Message ID
        message: String,
    },
    /// Trigger the typing indicator in a channel
    Typing {
        /// Channel ID
        channel: String,
    },
    /// Send a direct message to a user
    Dm {
        /// Recipient user ID
        user_id: String,
        /// Message content
        content: String,
        /// Attach a file (repeatable: --file a.png --file b.pdf)
        #[arg(long = "file", value_name = "PATH")]
        files: Vec<PathBuf>,
    },
    /// Send a voice message (audio attachment flagged as a voice message)
    #[command(name = "voice-message")]
    VoiceMessage {
        /// Channel ID
        channel: String,
        /// Audio file to send
        file: PathBuf,
        /// Duration in seconds. True duration probing is out of scope; this
        /// value is sent as-is and defaults to 1.0 when omitted.
        #[arg(long, value_name = "SECS")]
        duration: Option<f64>,
        /// Base64-encoded waveform bytes. When omitted, a short placeholder
        /// waveform is generated and base64-encoded.
        #[arg(long, value_name = "BASE64")]
        waveform: Option<String>,
    },

    // ---- Nested write/client command groups (multi-agent buildout) ----
    /// Thread management (create, join, archive, members, …)
    #[command(subcommand)]
    Thread(crate::cli::commands::dcw_threads::ThreadCmd),
    /// Channel management (create, edit, delete, slowmode)
    #[command(subcommand)]
    Channel(crate::cli::commands::dcw_channels::ChannelCmd),
    /// Channel permission overwrites (set, clear, list)
    #[command(subcommand)]
    Perms(crate::cli::commands::dcw_perms::PermsCmd),
    /// Guild member ops (nick, roles, timeout, kick)
    #[command(subcommand)]
    Member(crate::cli::commands::dcw_members::MemberCmd),
    /// Guild bans (add, remove, list, get)
    #[command(subcommand)]
    Ban(crate::cli::commands::dcw_bans::BanCmd),
    /// Guild roles (create, edit, delete, list)
    #[command(subcommand)]
    Role(crate::cli::commands::dcw_roles::RoleCmd),
    /// Invites (create, delete, list, get)
    #[command(subcommand)]
    Invite(crate::cli::commands::dcw_invites::InviteCmd),
    /// Relationships / friends (add, accept, remove, block)
    #[command(subcommand)]
    Friend(crate::cli::commands::dcw_friends::FriendCmd),
    /// Custom guild emojis (create, rename, delete)
    #[command(subcommand)]
    Emoji(crate::cli::commands::dcw_emojis::EmojiCmd),
    /// Custom guild stickers (create, edit, delete)
    #[command(subcommand)]
    Sticker(crate::cli::commands::dcw_stickers::StickerCmd),
    /// Advanced message ops (bulk-delete, crosspost, reactions)
    #[command(subcommand)]
    Message(crate::cli::commands::dcw_msgops::MessageCmd),
    /// Guild settings & own membership (edit, set-nick, leave)
    #[command(subcommand)]
    Guild(crate::cli::commands::dcw_guild::GuildCmd),
    /// Scheduled events (create, edit, delete)
    #[command(subcommand)]
    Event(crate::cli::commands::dcw_events::EventCmd),
    /// Stage instances (start, get, edit, stop)
    #[command(subcommand)]
    Stage(crate::cli::commands::dcw_stage::StageCmd),
    /// Webhooks (create, edit, delete, execute, list)
    #[command(subcommand)]
    Webhook(crate::cli::commands::dcw_webhooks::WebhookCmd),
    /// Rich message composition (embed, tts, poll, mark-read)
    #[command(subcommand)]
    Compose(crate::cli::commands::dcw_compose::ComposeCmd),
    /// Presence / status (set online status & custom activity)
    #[command(subcommand)]
    Presence(crate::cli::commands::dcw_presence::PresenceCmd),
    /// Group DM management (create, add, remove, set-name)
    #[command(subcommand, name = "dm-group")]
    DmGroup(crate::cli::commands::dcw_dmgroup::DmGroupCmd),
    /// Edit your own account/profile (username, bio, pronouns, avatar, banner)
    #[command(subcommand)]
    Account(crate::cli::commands::dcw_account::AccountCmd),
    /// Auto-moderation rules (list, get, create, edit, delete)
    #[command(subcommand)]
    Automod(crate::cli::commands::dcw_automod::AutomodCmd),
    /// Soundboard sounds (list, get, create, edit, delete, send)
    #[command(subcommand)]
    Soundboard(crate::cli::commands::dcw_soundboard::SoundboardCmd),
    /// Application (slash) commands (list, get, create, delete)
    #[command(subcommand, name = "command")]
    Command(crate::cli::commands::dcw_commands::AppCommandCmd),
    /// Notification settings & read-state (mute, level, mark-read)
    #[command(subcommand)]
    Notify(crate::cli::commands::dcw_notify::NotifyCmd),
    /// List your open DM and group-DM channels
    Dms,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search keyword (case-insensitive substring)
    pub keyword: String,
    /// Filter by channel ID or local channel name
    #[arg(short = 'c', long)]
    pub channel: Option<String>,
    /// Maximum results
    #[arg(short = 'n', long, default_value_t = 50)]
    pub limit: i64,
}

#[derive(Args, Debug)]
pub struct RecentArgs {
    /// Filter by channel ID or local channel name
    #[arg(short = 'c', long)]
    pub channel: Option<String>,
    /// Only messages from the last N hours
    #[arg(long)]
    pub hours: Option<i64>,
    /// Maximum results
    #[arg(short = 'n', long, default_value_t = 50)]
    pub limit: i64,
}

#[derive(Args, Debug)]
pub struct TodayArgs {
    /// Filter by channel ID or local channel name
    #[arg(short = 'c', long)]
    pub channel: Option<String>,
}

#[derive(Args, Debug)]
pub struct TopArgs {
    /// Filter by channel ID or local channel name
    #[arg(short = 'c', long)]
    pub channel: Option<String>,
    /// Only messages from the last N hours
    #[arg(long)]
    pub hours: Option<i64>,
    /// Maximum results
    #[arg(short = 'n', long, default_value_t = 20)]
    pub limit: i64,
}

#[derive(Args, Debug)]
pub struct TimelineArgs {
    /// Filter by channel ID or local channel name
    #[arg(short = 'c', long)]
    pub channel: Option<String>,
    /// Only messages from the last N hours
    #[arg(long)]
    pub hours: Option<i64>,
    /// Group by "day" or "hour"
    #[arg(long, default_value = "day", value_enum)]
    pub by: TimelineGrouping,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Channel ID or local channel name
    pub channel: String,
    /// Output format: "text" or "json"
    #[arg(short = 'f', long, default_value = "text", value_enum)]
    pub format: ExportFormat,
    /// Output file (stdout if omitted)
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Only messages from the last N hours
    #[arg(long)]
    pub hours: Option<i64>,
}

#[derive(Args, Debug)]
pub struct PurgeArgs {
    /// Channel ID or local channel name
    pub channel: String,
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}
