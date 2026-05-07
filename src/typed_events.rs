//! Typed event definitions and handlers for Discord gateway events
//!
//! This module provides type-safe event handling by defining strongly-typed
//! event structs that are automatically deserialized from gateway dispatch
//! events.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{AuditLogChange, Channel, Guild, Member, Message, PartialApplication, Relationship, Role, User};

// ==================== Typed Event Structs ====================

/// READY event data - sent when the gateway connection is established
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyEvent {
    /// Gateway protocol version
    pub v: u8,
    /// Current user data
    pub user: User,
    /// List of relationships (friends, blocked, pending)
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    /// Session ID for resuming
    pub session_id: String,
    /// URL for resuming the gateway connection
    #[serde(default)]
    pub resume_gateway_url: Option<String>,
    /// Private channels (DMs)
    #[serde(default)]
    pub private_channels: Vec<Value>,
    /// Guild data
    #[serde(default)]
    pub guilds: Vec<Value>,
    /// Partial application object with id and flags (present for bot accounts)
    #[serde(default)]
    pub application: Option<PartialApplication>,
}

/// RESUMED event data — sent after a successful session resume.
///
/// Discord sends an empty `d` for this event; the struct exists so handlers
/// can pattern-match on `TypedEvent::Resumed`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResumedEvent {}

/// MESSAGE_CREATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct MessageCreateEvent {
    #[serde(flatten)]
    pub message: Message,
}

/// MESSAGE_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct MessageUpdateEvent {
    pub id: String,
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub author: Option<User>,
    #[serde(default)]
    pub embeds: Vec<Value>,
    #[serde(default)]
    pub attachments: Vec<Value>,
}

/// MESSAGE_DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDeleteEvent {
    pub id: String,
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
}

/// TYPING_START event data
#[derive(Debug, Clone, Deserialize)]
pub struct TypingStartEvent {
    pub user_id: String,
    pub channel_id: String,
    pub timestamp: u64,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub member: Option<Member>,
}

/// RELATIONSHIP_ADD event data
#[derive(Debug, Clone, Deserialize)]
pub struct RelationshipAddEvent {
    #[serde(flatten)]
    pub relationship: Relationship,
}

/// RELATIONSHIP_REMOVE event data
#[derive(Debug, Clone, Deserialize)]
pub struct RelationshipRemoveEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub relationship_type: u8,
    #[serde(default)]
    pub nickname: Option<String>,
}

/// PRESENCE_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct PresenceUpdateEvent {
    pub user: PresenceUser,
    pub status: String,
    #[serde(default)]
    pub client_status: Option<ClientStatus>,
    #[serde(default)]
    pub activities: Vec<Activity>,
    #[serde(default)]
    pub guild_id: Option<String>,
}

/// Minimal user data in presence updates
#[derive(Debug, Clone, Deserialize)]
pub struct PresenceUser {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// Client status per platform
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClientStatus {
    #[serde(default)]
    pub desktop: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub web: Option<String>,
}

/// User activity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Activity {
    pub name: String,
    #[serde(rename = "type")]
    pub activity_type: u8,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// GUILD_MEMBER_ADD event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildMemberAddEvent {
    pub guild_id: String,
    #[serde(flatten)]
    pub member: Member,
}

/// GUILD_MEMBER_REMOVE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildMemberRemoveEvent {
    pub guild_id: String,
    pub user: User,
}

/// GUILD_MEMBER_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildMemberUpdateEvent {
    pub guild_id: String,
    pub user: User,
    #[serde(default)]
    pub nick: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub joined_at: Option<String>,
    #[serde(default)]
    pub premium_since: Option<String>,
    #[serde(default)]
    pub pending: bool,
    #[serde(default)]
    pub communication_disabled_until: Option<String>,
}

/// MESSAGE_REACTION_ADD event data
#[derive(Debug, Clone, Deserialize)]
pub struct MessageReactionAddEvent {
    pub user_id: String,
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub member: Option<Member>,
    pub emoji: ReactionEmoji,
    #[serde(default)]
    pub burst: bool,
}

/// MESSAGE_REACTION_REMOVE event data
#[derive(Debug, Clone, Deserialize)]
pub struct MessageReactionRemoveEvent {
    pub user_id: String,
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub emoji: ReactionEmoji,
}

/// Emoji data in reactions
#[derive(Debug, Clone, Deserialize)]
pub struct ReactionEmoji {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub animated: bool,
}

/// VOICE_STATE_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceStateUpdateEvent {
    pub user_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub deaf: bool,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub self_deaf: bool,
    #[serde(default)]
    pub self_mute: bool,
    #[serde(default)]
    pub self_video: bool,
    #[serde(default)]
    pub suppress: bool,
    #[serde(default)]
    pub member: Option<Member>,
}

/// CHANNEL_CREATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelCreateEvent {
    #[serde(flatten)]
    pub channel: Channel,
}

/// CHANNEL_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelUpdateEvent {
    #[serde(flatten)]
    pub channel: Channel,
}

/// CHANNEL_DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelDeleteEvent {
    #[serde(flatten)]
    pub channel: Channel,
}

/// GUILD_CREATE event data - sent when joining a guild or when guild becomes
/// available
#[derive(Debug, Clone, Deserialize)]
pub struct GuildCreateEvent {
    #[serde(flatten)]
    pub guild: Guild,
}

/// GUILD_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildUpdateEvent {
    #[serde(flatten)]
    pub guild: Guild,
}

/// GUILD_DELETE event data - sent when leaving/kicked from guild or guild
/// becomes unavailable
#[derive(Debug, Clone, Deserialize)]
pub struct GuildDeleteEvent {
    pub id: String,
    #[serde(default)]
    pub unavailable: bool,
}

/// USER_UPDATE event data - sent when current user's properties change
#[derive(Debug, Clone, Deserialize)]
pub struct UserUpdateEvent {
    #[serde(flatten)]
    pub user: User,
}

/// INTERACTION_CREATE event data - sent when user interacts with application
/// commands, message components, or modal submissions.
///
/// Mirrors Discord's [Interaction](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object)
/// object. The [`Interaction`] type alias points at this struct so consumers
/// can refer to either name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionCreateEvent {
    pub id: String,
    pub application_id: String,
    #[serde(rename = "type")]
    pub interaction_type: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<InteractionData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub version: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_permissions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entitlements: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorizing_integration_owners: Option<std::collections::HashMap<String, String>>,
    /// Interaction context type (`InteractionContextType`):
    /// 0 = GUILD, 1 = BOT_DM, 2 = PRIVATE_CHANNEL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_size_limit: Option<u64>,
}

/// Type alias mirroring Discord's `Interaction` object terminology.
///
/// `InteractionCreateEvent` and `Interaction` are the same struct — the alias
/// exists so consumers can refer to the type by either name.
pub type Interaction = InteractionCreateEvent;

/// Inner `data` payload of an interaction.
///
/// Discord doesn't tag this field directly — its shape is dictated by the
/// parent interaction's `type`. We deserialize it via `#[serde(untagged)]`
/// which tries each variant in order and picks the first that matches.
/// `ApplicationCommandData` is tried first because it has the most
/// distinguishing required fields (`id` + `name` + `type`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InteractionData {
    /// `APPLICATION_COMMAND` (type 2) and autocomplete (type 4) payloads.
    ApplicationCommand(ApplicationCommandData),
    /// `MESSAGE_COMPONENT` (type 3) payloads — buttons, selects, etc.
    MessageComponent(MessageComponentData),
    /// `MODAL_SUBMIT` (type 5) payloads.
    ModalSubmit(ModalSubmitData),
}

/// Application-command interaction data (slash commands, user/message
/// commands, autocomplete).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandData {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub command_type: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<ResolvedData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

/// Message-component interaction data (buttons, select menus).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageComponentData {
    pub custom_id: String,
    pub component_type: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<ResolvedData>,
}

/// Modal-submit interaction data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalSubmitData {
    pub custom_id: String,
    pub components: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<ResolvedData>,
}

/// Resolved entities referenced by snowflake IDs in interaction options /
/// component values. Discord pre-fetches these so handlers don't need extra
/// REST calls. Members / roles / channels / attachments are kept as raw
/// JSON for now — promote them as proper types when callers need it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResolvedData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub users: Option<std::collections::HashMap<String, User>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<std::collections::HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<std::collections::HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<std::collections::HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<std::collections::HashMap<String, Message>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<std::collections::HashMap<String, Value>>,
}

/// MESSAGE_REACTION_REMOVE_ALL event data — all reactions removed from a
/// message
#[derive(Debug, Clone, Deserialize)]
pub struct MessageReactionRemoveAllEvent {
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
}

/// MESSAGE_REACTION_REMOVE_EMOJI event data — all reactions of a specific emoji
/// removed
#[derive(Debug, Clone, Deserialize)]
pub struct MessageReactionRemoveEmojiEvent {
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub emoji: ReactionEmoji,
}

/// MESSAGE_DELETE_BULK event data — multiple messages deleted at once
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDeleteBulkEvent {
    pub ids: Vec<String>,
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
}

/// GUILD_ROLE_CREATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildRoleCreateEvent {
    pub guild_id: String,
    pub role: Role,
}

/// GUILD_ROLE_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildRoleUpdateEvent {
    pub guild_id: String,
    pub role: Role,
}

/// GUILD_ROLE_DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildRoleDeleteEvent {
    pub guild_id: String,
    pub role_id: String,
}

/// GUILD_BAN_ADD event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildBanAddEvent {
    pub guild_id: String,
    pub user: User,
}

/// GUILD_BAN_REMOVE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildBanRemoveEvent {
    pub guild_id: String,
    pub user: User,
}

/// THREAD_CREATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadCreateEvent {
    #[serde(flatten)]
    pub channel: Channel,
}

/// THREAD_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadUpdateEvent {
    #[serde(flatten)]
    pub channel: Channel,
}

/// THREAD_DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadDeleteEvent {
    pub id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(rename = "type", default)]
    pub channel_type: u8,
}

/// THREAD_LIST_SYNC event data — sent when gaining access to a channel with
/// active threads
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadListSyncEvent {
    pub guild_id: String,
    #[serde(default)]
    pub channel_ids: Vec<String>,
    #[serde(default)]
    pub threads: Vec<Channel>,
    #[serde(default)]
    pub members: Vec<Value>,
}

/// THREAD_MEMBER_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadMemberUpdateEvent {
    pub id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub join_timestamp: Option<String>,
    #[serde(default)]
    pub flags: u64,
}

/// VOICE_SERVER_UPDATE event data — provides voice connection credentials
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceServerUpdateEvent {
    pub token: String,
    pub guild_id: String,
    /// Voice server WebSocket endpoint (may be null during voice region
    /// migration)
    pub endpoint: Option<String>,
}

/// WEBHOOK_UPDATE event data — fired when webhooks are created/modified/deleted
/// in a channel
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookUpdateEvent {
    pub guild_id: String,
    pub channel_id: String,
}

/// GUILD_AUDIT_LOG_ENTRY_CREATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildAuditLogEntryCreateEvent {
    pub guild_id: String,
    /// Action type (see Discord docs for numeric values)
    pub action_type: u32,
    pub id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub changes: Vec<AuditLogChange>,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// AUTO_MODERATION_RULE_CREATE / UPDATE / DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct AutoModerationRuleEvent {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub creator_id: String,
    pub event_type: u8,
    pub trigger_type: u8,
    pub enabled: bool,
    #[serde(default)]
    pub exempt_roles: Vec<String>,
    #[serde(default)]
    pub exempt_channels: Vec<String>,
}

/// AUTO_MODERATION_ACTION_EXECUTION event data
#[derive(Debug, Clone, Deserialize)]
pub struct AutoModerationActionExecutionEvent {
    pub guild_id: String,
    #[serde(default)]
    pub channel_id: Option<String>,
    pub user_id: String,
    pub rule_id: String,
    pub rule_trigger_type: u8,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub matched_keyword: Option<String>,
    #[serde(default)]
    pub matched_content: Option<String>,
}

/// STAGE_INSTANCE_CREATE / UPDATE / DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct StageInstanceEvent {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub topic: String,
    /// Privacy level: 1=PUBLIC, 2=GUILD_ONLY
    pub privacy_level: u8,
    #[serde(default)]
    pub discoverable_disabled: bool,
    #[serde(default)]
    pub guild_scheduled_event_id: Option<String>,
}

/// GUILD_SCHEDULED_EVENT_CREATE / UPDATE / DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildScheduledEventEvent {
    pub id: String,
    pub guild_id: String,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub creator_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub scheduled_start_time: String,
    #[serde(default)]
    pub scheduled_end_time: Option<String>,
    pub privacy_level: u8,
    pub status: u8,
    pub entity_type: u8,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub user_count: Option<u32>,
    #[serde(default)]
    pub image: Option<String>,
}

/// GUILD_SCHEDULED_EVENT_USER_ADD / USER_REMOVE event data
#[derive(Debug, Clone, Deserialize)]
pub struct GuildScheduledEventUserEvent {
    pub guild_scheduled_event_id: String,
    pub user_id: String,
    pub guild_id: String,
}

/// SOUNDBOARD_SOUND_CREATE / UPDATE / DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct SoundboardSoundEvent {
    pub sound_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub name: String,
    pub volume: f64,
    #[serde(default)]
    pub emoji_id: Option<String>,
    #[serde(default)]
    pub emoji_name: Option<String>,
    #[serde(default)]
    pub available: bool,
}

/// CHANNEL_PINS_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelPinsUpdateEvent {
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub last_pin_timestamp: Option<String>,
}

/// GUILD_MEMBERS_CHUNK event data — response to a `Request Guild Members`
/// gateway command. Members are streamed in chunks of up to 1000.
#[derive(Debug, Clone, Deserialize)]
pub struct GuildMembersChunkEvent {
    pub guild_id: String,
    #[serde(default)]
    pub members: Vec<Member>,
    pub chunk_index: u32,
    pub chunk_count: u32,
    #[serde(default)]
    pub not_found: Option<Vec<String>>,
    #[serde(default)]
    pub presences: Option<Vec<Value>>,
    #[serde(default)]
    pub nonce: Option<String>,
}

/// MESSAGE_POLL_VOTE_ADD event data — a user voted on a poll
#[derive(Debug, Clone, Deserialize)]
pub struct MessagePollVoteAddEvent {
    pub user_id: String,
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub answer_id: u32,
}

/// MESSAGE_POLL_VOTE_REMOVE event data — a user removed their poll vote
#[derive(Debug, Clone, Deserialize)]
pub struct MessagePollVoteRemoveEvent {
    pub user_id: String,
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub answer_id: u32,
}

/// ENTITLEMENT_CREATE / UPDATE / DELETE event data — premium subscription /
/// SKU entitlement gained or revoked.
#[derive(Debug, Clone, Deserialize)]
pub struct EntitlementEvent {
    pub id: String,
    pub sku_id: String,
    pub application_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(rename = "type")]
    pub type_: u8,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub starts_at: Option<String>,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub consumed: Option<bool>,
}

/// INTEGRATION_CREATE / UPDATE / DELETE event data
///
/// First-pass: schema is large and rarely-needed; preserve the raw payload
/// alongside the obvious IDs so consumers can drill in if needed.
#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationEvent {
    pub id: String,
    pub guild_id: String,
    /// Full integration payload (Discord's Integration object).
    #[serde(flatten)]
    pub extra: Value,
}

/// GUILD_SOUNDBOARD_SOUND_CREATE / _UPDATE / _DELETE event data
///
/// Singular soundboard event (a single sound was created/updated/deleted in
/// a guild). Distinct from the older [`SoundboardSoundEvent`].
#[derive(Debug, Clone, Deserialize)]
pub struct GuildSoundboardSoundEvent {
    pub guild_id: String,
    pub sound_id: String,
    /// Remaining fields (name, volume, emoji_id, emoji_name, available, etc).
    #[serde(flatten)]
    pub extra: Value,
}

/// GUILD_SOUNDBOARD_SOUNDS_UPDATE event data — multiple soundboard sounds
/// updated for a guild (plural).
#[derive(Debug, Clone, Deserialize)]
pub struct GuildSoundboardSoundsUpdateEvent {
    pub guild_id: String,
    #[serde(default)]
    pub soundboard_sounds: Vec<Value>,
}

/// SOUNDBOARD_SOUNDS event data — gateway response to opcode 31
/// (Request Soundboard Sounds). Distinct from the singular CRUD events above.
#[derive(Debug, Clone, Deserialize)]
pub struct SoundboardSoundsEvent {
    pub guild_id: String,
    #[serde(default)]
    pub soundboard_sounds: Vec<Value>,
}

/// THREAD_MEMBERS_UPDATE event data — multiple thread members joined or left
#[derive(Debug, Clone, Deserialize)]
pub struct ThreadMembersUpdateEvent {
    pub id: String,
    pub guild_id: String,
    pub member_count: u32,
    #[serde(default)]
    pub added_members: Option<Vec<Value>>,
    #[serde(default)]
    pub removed_member_ids: Option<Vec<String>>,
}

/// INVITE_CREATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct InviteCreateEvent {
    pub channel_id: String,
    pub code: String,
    pub created_at: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub inviter: Option<User>,
    pub max_age: u32,
    pub max_uses: u32,
    #[serde(default)]
    pub target_type: Option<u8>,
    #[serde(default)]
    pub target_user: Option<User>,
    #[serde(default)]
    pub target_application: Option<Value>,
    #[serde(default)]
    pub temporary: bool,
    #[serde(default)]
    pub uses: u32,
}

/// INVITE_DELETE event data
#[derive(Debug, Clone, Deserialize)]
pub struct InviteDeleteEvent {
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub code: String,
}

/// APPLICATION_COMMAND_PERMISSIONS_UPDATE event data
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationCommandPermissionsUpdateEvent {
    pub id: String,
    pub application_id: String,
    pub guild_id: String,
    #[serde(default)]
    pub permissions: Vec<Value>,
}

/// VOICE_CHANNEL_EFFECT_SEND event data — emoji reaction / soundboard burst
/// in a voice channel.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceChannelEffectSendEvent {
    pub channel_id: String,
    pub guild_id: String,
    pub user_id: String,
    #[serde(default)]
    pub emoji: Option<Value>,
    #[serde(default)]
    pub animation_type: Option<u8>,
    #[serde(default)]
    pub animation_id: Option<u32>,
    #[serde(default)]
    pub sound_id: Option<String>,
    #[serde(default)]
    pub sound_volume: Option<f64>,
}

/// GUILD_EMOJIS_UPDATE event data — guild's emoji list changed
#[derive(Debug, Clone, Deserialize)]
pub struct GuildEmojisUpdateEvent {
    pub guild_id: String,
    #[serde(default)]
    pub emojis: Vec<Value>,
}

/// GUILD_STICKERS_UPDATE event data — guild's sticker list changed
#[derive(Debug, Clone, Deserialize)]
pub struct GuildStickersUpdateEvent {
    pub guild_id: String,
    #[serde(default)]
    pub stickers: Vec<Value>,
}

/// GUILD_INTEGRATIONS_UPDATE event data — guild's integration list changed
/// (e.g. an integration was added/removed). The full integration list must
/// be re-fetched via REST.
#[derive(Debug, Clone, Deserialize)]
pub struct GuildIntegrationsUpdateEvent {
    pub guild_id: String,
}

/// CALL_CREATE event data — a private/group DM call started.
#[derive(Debug, Clone, Deserialize)]
pub struct CallCreateEvent {
    pub channel_id: String,
    pub message_id: String,
    pub region: String,
    #[serde(default)]
    pub ringing: Vec<String>,
    #[serde(default)]
    pub embedded_activities: Vec<Value>,
    #[serde(default)]
    pub voice_states: Vec<Value>,
}

/// CALL_UPDATE event data — call state changed (region/ringing list).
#[derive(Debug, Clone, Deserialize)]
pub struct CallUpdateEvent {
    pub channel_id: String,
    pub message_id: String,
    pub region: String,
    #[serde(default)]
    pub ringing: Vec<String>,
    #[serde(default)]
    pub guild_id: Option<String>,
}

/// CALL_DELETE event data — call ended.
#[derive(Debug, Clone, Deserialize)]
pub struct CallDeleteEvent {
    pub channel_id: String,
}

/// MESSAGE_ACK event — emitted when the user reads (acknowledges) a message,
/// either manually or implicitly. Drives unread/mention counters.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageAckEvent {
    pub channel_id: String,
    pub message_id: String,
    pub version: u64,
    #[serde(default)]
    pub manual: Option<bool>,
    #[serde(default)]
    pub mention_count: Option<u32>,
    #[serde(default)]
    pub last_viewed: Option<u32>,
    #[serde(default)]
    pub flags: Option<u32>,
    #[serde(default)]
    pub ack_type: Option<u32>,
}

/// CHANNEL_RECIPIENT_ADD / CHANNEL_RECIPIENT_REMOVE event — a user joined or
/// left a group DM. Same payload shape for both directions.
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelRecipientEvent {
    pub channel_id: String,
    pub user: crate::types::user::User,
}

/// NOTIFICATION_CENTER_ITEM_CREATE event — a new entry was added to the
/// in-app Notification Center.
#[derive(Debug, Clone, Deserialize)]
pub struct NotificationCenterItemCreateEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub body: String,
    pub deeplink: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub icon_name: Option<String>,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub acked: bool,
    #[serde(default)]
    pub disable_action: bool,
    #[serde(default)]
    pub is_voice_message: bool,
    #[serde(default)]
    pub bundle_id: Option<String>,
    #[serde(default)]
    pub other_user: Option<Value>,
    #[serde(default)]
    pub application: Option<Value>,
    #[serde(default)]
    pub callout: Option<Value>,
    #[serde(default)]
    pub item_enum: Option<Value>,
    #[serde(default)]
    pub message: Option<Value>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub message_channel_id: Option<String>,
    #[serde(default)]
    pub message_content: Option<String>,
    #[serde(default)]
    pub message_embed_count: Option<u32>,
    #[serde(default)]
    pub message_attachment_count: Option<u32>,
    #[serde(default)]
    pub message_sticker_count: Option<u32>,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub guild_scheduled_event_id: Option<String>,
}

/// REACTION_NOTIFICATION_SENT event — push-notification audit for a reaction.
#[derive(Debug, Clone, Deserialize)]
pub struct ReactionNotificationSentEvent {
    pub route: String,
    pub reactor_user_id: String,
    pub message: Value,
    pub emoji: Value,
}

/// GENERIC_PUSH_NOTIFICATION_SENT event — generic push-notification audit.
#[derive(Debug, Clone, Deserialize)]
pub struct GenericPushNotificationSentEvent {
    #[serde(rename = "type")]
    pub notif_kind: String,
    pub tracking_type: String,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    pub route: String,
    pub deeplink: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub channel_type: Option<u8>,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub message_type: Option<u8>,
    #[serde(default)]
    pub notif_type_id: Option<String>,
    pub user_id: String,
    pub user_username: String,
    #[serde(default)]
    pub user_discriminator: Option<u16>,
    #[serde(default)]
    pub user_avatar: Option<String>,
}

/// USER_APPLICATION_IDENTITY_UPDATE event — user-installed app identity
/// (per-app username/avatar override) was edited.
#[derive(Debug, Clone, Deserialize)]
pub struct UserApplicationIdentityUpdateEvent {
    pub application_id: String,
    pub user_id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub avatar_hash: Option<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// USER_REQUIRED_ACTION_UPDATE event — current user must take an action
/// (e.g. verify email). `required_action` may be null to clear.
#[derive(Debug, Clone, Deserialize)]
pub struct UserRequiredActionUpdateEvent {
    #[serde(default)]
    pub required_action: Option<String>,
}

/// DM_SETTINGS_UPSELL_SHOW event — Nitro / DM upsell prompt signal.
#[derive(Debug, Clone, Deserialize)]
pub struct DmSettingsUpsellShowEvent {
    pub guild_id: String,
    #[serde(flatten)]
    pub extra: Value,
}

/// GUILD_APPLICATION_COMMAND_INDEX_UPDATE event — slash-command index
/// refresh signal for a guild.
#[derive(Debug, Clone, Deserialize)]
pub struct GuildApplicationCommandIndexUpdateEvent {
    pub guild_id: String,
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub bot_users: Vec<String>,
    #[serde(default)]
    pub application_command_counts: std::collections::HashMap<String, i64>,
    #[serde(flatten)]
    pub extra: Value,
}

/// GUILD_APPLIED_BOOSTS_UPDATE event — server boost application changed.
/// Payload is a single applied-boost record.
#[derive(Debug, Clone, Deserialize)]
pub struct GuildAppliedBoostsUpdateEvent {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub pause_ends_at: Option<String>,
    #[serde(default)]
    pub ended: bool,
    #[serde(flatten)]
    pub extra: Value,
}

/// SESSIONS_REPLACE event data — replaces the full list of sessions for the
/// current user across all connected clients (other devices/browsers signed
/// into the same account). Dispatched whenever that set changes.
///
/// The gateway sends `d` as a bare JSON array; this newtype wraps it so it
/// can be deserialized as a typed event payload.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct SessionsReplaceEvent {
    /// Full replacement list of active sessions for the current user.
    pub sessions: Vec<crate::types::Session>,
}

/// CONTENT_INVENTORY_INBOX_STALE event data — signals the user's
/// content-inventory inbox cache is stale and should be refetched via REST.
/// Payload is intentionally minimal; Discord may send it empty or with a
/// `last_viewed` marker (days since Unix epoch).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContentInventoryInboxStaleEvent {
    #[serde(default)]
    pub last_viewed: Option<i64>,
}

/// READY_SUPPLEMENTAL event data — sent immediately after READY with bulk
/// state. Sub-shapes are large and evolve frequently, so inner fields are
/// kept as raw `Value`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReadySupplementalEvent {
    /// Merged presence data: typically `{ friends: [...], guilds: [[...], ...] }`.
    #[serde(default)]
    pub merged_presences: Option<Value>,
    /// Merged member arrays — one inner array per guild (parallel to `guilds`).
    #[serde(default)]
    pub merged_members: Vec<Value>,
    /// Per-guild supplemental data (voice_states, embedded_activities, threads, ...).
    #[serde(default)]
    pub guilds: Vec<Value>,
    /// Disclosed feature flags / opt-in disclosures.
    #[serde(default)]
    pub disclose: Vec<Value>,
    /// Lazy-loaded private channel hints.
    #[serde(default)]
    pub lazy_private_channels: Vec<Value>,
    /// Optional outstanding game invites.
    #[serde(default)]
    pub game_invites: Option<Value>,
    /// Optional top-level voice state list.
    #[serde(default)]
    pub voice_states: Option<Value>,
    /// Forward-compatible catchall.
    #[serde(flatten)]
    pub extra: Value,
}

/// USER_SETTINGS_PROTO_UPDATE event data — sent when the user's settings
/// proto is updated. The `proto` payload is a base64-encoded protobuf whose
/// schema depends on `settings.type`.
#[derive(Debug, Clone, Deserialize)]
pub struct UserSettingsProtoUpdateEvent {
    pub settings: UserSettingsProtoData,
    #[serde(default)]
    pub partial: bool,
}

/// Inner `settings` object for [`UserSettingsProtoUpdateEvent`].
///
/// `settings_type`: 1 = PreloadedUserSettings, 2 = FrecencyAndFavoritesSettings,
/// 3 = TestSettings.
#[derive(Debug, Clone, Deserialize)]
pub struct UserSettingsProtoData {
    #[serde(rename = "type")]
    pub settings_type: u8,
    /// Base64-encoded protobuf bytes.
    pub proto: String,
}

/// CONVERSATION_SUMMARY_UPDATE event data — Discord's auto-generated
/// conversation summaries for active text channels were updated.
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationSummaryUpdateEvent {
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub summaries: Vec<ConversationSummary>,
}

/// One conversation summary inside [`ConversationSummaryUpdateEvent`].
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub summ_short: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub message_ids: Vec<String>,
    #[serde(default)]
    pub people: Vec<String>,
    #[serde(rename = "unsafe", default)]
    pub unsafe_: Option<bool>,
    #[serde(default)]
    pub start_id: Option<String>,
    #[serde(default)]
    pub end_id: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(rename = "type", default)]
    pub summary_type: Option<u8>,
    #[serde(default)]
    pub source: Option<u8>,
}

/// EMBEDDED_ACTIVITY_UPDATE_V2 event data — published when an embedded
/// (voice-channel) activity instance changes state. Inner `state` is left
/// as raw JSON because the schema is large and evolves frequently.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddedActivityUpdateV2Event {
    pub guild_id: String,
    pub channel_id: String,
    #[serde(default)]
    pub users: Vec<String>,
    pub embedded_activity: EmbeddedActivity,
}

/// Inner `embedded_activity` payload of [`EmbeddedActivityUpdateV2Event`].
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddedActivity {
    pub application_id: String,
    pub instance_id: String,
    #[serde(default)]
    pub launch_id: Option<String>,
    #[serde(default)]
    pub user_ids: Vec<String>,
    /// Opaque/evolving inner state (users with current_room_id, rooms,
    /// premium_tier_requirement, etc.).
    #[serde(default)]
    pub state: Value,
}

/// CHANNEL_INFO event data — sent in response to opcode 43 (Request Channel
/// Info). Carries ephemeral per-channel data (voice status, voice start time)
/// for channels in a guild.
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelInfoEvent {
    pub guild_id: String,
    #[serde(default)]
    pub channels: Vec<ChannelInfoChannel>,
}

/// One channel's ephemeral data inside [`ChannelInfoEvent`].
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelInfoChannel {
    pub id: String,
    /// Voice channel status string (nullable, optional).
    #[serde(default)]
    pub status: Option<String>,
    /// Unix timestamp (in seconds) of when the voice session started
    /// (nullable, optional).
    #[serde(default)]
    pub voice_start_time: Option<i64>,
}

/// VOICE_CHANNEL_STATUS_UPDATE event — voice channel status string changed.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceChannelStatusUpdateEvent {
    pub id: String,
    pub guild_id: String,
    #[serde(default)]
    pub status: Option<String>,
}

/// VOICE_CHANNEL_START_TIME_UPDATE event — voice session start timestamp
/// changed for a voice channel.
#[derive(Debug, Clone, Deserialize)]
pub struct VoiceChannelStartTimeUpdateEvent {
    pub id: String,
    pub guild_id: String,
    #[serde(default)]
    pub voice_start_time: Option<i64>,
}

/// SUBSCRIPTION_CREATE / UPDATE / DELETE event data
///
/// First-pass: full subscription schema kept in `extra` for drill-down.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionEvent {
    pub id: String,
    pub user_id: String,
    #[serde(default)]
    pub sku_ids: Vec<String>,
    /// Remaining fields (status, period_start, period_end, etc).
    #[serde(flatten)]
    pub extra: Value,
}

// ==================== Typed Event Enum ====================

/// Strongly-typed Discord gateway events
#[derive(Debug, Clone)]
pub enum TypedEvent {
    /// Gateway connected and ready
    Ready(ReadyEvent),
    /// Session successfully resumed (after RESUME opcode acknowledged)
    Resumed(ResumedEvent),
    /// A message was created
    MessageCreate(Box<MessageCreateEvent>),
    /// A message was updated
    MessageUpdate(MessageUpdateEvent),
    /// A message was deleted
    MessageDelete(MessageDeleteEvent),
    /// User started typing
    TypingStart(TypingStartEvent),
    /// Relationship added (friend request, etc)
    RelationshipAdd(RelationshipAddEvent),
    /// Relationship removed
    RelationshipRemove(RelationshipRemoveEvent),
    /// User presence changed
    PresenceUpdate(PresenceUpdateEvent),
    /// Member joined a guild
    GuildMemberAdd(GuildMemberAddEvent),
    /// Member left a guild
    GuildMemberRemove(GuildMemberRemoveEvent),
    /// Member updated in a guild
    GuildMemberUpdate(GuildMemberUpdateEvent),
    /// Reaction added to a message
    MessageReactionAdd(MessageReactionAddEvent),
    /// Reaction removed from a message
    MessageReactionRemove(MessageReactionRemoveEvent),
    /// All reactions removed from a message
    MessageReactionRemoveAll(MessageReactionRemoveAllEvent),
    /// All reactions of a specific emoji removed from a message
    MessageReactionRemoveEmoji(MessageReactionRemoveEmojiEvent),
    /// Multiple messages deleted at once
    MessageDeleteBulk(MessageDeleteBulkEvent),
    /// Voice state changed
    VoiceStateUpdate(VoiceStateUpdateEvent),
    /// Channel created
    ChannelCreate(ChannelCreateEvent),
    /// Channel updated
    ChannelUpdate(ChannelUpdateEvent),
    /// Channel deleted
    ChannelDelete(ChannelDeleteEvent),
    /// Channel pins updated
    ChannelPinsUpdate(ChannelPinsUpdateEvent),
    /// Guild joined or became available
    GuildCreate(GuildCreateEvent),
    /// Guild updated
    GuildUpdate(GuildUpdateEvent),
    /// Guild left or became unavailable
    GuildDelete(GuildDeleteEvent),
    /// Guild role created
    GuildRoleCreate(GuildRoleCreateEvent),
    /// Guild role updated
    GuildRoleUpdate(GuildRoleUpdateEvent),
    /// Guild role deleted
    GuildRoleDelete(GuildRoleDeleteEvent),
    /// User banned from guild
    GuildBanAdd(GuildBanAddEvent),
    /// User unbanned from guild
    GuildBanRemove(GuildBanRemoveEvent),
    /// Current user updated
    UserUpdate(UserUpdateEvent),
    /// Interaction received
    InteractionCreate(Box<InteractionCreateEvent>),
    /// Thread created
    ThreadCreate(ThreadCreateEvent),
    /// Thread updated
    ThreadUpdate(ThreadUpdateEvent),
    /// Thread deleted
    ThreadDelete(ThreadDeleteEvent),
    /// Thread list synced
    ThreadListSync(ThreadListSyncEvent),
    /// Thread member updated
    ThreadMemberUpdate(ThreadMemberUpdateEvent),
    /// Voice server update (provides voice connection credentials)
    VoiceServerUpdate(VoiceServerUpdateEvent),
    /// Webhooks created/modified/deleted in a channel
    WebhookUpdate(WebhookUpdateEvent),
    /// Audit log entry created
    GuildAuditLogEntryCreate(GuildAuditLogEntryCreateEvent),
    /// Auto-moderation rule created
    AutoModerationRuleCreate(AutoModerationRuleEvent),
    /// Auto-moderation rule updated
    AutoModerationRuleUpdate(AutoModerationRuleEvent),
    /// Auto-moderation rule deleted
    AutoModerationRuleDelete(AutoModerationRuleEvent),
    /// Auto-moderation action executed
    AutoModerationActionExecution(AutoModerationActionExecutionEvent),
    /// Stage instance created
    StageInstanceCreate(StageInstanceEvent),
    /// Stage instance updated
    StageInstanceUpdate(StageInstanceEvent),
    /// Stage instance deleted
    StageInstanceDelete(StageInstanceEvent),
    /// Guild scheduled event created
    GuildScheduledEventCreate(GuildScheduledEventEvent),
    /// Guild scheduled event updated
    GuildScheduledEventUpdate(GuildScheduledEventEvent),
    /// Guild scheduled event deleted
    GuildScheduledEventDelete(GuildScheduledEventEvent),
    /// User subscribed to guild scheduled event
    GuildScheduledEventUserAdd(GuildScheduledEventUserEvent),
    /// User unsubscribed from guild scheduled event
    GuildScheduledEventUserRemove(GuildScheduledEventUserEvent),
    /// Soundboard sound created
    SoundboardSoundCreate(SoundboardSoundEvent),
    /// Soundboard sound updated
    SoundboardSoundUpdate(SoundboardSoundEvent),
    /// Soundboard sound deleted
    SoundboardSoundDelete(SoundboardSoundEvent),
    /// Chunk of guild members in response to a Request Guild Members op
    GuildMembersChunk(Box<GuildMembersChunkEvent>),
    /// Poll vote added
    MessagePollVoteAdd(MessagePollVoteAddEvent),
    /// Poll vote removed
    MessagePollVoteRemove(MessagePollVoteRemoveEvent),
    /// Entitlement created (premium / SKU access granted)
    EntitlementCreate(EntitlementEvent),
    /// Entitlement updated
    EntitlementUpdate(EntitlementEvent),
    /// Entitlement deleted (premium / SKU access revoked)
    EntitlementDelete(EntitlementEvent),
    /// Integration created
    IntegrationCreate(IntegrationEvent),
    /// Integration updated
    IntegrationUpdate(IntegrationEvent),
    /// Integration deleted
    IntegrationDelete(IntegrationEvent),
    /// Guild soundboard sound created (singular CRUD; distinct from
    /// [`TypedEvent::SoundboardSoundCreate`])
    GuildSoundboardSoundCreate(GuildSoundboardSoundEvent),
    /// Guild soundboard sound updated
    GuildSoundboardSoundUpdate(GuildSoundboardSoundEvent),
    /// Guild soundboard sound deleted
    GuildSoundboardSoundDelete(GuildSoundboardSoundEvent),
    /// Multiple soundboard sounds updated for a guild (plural)
    GuildSoundboardSoundsUpdate(GuildSoundboardSoundsUpdateEvent),
    /// Gateway response to a Request Soundboard Sounds (op 31)
    SoundboardSounds(SoundboardSoundsEvent),
    /// Multiple thread members joined or left
    ThreadMembersUpdate(ThreadMembersUpdateEvent),
    /// Invite created
    InviteCreate(InviteCreateEvent),
    /// Invite deleted
    InviteDelete(InviteDeleteEvent),
    /// Application command permissions updated
    ApplicationCommandPermissionsUpdate(ApplicationCommandPermissionsUpdateEvent),
    /// Voice channel effect (emoji reaction / soundboard burst) sent
    VoiceChannelEffectSend(VoiceChannelEffectSendEvent),
    /// Guild's emoji list changed
    GuildEmojisUpdate(GuildEmojisUpdateEvent),
    /// Guild's sticker list changed
    GuildStickersUpdate(GuildStickersUpdateEvent),
    /// Guild's integration list changed
    GuildIntegrationsUpdate(GuildIntegrationsUpdateEvent),
    /// Subscription created
    SubscriptionCreate(SubscriptionEvent),
    /// Subscription updated
    SubscriptionUpdate(SubscriptionEvent),
    /// Subscription deleted
    SubscriptionDelete(SubscriptionEvent),
    /// Channel info — response to opcode 43 (Request Channel Info)
    ChannelInfo(ChannelInfoEvent),
    /// Voice channel status string changed
    VoiceChannelStatusUpdate(VoiceChannelStatusUpdateEvent),
    /// Voice channel start time changed
    VoiceChannelStartTimeUpdate(VoiceChannelStartTimeUpdateEvent),
    /// Full replacement list of the current user's active sessions
    SessionsReplace(SessionsReplaceEvent),
    /// Content-inventory inbox cache was invalidated
    ContentInventoryInboxStale(ContentInventoryInboxStaleEvent),
    /// Bulk lazy state delivered immediately after READY
    ReadySupplemental(Box<ReadySupplementalEvent>),
    /// User's protobuf-encoded settings were updated
    UserSettingsProtoUpdate(UserSettingsProtoUpdateEvent),
    /// Auto-generated conversation summaries were updated for a channel
    ConversationSummaryUpdate(ConversationSummaryUpdateEvent),
    /// Embedded voice-channel activity instance state changed (v2 schema)
    EmbeddedActivityUpdateV2(EmbeddedActivityUpdateV2Event),
    /// DM voice/video call started
    CallCreate(CallCreateEvent),
    /// DM voice/video call state changed
    CallUpdate(CallUpdateEvent),
    /// DM voice/video call ended
    CallDelete(CallDeleteEvent),
    /// User read (acknowledged) a message
    MessageAck(MessageAckEvent),
    /// User joined a group DM
    ChannelRecipientAdd(ChannelRecipientEvent),
    /// User left a group DM
    ChannelRecipientRemove(ChannelRecipientEvent),
    /// New entry added to the in-app Notification Center
    NotificationCenterItemCreate(Box<NotificationCenterItemCreateEvent>),
    /// Push notification dispatched for a reaction
    ReactionNotificationSent(ReactionNotificationSentEvent),
    /// Generic push notification audit
    GenericPushNotificationSent(GenericPushNotificationSentEvent),
    /// Friend / block edge state changed (reuses RelationshipAddEvent shape)
    RelationshipUpdate(RelationshipAddEvent),
    /// User-installed app identity (per-app username/avatar) was edited
    UserApplicationIdentityUpdate(UserApplicationIdentityUpdateEvent),
    /// Account requires action (reverify / etc.)
    UserRequiredActionUpdate(UserRequiredActionUpdateEvent),
    /// Nitro / DM upsell prompt
    DmSettingsUpsellShow(DmSettingsUpsellShowEvent),
    /// Slash-command index refresh signal for a guild
    GuildApplicationCommandIndexUpdate(GuildApplicationCommandIndexUpdateEvent),
    /// Server boost application changed
    GuildAppliedBoostsUpdate(GuildAppliedBoostsUpdateEvent),
    /// Unknown/unhandled event with raw data
    Unknown { event_type: String, data: Value },
}

impl TypedEvent {
    /// Parse a raw dispatch event into a typed event
    pub fn from_raw(event_type: &str, data: Value) -> Self {
        macro_rules! parse {
            ($variant:ident) => {
                serde_json::from_value(data.clone()).map(TypedEvent::$variant).unwrap_or_else(|_| TypedEvent::Unknown { event_type: event_type.to_string(), data })
            };
        }
        match event_type {
            "READY" => parse!(Ready),
            "RESUMED" => TypedEvent::Resumed(ResumedEvent {}),
            "MESSAGE_CREATE" => serde_json::from_value(data.clone()).map(|e| TypedEvent::MessageCreate(Box::new(e))).unwrap_or_else(|_| TypedEvent::Unknown { event_type: event_type.to_string(), data }),
            "MESSAGE_UPDATE" => parse!(MessageUpdate),
            "MESSAGE_DELETE" => parse!(MessageDelete),
            "MESSAGE_DELETE_BULK" => parse!(MessageDeleteBulk),
            "TYPING_START" => parse!(TypingStart),
            "RELATIONSHIP_ADD" => parse!(RelationshipAdd),
            "RELATIONSHIP_REMOVE" => parse!(RelationshipRemove),
            "PRESENCE_UPDATE" => parse!(PresenceUpdate),
            "GUILD_MEMBER_ADD" => parse!(GuildMemberAdd),
            "GUILD_MEMBER_REMOVE" => parse!(GuildMemberRemove),
            "GUILD_MEMBER_UPDATE" => parse!(GuildMemberUpdate),
            "MESSAGE_REACTION_ADD" => parse!(MessageReactionAdd),
            "MESSAGE_REACTION_REMOVE" => parse!(MessageReactionRemove),
            "MESSAGE_REACTION_REMOVE_ALL" => parse!(MessageReactionRemoveAll),
            "MESSAGE_REACTION_REMOVE_EMOJI" => parse!(MessageReactionRemoveEmoji),
            "VOICE_STATE_UPDATE" => parse!(VoiceStateUpdate),
            "CHANNEL_CREATE" => parse!(ChannelCreate),
            "CHANNEL_UPDATE" => parse!(ChannelUpdate),
            "CHANNEL_DELETE" => parse!(ChannelDelete),
            "CHANNEL_PINS_UPDATE" => parse!(ChannelPinsUpdate),
            "GUILD_CREATE" => parse!(GuildCreate),
            "GUILD_UPDATE" => parse!(GuildUpdate),
            "GUILD_DELETE" => parse!(GuildDelete),
            "GUILD_ROLE_CREATE" => parse!(GuildRoleCreate),
            "GUILD_ROLE_UPDATE" => parse!(GuildRoleUpdate),
            "GUILD_ROLE_DELETE" => parse!(GuildRoleDelete),
            "GUILD_BAN_ADD" => parse!(GuildBanAdd),
            "GUILD_BAN_REMOVE" => parse!(GuildBanRemove),
            "USER_UPDATE" => parse!(UserUpdate),
            "INTERACTION_CREATE" => serde_json::from_value(data.clone()).map(|e| TypedEvent::InteractionCreate(Box::new(e))).unwrap_or_else(|_| TypedEvent::Unknown { event_type: event_type.to_string(), data }),
            "THREAD_CREATE" => parse!(ThreadCreate),
            "THREAD_UPDATE" => parse!(ThreadUpdate),
            "THREAD_DELETE" => parse!(ThreadDelete),
            "THREAD_LIST_SYNC" => parse!(ThreadListSync),
            "THREAD_MEMBER_UPDATE" => parse!(ThreadMemberUpdate),
            "VOICE_SERVER_UPDATE" => parse!(VoiceServerUpdate),
            "WEBHOOKS_UPDATE" => parse!(WebhookUpdate),
            "GUILD_AUDIT_LOG_ENTRY_CREATE" => parse!(GuildAuditLogEntryCreate),
            "AUTO_MODERATION_RULE_CREATE" => parse!(AutoModerationRuleCreate),
            "AUTO_MODERATION_RULE_UPDATE" => parse!(AutoModerationRuleUpdate),
            "AUTO_MODERATION_RULE_DELETE" => parse!(AutoModerationRuleDelete),
            "AUTO_MODERATION_ACTION_EXECUTION" => parse!(AutoModerationActionExecution),
            "STAGE_INSTANCE_CREATE" => parse!(StageInstanceCreate),
            "STAGE_INSTANCE_UPDATE" => parse!(StageInstanceUpdate),
            "STAGE_INSTANCE_DELETE" => parse!(StageInstanceDelete),
            "GUILD_SCHEDULED_EVENT_CREATE" => parse!(GuildScheduledEventCreate),
            "GUILD_SCHEDULED_EVENT_UPDATE" => parse!(GuildScheduledEventUpdate),
            "GUILD_SCHEDULED_EVENT_DELETE" => parse!(GuildScheduledEventDelete),
            "GUILD_SCHEDULED_EVENT_USER_ADD" => parse!(GuildScheduledEventUserAdd),
            "GUILD_SCHEDULED_EVENT_USER_REMOVE" => parse!(GuildScheduledEventUserRemove),
            "SOUNDBOARD_SOUND_CREATE" => parse!(SoundboardSoundCreate),
            "SOUNDBOARD_SOUND_UPDATE" => parse!(SoundboardSoundUpdate),
            "SOUNDBOARD_SOUND_DELETE" => parse!(SoundboardSoundDelete),
            "GUILD_MEMBERS_CHUNK" => serde_json::from_value(data.clone()).map(|e| TypedEvent::GuildMembersChunk(Box::new(e))).unwrap_or_else(|_| TypedEvent::Unknown { event_type: event_type.to_string(), data }),
            "MESSAGE_POLL_VOTE_ADD" => parse!(MessagePollVoteAdd),
            "MESSAGE_POLL_VOTE_REMOVE" => parse!(MessagePollVoteRemove),
            "ENTITLEMENT_CREATE" => parse!(EntitlementCreate),
            "ENTITLEMENT_UPDATE" => parse!(EntitlementUpdate),
            "ENTITLEMENT_DELETE" => parse!(EntitlementDelete),
            "INTEGRATION_CREATE" => parse!(IntegrationCreate),
            "INTEGRATION_UPDATE" => parse!(IntegrationUpdate),
            "INTEGRATION_DELETE" => parse!(IntegrationDelete),
            "GUILD_SOUNDBOARD_SOUND_CREATE" => parse!(GuildSoundboardSoundCreate),
            "GUILD_SOUNDBOARD_SOUND_UPDATE" => parse!(GuildSoundboardSoundUpdate),
            "GUILD_SOUNDBOARD_SOUND_DELETE" => parse!(GuildSoundboardSoundDelete),
            "GUILD_SOUNDBOARD_SOUNDS_UPDATE" => parse!(GuildSoundboardSoundsUpdate),
            "SOUNDBOARD_SOUNDS" => parse!(SoundboardSounds),
            "THREAD_MEMBERS_UPDATE" => parse!(ThreadMembersUpdate),
            "INVITE_CREATE" => parse!(InviteCreate),
            "INVITE_DELETE" => parse!(InviteDelete),
            "APPLICATION_COMMAND_PERMISSIONS_UPDATE" => parse!(ApplicationCommandPermissionsUpdate),
            "VOICE_CHANNEL_EFFECT_SEND" => parse!(VoiceChannelEffectSend),
            "GUILD_EMOJIS_UPDATE" => parse!(GuildEmojisUpdate),
            "GUILD_STICKERS_UPDATE" => parse!(GuildStickersUpdate),
            "GUILD_INTEGRATIONS_UPDATE" => parse!(GuildIntegrationsUpdate),
            "CHANNEL_INFO" => parse!(ChannelInfo),
            "VOICE_CHANNEL_STATUS_UPDATE" => parse!(VoiceChannelStatusUpdate),
            "VOICE_CHANNEL_START_TIME_UPDATE" => parse!(VoiceChannelStartTimeUpdate),
            "SESSIONS_REPLACE" => parse!(SessionsReplace),
            "CONTENT_INVENTORY_INBOX_STALE" => parse!(ContentInventoryInboxStale),
            "READY_SUPPLEMENTAL" => serde_json::from_value(data.clone()).map(|e| TypedEvent::ReadySupplemental(Box::new(e))).unwrap_or_else(|_| TypedEvent::Unknown { event_type: event_type.to_string(), data }),
            "USER_SETTINGS_PROTO_UPDATE" => parse!(UserSettingsProtoUpdate),
            "CONVERSATION_SUMMARY_UPDATE" => parse!(ConversationSummaryUpdate),
            "EMBEDDED_ACTIVITY_UPDATE_V2" => parse!(EmbeddedActivityUpdateV2),
            "CALL_CREATE" => parse!(CallCreate),
            "CALL_UPDATE" => parse!(CallUpdate),
            "CALL_DELETE" => parse!(CallDelete),
            "MESSAGE_ACK" => parse!(MessageAck),
            "CHANNEL_RECIPIENT_ADD" => parse!(ChannelRecipientAdd),
            "CHANNEL_RECIPIENT_REMOVE" => parse!(ChannelRecipientRemove),
            "NOTIFICATION_CENTER_ITEM_CREATE" => serde_json::from_value(data.clone()).map(|e| TypedEvent::NotificationCenterItemCreate(Box::new(e))).unwrap_or_else(|_| TypedEvent::Unknown { event_type: event_type.to_string(), data }),
            "REACTION_NOTIFICATION_SENT" => parse!(ReactionNotificationSent),
            "GENERIC_PUSH_NOTIFICATION_SENT" => parse!(GenericPushNotificationSent),
            "RELATIONSHIP_UPDATE" => parse!(RelationshipUpdate),
            "USER_APPLICATION_IDENTITY_UPDATE" => parse!(UserApplicationIdentityUpdate),
            "USER_REQUIRED_ACTION_UPDATE" => parse!(UserRequiredActionUpdate),
            "DM_SETTINGS_UPSELL_SHOW" => parse!(DmSettingsUpsellShow),
            "GUILD_APPLICATION_COMMAND_INDEX_UPDATE" => parse!(GuildApplicationCommandIndexUpdate),
            "GUILD_APPLIED_BOOSTS_UPDATE" => parse!(GuildAppliedBoostsUpdate),
            "SUBSCRIPTION_CREATE" => parse!(SubscriptionCreate),
            "SUBSCRIPTION_UPDATE" => parse!(SubscriptionUpdate),
            "SUBSCRIPTION_DELETE" => parse!(SubscriptionDelete),
            _ => TypedEvent::Unknown { event_type: event_type.to_string(), data },
        }
    }

    /// Get the event type name
    pub fn event_type(&self) -> &str {
        match self {
            TypedEvent::Ready(_) => "READY",
            TypedEvent::Resumed(_) => "RESUMED",
            TypedEvent::MessageCreate(_) => "MESSAGE_CREATE",
            TypedEvent::MessageUpdate(_) => "MESSAGE_UPDATE",
            TypedEvent::MessageDelete(_) => "MESSAGE_DELETE",
            TypedEvent::MessageDeleteBulk(_) => "MESSAGE_DELETE_BULK",
            TypedEvent::TypingStart(_) => "TYPING_START",
            TypedEvent::RelationshipAdd(_) => "RELATIONSHIP_ADD",
            TypedEvent::RelationshipRemove(_) => "RELATIONSHIP_REMOVE",
            TypedEvent::PresenceUpdate(_) => "PRESENCE_UPDATE",
            TypedEvent::GuildMemberAdd(_) => "GUILD_MEMBER_ADD",
            TypedEvent::GuildMemberRemove(_) => "GUILD_MEMBER_REMOVE",
            TypedEvent::GuildMemberUpdate(_) => "GUILD_MEMBER_UPDATE",
            TypedEvent::MessageReactionAdd(_) => "MESSAGE_REACTION_ADD",
            TypedEvent::MessageReactionRemove(_) => "MESSAGE_REACTION_REMOVE",
            TypedEvent::MessageReactionRemoveAll(_) => "MESSAGE_REACTION_REMOVE_ALL",
            TypedEvent::MessageReactionRemoveEmoji(_) => "MESSAGE_REACTION_REMOVE_EMOJI",
            TypedEvent::VoiceStateUpdate(_) => "VOICE_STATE_UPDATE",
            TypedEvent::ChannelCreate(_) => "CHANNEL_CREATE",
            TypedEvent::ChannelUpdate(_) => "CHANNEL_UPDATE",
            TypedEvent::ChannelDelete(_) => "CHANNEL_DELETE",
            TypedEvent::ChannelPinsUpdate(_) => "CHANNEL_PINS_UPDATE",
            TypedEvent::GuildCreate(_) => "GUILD_CREATE",
            TypedEvent::GuildUpdate(_) => "GUILD_UPDATE",
            TypedEvent::GuildDelete(_) => "GUILD_DELETE",
            TypedEvent::GuildRoleCreate(_) => "GUILD_ROLE_CREATE",
            TypedEvent::GuildRoleUpdate(_) => "GUILD_ROLE_UPDATE",
            TypedEvent::GuildRoleDelete(_) => "GUILD_ROLE_DELETE",
            TypedEvent::GuildBanAdd(_) => "GUILD_BAN_ADD",
            TypedEvent::GuildBanRemove(_) => "GUILD_BAN_REMOVE",
            TypedEvent::UserUpdate(_) => "USER_UPDATE",
            TypedEvent::InteractionCreate(_) => "INTERACTION_CREATE",
            TypedEvent::ThreadCreate(_) => "THREAD_CREATE",
            TypedEvent::ThreadUpdate(_) => "THREAD_UPDATE",
            TypedEvent::ThreadDelete(_) => "THREAD_DELETE",
            TypedEvent::ThreadListSync(_) => "THREAD_LIST_SYNC",
            TypedEvent::ThreadMemberUpdate(_) => "THREAD_MEMBER_UPDATE",
            TypedEvent::VoiceServerUpdate(_) => "VOICE_SERVER_UPDATE",
            TypedEvent::WebhookUpdate(_) => "WEBHOOKS_UPDATE",
            TypedEvent::GuildAuditLogEntryCreate(_) => "GUILD_AUDIT_LOG_ENTRY_CREATE",
            TypedEvent::AutoModerationRuleCreate(_) => "AUTO_MODERATION_RULE_CREATE",
            TypedEvent::AutoModerationRuleUpdate(_) => "AUTO_MODERATION_RULE_UPDATE",
            TypedEvent::AutoModerationRuleDelete(_) => "AUTO_MODERATION_RULE_DELETE",
            TypedEvent::AutoModerationActionExecution(_) => "AUTO_MODERATION_ACTION_EXECUTION",
            TypedEvent::StageInstanceCreate(_) => "STAGE_INSTANCE_CREATE",
            TypedEvent::StageInstanceUpdate(_) => "STAGE_INSTANCE_UPDATE",
            TypedEvent::StageInstanceDelete(_) => "STAGE_INSTANCE_DELETE",
            TypedEvent::GuildScheduledEventCreate(_) => "GUILD_SCHEDULED_EVENT_CREATE",
            TypedEvent::GuildScheduledEventUpdate(_) => "GUILD_SCHEDULED_EVENT_UPDATE",
            TypedEvent::GuildScheduledEventDelete(_) => "GUILD_SCHEDULED_EVENT_DELETE",
            TypedEvent::GuildScheduledEventUserAdd(_) => "GUILD_SCHEDULED_EVENT_USER_ADD",
            TypedEvent::GuildScheduledEventUserRemove(_) => "GUILD_SCHEDULED_EVENT_USER_REMOVE",
            TypedEvent::SoundboardSoundCreate(_) => "SOUNDBOARD_SOUND_CREATE",
            TypedEvent::SoundboardSoundUpdate(_) => "SOUNDBOARD_SOUND_UPDATE",
            TypedEvent::SoundboardSoundDelete(_) => "SOUNDBOARD_SOUND_DELETE",
            TypedEvent::GuildMembersChunk(_) => "GUILD_MEMBERS_CHUNK",
            TypedEvent::MessagePollVoteAdd(_) => "MESSAGE_POLL_VOTE_ADD",
            TypedEvent::MessagePollVoteRemove(_) => "MESSAGE_POLL_VOTE_REMOVE",
            TypedEvent::EntitlementCreate(_) => "ENTITLEMENT_CREATE",
            TypedEvent::EntitlementUpdate(_) => "ENTITLEMENT_UPDATE",
            TypedEvent::EntitlementDelete(_) => "ENTITLEMENT_DELETE",
            TypedEvent::IntegrationCreate(_) => "INTEGRATION_CREATE",
            TypedEvent::IntegrationUpdate(_) => "INTEGRATION_UPDATE",
            TypedEvent::IntegrationDelete(_) => "INTEGRATION_DELETE",
            TypedEvent::GuildSoundboardSoundCreate(_) => "GUILD_SOUNDBOARD_SOUND_CREATE",
            TypedEvent::GuildSoundboardSoundUpdate(_) => "GUILD_SOUNDBOARD_SOUND_UPDATE",
            TypedEvent::GuildSoundboardSoundDelete(_) => "GUILD_SOUNDBOARD_SOUND_DELETE",
            TypedEvent::GuildSoundboardSoundsUpdate(_) => "GUILD_SOUNDBOARD_SOUNDS_UPDATE",
            TypedEvent::SoundboardSounds(_) => "SOUNDBOARD_SOUNDS",
            TypedEvent::ThreadMembersUpdate(_) => "THREAD_MEMBERS_UPDATE",
            TypedEvent::InviteCreate(_) => "INVITE_CREATE",
            TypedEvent::InviteDelete(_) => "INVITE_DELETE",
            TypedEvent::ApplicationCommandPermissionsUpdate(_) => "APPLICATION_COMMAND_PERMISSIONS_UPDATE",
            TypedEvent::VoiceChannelEffectSend(_) => "VOICE_CHANNEL_EFFECT_SEND",
            TypedEvent::GuildEmojisUpdate(_) => "GUILD_EMOJIS_UPDATE",
            TypedEvent::GuildStickersUpdate(_) => "GUILD_STICKERS_UPDATE",
            TypedEvent::GuildIntegrationsUpdate(_) => "GUILD_INTEGRATIONS_UPDATE",
            TypedEvent::SubscriptionCreate(_) => "SUBSCRIPTION_CREATE",
            TypedEvent::SubscriptionUpdate(_) => "SUBSCRIPTION_UPDATE",
            TypedEvent::SubscriptionDelete(_) => "SUBSCRIPTION_DELETE",
            TypedEvent::ChannelInfo(_) => "CHANNEL_INFO",
            TypedEvent::VoiceChannelStatusUpdate(_) => "VOICE_CHANNEL_STATUS_UPDATE",
            TypedEvent::VoiceChannelStartTimeUpdate(_) => "VOICE_CHANNEL_START_TIME_UPDATE",
            TypedEvent::SessionsReplace(_) => "SESSIONS_REPLACE",
            TypedEvent::ContentInventoryInboxStale(_) => "CONTENT_INVENTORY_INBOX_STALE",
            TypedEvent::ReadySupplemental(_) => "READY_SUPPLEMENTAL",
            TypedEvent::UserSettingsProtoUpdate(_) => "USER_SETTINGS_PROTO_UPDATE",
            TypedEvent::ConversationSummaryUpdate(_) => "CONVERSATION_SUMMARY_UPDATE",
            TypedEvent::EmbeddedActivityUpdateV2(_) => "EMBEDDED_ACTIVITY_UPDATE_V2",
            TypedEvent::CallCreate(_) => "CALL_CREATE",
            TypedEvent::CallUpdate(_) => "CALL_UPDATE",
            TypedEvent::CallDelete(_) => "CALL_DELETE",
            TypedEvent::MessageAck(_) => "MESSAGE_ACK",
            TypedEvent::ChannelRecipientAdd(_) => "CHANNEL_RECIPIENT_ADD",
            TypedEvent::ChannelRecipientRemove(_) => "CHANNEL_RECIPIENT_REMOVE",
            TypedEvent::NotificationCenterItemCreate(_) => "NOTIFICATION_CENTER_ITEM_CREATE",
            TypedEvent::ReactionNotificationSent(_) => "REACTION_NOTIFICATION_SENT",
            TypedEvent::GenericPushNotificationSent(_) => "GENERIC_PUSH_NOTIFICATION_SENT",
            TypedEvent::RelationshipUpdate(_) => "RELATIONSHIP_UPDATE",
            TypedEvent::UserApplicationIdentityUpdate(_) => "USER_APPLICATION_IDENTITY_UPDATE",
            TypedEvent::UserRequiredActionUpdate(_) => "USER_REQUIRED_ACTION_UPDATE",
            TypedEvent::DmSettingsUpsellShow(_) => "DM_SETTINGS_UPSELL_SHOW",
            TypedEvent::GuildApplicationCommandIndexUpdate(_) => "GUILD_APPLICATION_COMMAND_INDEX_UPDATE",
            TypedEvent::GuildAppliedBoostsUpdate(_) => "GUILD_APPLIED_BOOSTS_UPDATE",
            TypedEvent::Unknown { event_type, .. } => event_type,
        }
    }
}
