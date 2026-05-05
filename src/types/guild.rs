//! Discord guild and channel types

use serde::{Deserialize, Serialize};

use super::User;
use crate::ChannelType;

/// Discord guild (server)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Guild {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub icon_hash: Option<String>,
    #[serde(default)]
    pub splash: Option<String>,
    #[serde(default)]
    pub discovery_splash: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: Option<bool>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub permissions: Option<String>,
    /// Voice region ID (deprecated; replaced by per-channel `rtc_region`).
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub afk_channel_id: Option<String>,
    #[serde(default)]
    pub afk_timeout: Option<u64>,
    #[serde(default)]
    pub widget_enabled: Option<bool>,
    #[serde(default)]
    pub widget_channel_id: Option<String>,
    #[serde(default)]
    pub default_message_notifications: Option<u8>,
    #[serde(default)]
    pub explicit_content_filter: Option<u8>,
    #[serde(default)]
    pub mfa_level: Option<u8>,
    #[serde(default)]
    pub application_id: Option<String>,
    #[serde(default)]
    pub system_channel_id: Option<String>,
    #[serde(default)]
    pub system_channel_flags: Option<u64>,
    #[serde(default)]
    pub rules_channel_id: Option<String>,
    #[serde(default)]
    pub max_presences: Option<u64>,
    #[serde(default)]
    pub max_members: Option<u64>,
    #[serde(default)]
    pub vanity_url_code: Option<String>,
    #[serde(default)]
    pub preferred_locale: Option<String>,
    #[serde(default)]
    pub public_updates_channel_id: Option<String>,
    #[serde(default)]
    pub max_video_channel_users: Option<u64>,
    #[serde(default)]
    pub max_stage_video_channel_users: Option<u64>,
    #[serde(default)]
    pub approximate_member_count: Option<u64>,
    #[serde(default)]
    pub approximate_presence_count: Option<u64>,
    #[serde(default)]
    pub welcome_screen: Option<WelcomeScreen>,
    #[serde(default)]
    pub premium_progress_bar_enabled: Option<bool>,
    #[serde(default)]
    pub safety_alerts_channel_id: Option<String>,
    #[serde(default)]
    pub incidents_data: Option<IncidentsData>,
    #[serde(default)]
    pub member_count: Option<u32>,
    #[serde(default)]
    pub premium_subscription_count: u32,
    #[serde(default)]
    pub premium_tier: u8,
    #[serde(default)]
    pub verification_level: u8,
    #[serde(default)]
    pub nsfw_level: u8,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub channels: Vec<Channel>,
    #[serde(default)]
    pub emojis: Vec<GuildEmoji>,
    #[serde(default)]
    pub stickers: Vec<Sticker>,
    #[serde(default)]
    pub joined_at: Option<String>,
    #[serde(default)]
    pub large: bool,
    #[serde(default)]
    pub lazy: bool,
}

/// Welcome screen object — shown to new members when they join a Community guild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeScreen {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub welcome_channels: Vec<WelcomeChannel>,
}

/// A featured channel inside a [`WelcomeScreen`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WelcomeChannel {
    pub channel_id: String,
    pub description: String,
    #[serde(default)]
    pub emoji_id: Option<String>,
    #[serde(default)]
    pub emoji_name: Option<String>,
}

/// Guild incidents data — administrative pause flags / detection timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentsData {
    #[serde(default)]
    pub invites_disabled_until: Option<String>,
    #[serde(default)]
    pub dms_disabled_until: Option<String>,
    #[serde(default)]
    pub dm_spam_detected_at: Option<String>,
    #[serde(default)]
    pub raid_detected_at: Option<String>,
}

impl Guild {
    /// Get guild icon URL
    pub fn icon_url(&self, size: u32) -> Option<String> {
        self.icon.as_ref().map(|hash| {
            let ext = if hash.starts_with("a_") { "gif" } else { "png" };
            format!("https://cdn.discordapp.com/icons/{}/{}.{}?size={}", self.id, hash, ext, size)
        })
    }

    /// Get guild splash URL (the invite screen background image), if set.
    ///
    /// Mirrors serenity's `Guild::splash_url()`.
    pub fn splash_url(&self) -> Option<String> {
        self.splash.as_ref().map(|hash| format!("https://cdn.discordapp.com/splashes/{}/{}.png?size=512", self.id, hash))
    }

    /// Get guild banner URL, if set.
    ///
    /// Mirrors serenity's `Guild::banner_url()`.
    pub fn banner_url(&self) -> Option<String> {
        self.banner.as_ref().map(|hash| {
            let ext = if hash.starts_with("a_") { "gif" } else { "webp" };
            format!("https://cdn.discordapp.com/banners/{}/{}.{}?size=512", self.id, hash, ext)
        })
    }

    /// Find a role in this guild by name (case-insensitive).
    ///
    /// Returns the first matching role, or `None` if not found.
    /// Mirrors serenity's `Guild::role_by_name()`.
    pub fn role_by_name(&self, name: &str) -> Option<&Role> {
        let lower = name.to_lowercase();
        self.roles.iter().find(|r| r.name.to_lowercase() == lower)
    }

    /// Find a channel in this guild by name (case-insensitive).
    ///
    /// Returns the first matching channel, or `None` if not found.
    /// Mirrors serenity's `Guild::channel_id_from_name()`.
    pub fn channel_by_name(&self, name: &str) -> Option<&Channel> {
        let lower = name.to_lowercase();
        self.channels.iter().find(|c| c.name.as_deref().map(|n| n.to_lowercase()) == Some(lower.as_str().to_string()))
    }

    /// Extract the guild's creation timestamp from its snowflake ID.
    ///
    /// Returns `None` if the `id` field cannot be parsed as a valid snowflake.
    pub fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        use chrono::TimeZone;
        let id: u64 = self.id.parse().ok()?;
        let ms = (id >> 22) + 1_420_070_400_000; // Discord epoch
        chrono::Utc.timestamp_millis_opt(ms as i64).single()
    }
}

/// Discord channel
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Channel {
    pub id: String,
    #[serde(default, rename = "type")]
    pub channel_type: ChannelType,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub permission_overwrites: Vec<PermissionOverwrite>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub last_message_id: Option<String>,
    #[serde(default)]
    pub bitrate: Option<u32>,
    #[serde(default)]
    pub user_limit: Option<u32>,
    #[serde(default)]
    pub rate_limit_per_user: Option<u32>,
    #[serde(default)]
    pub recipients: Vec<User>,
    #[serde(default)]
    pub recipient_ids: Vec<String>,
    #[serde(default)]
    pub flags: u32,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub application_id: Option<String>,
    #[serde(default)]
    pub managed: Option<bool>,
    #[serde(default)]
    pub rtc_region: Option<String>,
    #[serde(default)]
    pub video_quality_mode: Option<u8>,
    #[serde(default)]
    pub message_count: Option<u64>,
    #[serde(default)]
    pub member_count: Option<u32>,
    #[serde(default)]
    pub thread_metadata: Option<ThreadMetadata>,
    #[serde(default)]
    pub member: Option<ThreadMember>,
    #[serde(default)]
    pub default_auto_archive_duration: Option<u32>,
    #[serde(default)]
    pub permissions: Option<String>,
    #[serde(default)]
    pub total_message_sent: Option<u64>,
    #[serde(default)]
    pub available_tags: Option<Vec<ForumTag>>,
    #[serde(default)]
    pub applied_tags: Option<Vec<String>>,
    #[serde(default)]
    pub default_reaction_emoji: Option<DefaultReaction>,
    #[serde(default)]
    pub default_thread_rate_limit_per_user: Option<u64>,
    #[serde(default)]
    pub default_sort_order: Option<u8>,
    #[serde(default)]
    pub default_forum_layout: Option<u8>,
}

/// Metadata describing a thread channel's archival/lock state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMetadata {
    pub archived: bool,
    pub auto_archive_duration: u32,
    pub archive_timestamp: String,
    pub locked: bool,
    #[serde(default)]
    pub invitable: Option<bool>,
    #[serde(default)]
    pub create_timestamp: Option<String>,
}

/// A thread member — represents a user's participation in a thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMember {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    pub join_timestamp: String,
    pub flags: u64,
    #[serde(default)]
    pub member: Option<serde_json::Value>,
}

/// A forum/media channel tag, optionally assignable to threads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumTag {
    pub id: String,
    pub name: String,
    pub moderated: bool,
    #[serde(default)]
    pub emoji_id: Option<String>,
    #[serde(default)]
    pub emoji_name: Option<String>,
}

/// The default emoji shown on a forum/media channel's reaction button.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultReaction {
    #[serde(default)]
    pub emoji_id: Option<String>,
    #[serde(default)]
    pub emoji_name: Option<String>,
}

impl Channel {
    /// Format this channel as a mention string (`<#channel_id>`).
    ///
    /// Mirrors serenity's `Channel::mention()` / `GuildChannel::mention()`.
    pub fn mention(&self) -> String {
        format!("<#{}>", self.id)
    }
}

/// Permission overwrite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOverwrite {
    pub id: String,
    #[serde(rename = "type")]
    pub overwrite_type: u8,
    #[serde(default)]
    pub allow: String,
    #[serde(default)]
    pub deny: String,
}

/// Discord role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: u32,
    #[serde(default)]
    pub hoist: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub unicode_emoji: Option<String>,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub permissions: String,
    #[serde(default)]
    pub managed: bool,
    #[serde(default)]
    pub mentionable: bool,
    #[serde(default)]
    pub flags: u32,
    #[serde(default)]
    pub tags: Option<serde_json::Value>,
}

impl Role {
    /// Format this role as a mention string (`<@&role_id>`).
    ///
    /// Mirrors serenity's `Role::mention()` / `Mentionable` impl.
    pub fn mention(&self) -> String {
        format!("<@&{}>", self.id)
    }

    /// Return the role color as a `#RRGGBB` hex string.
    /// Returns `"#000000"` (black) when the color is 0 (default/colorless
    /// role).
    ///
    /// Mirrors serenity's `Colour` formatting helpers.
    pub fn color_hex(&self) -> String {
        format!("#{:06X}", self.color)
    }
}

/// Guild emoji
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildEmoji {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    /// The user who created this emoji (present when fetched via GET
    /// /guilds/{id}/emojis)
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub require_colons: bool,
    #[serde(default)]
    pub managed: bool,
    #[serde(default)]
    pub animated: bool,
    #[serde(default)]
    pub available: bool,
}

impl GuildEmoji {
    /// CDN URL for this emoji's image.
    ///
    /// Mirrors serenity's `Emoji::url()`.
    pub fn url(&self) -> String {
        let ext = if self.animated { "gif" } else { "webp" };
        format!("https://cdn.discordapp.com/emojis/{}.{}?size=128", self.id, ext)
    }

    /// Format as a reaction or message string.
    ///
    /// Returns `<:name:id>` for static emojis or `<a:name:id>` for animated
    /// ones. When `name` is `None`, falls back to the emoji ID.
    ///
    /// Mirrors serenity's `Emoji::to_string()` / reaction formatting.
    pub fn reaction_string(&self) -> String {
        let name = self.name.as_deref().unwrap_or(&self.id);
        if self.animated {
            format!("<a:{}:{}>", name, self.id)
        } else {
            format!("<:{}:{}>", name, self.id)
        }
    }
}

/// Discord sticker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sticker {
    pub id: String,
    #[serde(default)]
    pub pack_id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
    /// Sticker type: 1 = STANDARD, 2 = GUILD.
    #[serde(default, rename = "type")]
    pub sticker_type: u8,
    #[serde(default)]
    pub format_type: u8,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub sort_value: Option<u32>,
    #[serde(default)]
    pub user: Option<User>,
}

/// Guild invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    #[serde(default)]
    pub guild: Option<PartialGuild>,
    #[serde(default)]
    pub channel: Option<PartialChannel>,
    #[serde(default)]
    pub inviter: Option<User>,
    #[serde(default)]
    pub uses: u32,
    #[serde(default)]
    pub max_uses: u32,
    #[serde(default)]
    pub max_age: u32,
    #[serde(default)]
    pub temporary: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(rename = "type", default)]
    pub invite_type: u8,
    #[serde(default)]
    pub approximate_presence_count: Option<u64>,
    #[serde(default)]
    pub approximate_member_count: Option<u64>,
    /// Target type: 1 = STREAM, 2 = EMBEDDED_APPLICATION.
    #[serde(default)]
    pub target_type: Option<u8>,
    #[serde(default)]
    pub target_user: Option<User>,
    #[serde(default)]
    pub target_application: Option<serde_json::Value>,
    #[serde(default)]
    pub flags: Option<u64>,
    #[serde(default)]
    pub guild_scheduled_event: Option<serde_json::Value>,
}

impl Invite {
    /// Get full invite URL
    pub fn url(&self) -> String {
        format!("https://discord.gg/{}", self.code)
    }
}

/// Partial guild for invite response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialGuild {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub splash: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub verification_level: u8,
    #[serde(default)]
    pub vanity_url_code: Option<String>,
    #[serde(default)]
    pub nsfw_level: u8,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub premium_subscription_count: u32,
}

/// Partial channel for invite response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialChannel {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type", default)]
    pub channel_type: ChannelType,
}

/// A change recorded in an audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogChange {
    /// Name of the changed field
    pub key: String,
    /// Previous value (absent for create)
    #[serde(default)]
    pub old_value: Option<serde_json::Value>,
    /// New value (absent for delete)
    #[serde(default)]
    pub new_value: Option<serde_json::Value>,
}

/// A single entry in the guild audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// ID of the entry
    pub id: String,
    /// ID of the affected entity (user, role, channel, …)
    #[serde(default)]
    pub target_id: Option<String>,
    /// User who performed the action
    #[serde(default)]
    pub user_id: Option<String>,
    /// [Action type](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events)
    pub action_type: u32,
    /// List of changed fields
    #[serde(default)]
    pub changes: Vec<AuditLogChange>,
    /// Additional metadata for certain action types
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    /// Optional reason attached to the audit log entry
    #[serde(default)]
    pub reason: Option<String>,
}

/// Response from GET /guilds/{id}/audit-logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub audit_log_entries: Vec<AuditLogEntry>,
    #[serde(default)]
    pub users: Vec<super::User>,
}

/// A Discord webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    /// Webhook type: 1=Incoming, 2=ChannelFollower, 3=Application
    #[serde(rename = "type")]
    pub webhook_type: u8,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    /// The webhook's secure token (only for Incoming webhooks)
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub application_id: Option<String>,
    /// URL for executing the webhook (only returned by `create_webhook`)
    #[serde(default)]
    pub url: Option<String>,
}

/// A guild ban entry returned by GET /guilds/{id}/bans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ban {
    /// The reason for the ban (may be null)
    pub reason: Option<String>,
    /// The banned user
    pub user: super::User,
}

/// An auto-moderation rule.
///
/// Returned by `GET /guilds/{id}/auto-moderation/rules` and related endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoModerationRule {
    /// Snowflake ID of the rule.
    pub id: String,
    /// Guild that owns the rule.
    pub guild_id: String,
    /// Human-readable rule name.
    pub name: String,
    /// User who created the rule.
    pub creator_id: String,
    /// Event type that triggers evaluation: 1=MESSAGE_SEND, 2=MEMBER_UPDATE.
    pub event_type: u8,
    /// Trigger type: 1=KEYWORD, 2=SPAM, 3=KEYWORD_PRESET, 4=MENTION_SPAM,
    /// 5=MEMBER_PROFILE.
    pub trigger_type: u8,
    /// Trigger metadata (keyword lists, presets, thresholds).
    pub trigger_metadata: serde_json::Value,
    /// Actions to perform when the rule is triggered.
    pub actions: Vec<serde_json::Value>,
    /// Whether the rule is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Channel IDs exempt from the rule.
    #[serde(default)]
    pub exempt_channels: Vec<String>,
    /// Role IDs exempt from the rule.
    #[serde(default)]
    pub exempt_roles: Vec<String>,
}

/// A guild scheduled event.
///
/// Returned by `GET /guilds/{id}/scheduled-events` and related endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledEvent {
    /// Snowflake ID of the event.
    pub id: String,
    /// Guild that owns the event.
    pub guild_id: String,
    /// Stage/voice channel where the event will be hosted, if applicable.
    pub channel_id: Option<String>,
    /// User who created the event.
    pub creator_id: Option<String>,
    /// Name of the event (1-100 characters).
    pub name: String,
    /// Description of the event (1-1000 characters).
    pub description: Option<String>,
    /// ISO 8601 start time.
    pub scheduled_start_time: String,
    /// ISO 8601 end time (required for EXTERNAL events).
    pub scheduled_end_time: Option<String>,
    /// Privacy level: 2 = GUILD_ONLY.
    pub privacy_level: u8,
    /// Status: 1=SCHEDULED, 2=ACTIVE, 3=COMPLETED, 4=CANCELLED.
    pub status: u8,
    /// Entity type: 1=STAGE_INSTANCE, 2=VOICE, 3=EXTERNAL.
    pub entity_type: u8,
    /// Entity ID (stage instance, if applicable).
    pub entity_id: Option<String>,
    /// Additional metadata for EXTERNAL events.
    pub entity_metadata: Option<serde_json::Value>,
    /// Creator user object (only present when `with_user_count=true` or via
    /// REST).
    pub creator: Option<super::User>,
    /// Number of users subscribed (only present when `with_user_count=true`).
    pub user_count: Option<u32>,
    /// Cover image hash.
    pub image: Option<String>,
    /// Recurrence rule for repeating events.
    #[serde(default)]
    pub recurrence_rule: Option<RecurrenceRule>,
}

/// Recurrence rule for a scheduled event.
///
/// Mirrors the `guild_scheduled_event_recurrence_rule` object documented at
/// <https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object>.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRule {
    /// ISO 8601 timestamp the recurrence interval starts at.
    pub start: String,
    /// ISO 8601 timestamp the recurrence interval ends at, if any.
    #[serde(default)]
    pub end: Option<String>,
    /// Frequency: 0=YEARLY, 1=MONTHLY, 2=WEEKLY, 3=DAILY.
    pub frequency: u8,
    /// Spacing between events (e.g. interval=2 with weekly = every other week).
    pub interval: u32,
    /// Specific weekdays (0=Mon..6=Sun) the event recurs on.
    #[serde(default)]
    pub by_weekday: Option<Vec<u8>>,
    /// Specific days within a week the event recurs on (`{n, day}` pairs).
    #[serde(default)]
    pub by_n_weekday: Option<Vec<serde_json::Value>>,
    /// Months (1-12) the event recurs on.
    #[serde(default)]
    pub by_month: Option<Vec<u8>>,
    /// Days of the month (1-31) the event recurs on.
    #[serde(default)]
    pub by_month_day: Option<Vec<u32>>,
    /// Days of the year (1-364) the event recurs on.
    #[serde(default)]
    pub by_year_day: Option<Vec<u32>>,
    /// Total number of times the event will recur, if bounded.
    #[serde(default)]
    pub count: Option<u32>,
}

/// A Stage instance — a live audio session in a Stage channel.
///
/// Returned by `GET/POST/PATCH /stage-instances/{channel_id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInstance {
    /// Snowflake ID of the stage instance.
    pub id: String,
    /// Guild that owns the stage channel.
    pub guild_id: String,
    /// The stage channel this instance is associated with.
    pub channel_id: String,
    /// The topic of the stage (1-120 characters).
    pub topic: String,
    /// Privacy level: 1 = PUBLIC, 2 = GUILD_ONLY.
    pub privacy_level: u8,
    /// Whether the stage channel invite is discoverable (deprecated, always
    /// false).
    #[serde(default)]
    pub discoverable_disabled: bool,
    /// Associated scheduled event ID, if any.
    pub guild_scheduled_event_id: Option<String>,
}

/// A voice region entry returned by GET /voice/regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceRegion {
    /// Unique ID for the region.
    pub id: String,
    /// Human-readable name of the region.
    pub name: String,
    /// Whether this is a custom region (for VIP guilds).
    #[serde(default)]
    pub custom: bool,
    /// Whether this is a deprecated region (avoid for new guilds).
    #[serde(default)]
    pub deprecated: bool,
    /// Whether this is the closest region to the current user.
    #[serde(default)]
    pub optimal: bool,
}

/// A soundboard sound — either a built-in default or a guild-uploaded sound.
///
/// Returned by `GET /soundboard-default-sounds` and
/// `GET /guilds/{id}/soundboard-sounds`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundboardSound {
    /// Unique ID of the sound.
    pub sound_id: String,
    /// Display name (max 32 characters).
    pub name: String,
    /// Volume (0.0–1.0).
    pub volume: f64,
    /// Custom emoji ID (if the sound uses a custom emoji).
    pub emoji_id: Option<String>,
    /// Unicode emoji (if the sound uses a built-in emoji).
    pub emoji_name: Option<String>,
    /// Guild ID (absent for default sounds).
    pub guild_id: Option<String>,
    /// Whether this sound is available (false when guild loses premium).
    #[serde(default = "default_true")]
    pub available: bool,
    /// User who uploaded the sound (present when fetched via guild endpoint).
    pub user: Option<super::User>,
}

fn default_true() -> bool {
    true
}

/// A Discord application command (slash command, user command, or message
/// command).
///
/// Returned by `GET /applications/{id}/commands` and related endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommand {
    /// Snowflake ID of the command.
    pub id: String,
    /// Command type: 1=CHAT_INPUT, 2=USER, 3=MESSAGE.
    #[serde(rename = "type", default = "default_command_type")]
    pub command_type: u8,
    /// Application that owns the command.
    pub application_id: String,
    /// Guild ID, if this is a guild-specific command.
    pub guild_id: Option<String>,
    /// Command name (1-32 characters, lowercase for CHAT_INPUT).
    pub name: String,
    /// Description (1-100 chars for CHAT_INPUT, empty for USER/MESSAGE).
    #[serde(default)]
    pub description: String,
    /// Command options (parameters / subcommands).
    #[serde(default)]
    pub options: Vec<serde_json::Value>,
    /// Whether the command is enabled in DMs (global commands only).
    #[serde(default = "default_true")]
    pub dm_permission: bool,
    /// Whether the command is age-restricted (NSFW).
    #[serde(default)]
    pub nsfw: bool,
    /// Auto-incrementing version snowflake.
    pub version: Option<String>,
}

fn default_command_type() -> u8 {
    1
}
