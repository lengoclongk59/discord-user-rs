//! Discord message types

use serde::{Deserialize, Serialize};

use super::{ChannelId, GuildId, Member, MessageFlags, MessageId, MessageType, User, UserId};

/// Discord message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: Option<String>,
    pub author: User,
    #[serde(default)]
    pub member: Option<Member>,
    pub content: String,
    pub timestamp: String,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub tts: bool,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub mention_roles: Vec<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    #[serde(default)]
    pub nonce: Option<serde_json::Value>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub webhook_id: Option<String>,
    #[serde(default, rename = "type")]
    pub message_type: MessageType,
    #[serde(default)]
    pub flags: MessageFlags,
    #[serde(default)]
    pub referenced_message: Option<Box<Message>>,
    #[serde(default)]
    pub components: Vec<serde_json::Value>,
    /// Attached poll, if this message contains one.
    #[serde(default)]
    pub poll: Option<Poll>,
    /// Channels specifically mentioned in this message (only crossposted
    /// messages currently populate this).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention_channels: Option<Vec<ChannelMention>>,
    /// Rich Presence-related activity attached to the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<MessageActivity>,
    /// Application object for Rich Presence-related messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<serde_json::Value>,
    /// If the message is an Interaction or application-owned webhook, this is
    /// the id of the application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
    /// Metadata about the interaction that produced this message (preferred
    /// over the deprecated `interaction` field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_metadata: Option<serde_json::Value>,
    /// Snapshot of the thread that was started from this message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<serde_json::Value>,
    /// Sticker items sent with the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker_items: Option<Vec<StickerItem>>,
    /// **Deprecated** full sticker objects (still returned in some payloads).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stickers: Option<Vec<serde_json::Value>>,
    /// Approximate position of this message within a thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// Data attached to ROLE_SUBSCRIPTION_PURCHASE messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_subscription_data: Option<RoleSubscriptionData>,
    /// Resolved data for users/members/channels/roles referenced in this
    /// message (interaction context).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<serde_json::Value>,
    /// Information about a call started in this message (DM/Group DM).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<MessageCall>,
    /// Forwarded message snapshots (immutable copies of the source).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_snapshots: Option<Vec<MessageSnapshot>>,
}

impl Message {
    /// Typed accessor for the message's channel ID.
    pub fn channel_id_typed(&self) -> Option<ChannelId> {
        self.channel_id.parse().ok().map(ChannelId::new)
    }

    /// Typed accessor for the message author's user ID.
    pub fn author_id(&self) -> Option<UserId> {
        self.author.id.parse().ok().map(UserId::new)
    }

    /// Typed accessor for the guild ID (None for DMs).
    pub fn guild_id_typed(&self) -> Option<GuildId> {
        self.guild_id.as_deref()?.parse().ok().map(GuildId::new)
    }

    /// Typed accessor for the message's own ID.
    pub fn message_id(&self) -> Option<MessageId> {
        self.id.parse().ok().map(MessageId::new)
    }

    /// Create a `MessageReference` pointing to this message, suitable for use
    /// as a reply in [`SendMessageRequest`].
    ///
    /// Returns `None` if the channel ID cannot be parsed.
    pub fn as_reference(&self) -> MessageReference {
        MessageReference {
            reference_type: None,
            message_id: Some(self.id.clone()),
            channel_id: Some(self.channel_id.clone()),
            guild_id: self.guild_id.clone(),
        }
    }

    /// Create a [`SendMessageRequest`]-compatible reply builder pre-filled with
    /// a message reference pointing to this message.
    pub fn reply_builder<'a>(&self, content: &'a str) -> crate::types::SendMessageRequest<'a> {
        crate::types::SendMessageRequest {
            content,
            tts: false,
            flags: 0,
            message_reference: Some(self.as_reference()),
            nonce: None,
            mobile_network_type: None,
        }
    }

    /// Whether the message is pinned in its channel.
    ///
    /// Mirrors serenity's `Message::pinned`.
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }

    /// Whether this is a text-to-speech message.
    pub fn is_tts(&self) -> bool {
        self.tts
    }

    /// Whether the message is a system message (join notification, boost, pin,
    /// etc.) rather than a user-authored message.
    ///
    /// Mirrors serenity's `Message::is_system()`.
    pub fn is_system(&self) -> bool {
        !matches!(self.message_type, MessageType::Default | MessageType::Reply | MessageType::ChatInputCommand | MessageType::ContextMenuCommand)
    }

    /// Whether this is an ephemeral message visible only to the target user.
    ///
    /// Mirrors serenity's `Message::flags` check for `EPHEMERAL`.
    pub fn is_ephemeral(&self) -> bool {
        self.flags.contains(MessageFlags::EPHEMERAL)
    }

    /// Whether the message @mentions @everyone or @here.
    pub fn mentions_everyone(&self) -> bool {
        self.mention_everyone
    }

    /// Whether the message content mentions a specific user by their ID.
    ///
    /// Checks the `mentions` array (populated by Discord when content contains
    /// `<@user_id>`).  Mirrors serenity's `Message::mentions_user()`.
    pub fn mentions_user_id(&self, user_id: &str) -> bool {
        self.mentions.iter().any(|u| u.id == user_id)
    }

    /// Whether this message is a reply to another message.
    pub fn is_reply(&self) -> bool {
        matches!(self.message_type, MessageType::Reply) || self.referenced_message.is_some()
    }

    /// Whether the message has been crossposted to follower channels.
    pub fn is_crosspost(&self) -> bool {
        self.flags.contains(MessageFlags::IS_CROSSPOST)
    }
}

/// A textual channel mentioned in a message (only present for crossposted
/// messages; see Discord docs for `mention_channels`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMention {
    pub id: String,
    pub guild_id: String,
    /// Discord channel type integer.
    #[serde(rename = "type")]
    pub type_: u8,
    pub name: String,
}

/// Rich Presence-style activity attached to a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageActivity {
    /// Type of message activity (1=JOIN, 2=SPECTATE, 3=LISTEN, 5=JOIN_REQUEST).
    #[serde(rename = "type")]
    pub activity_type: u8,
    /// `party_id` from a Rich Presence event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_id: Option<String>,
}

/// Minimal sticker reference attached to a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerItem {
    pub id: String,
    pub name: String,
    /// Sticker format type (1=PNG, 2=APNG, 3=LOTTIE, 4=GIF).
    pub format_type: u8,
}

/// Data attached to a `ROLE_SUBSCRIPTION_PURCHASE` system message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleSubscriptionData {
    pub role_subscription_listing_id: String,
    pub tier_name: String,
    pub total_months_subscribed: u32,
    pub is_renewal: bool,
}

/// Information about a call started in a DM or Group DM message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCall {
    /// User IDs of those who participated in the call.
    #[serde(default)]
    pub participants: Vec<String>,
    /// ISO 8601 timestamp when the call ended, if it has ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_timestamp: Option<String>,
}

/// A Discord poll attached to a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    /// The question text.
    pub question: PollMedia,
    /// The available answers.
    pub answers: Vec<PollAnswer>,
    /// ISO 8601 expiry time.
    pub expiry: Option<String>,
    /// Whether this is a multi-select poll.
    #[serde(default)]
    pub allow_multiselect: bool,
    /// Layout type: 1 = DEFAULT.
    #[serde(default = "default_layout")]
    pub layout_type: u8,
    /// Results (present when poll has ended or `with_results=true`).
    #[serde(default)]
    pub results: Option<PollResults>,
}

fn default_layout() -> u8 {
    1
}

/// Text/emoji content used for poll questions and answers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PollMedia {
    /// Display text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Emoji — either `{"id": "snowflake"}` for custom or `{"name": "🎉"}` for
    /// Unicode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<serde_json::Value>,
}

/// A single answer option in a poll.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollAnswer {
    /// Answer index (1-based, assigned by Discord).
    pub answer_id: u64,
    /// Display content.
    pub poll_media: PollMedia,
}

/// Vote counts after a poll closes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollResults {
    /// Whether the results are finalized.
    #[serde(default)]
    pub is_finalized: bool,
    /// Per-answer vote counts.
    pub answer_counts: Vec<PollAnswerCount>,
}

/// Vote count for a single poll answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollAnswerCount {
    pub id: u64,
    pub count: u64,
    /// Whether the current user voted for this answer.
    #[serde(default)]
    pub me_voted: bool,
}

/// Message attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    pub size: u64,
    pub url: String,
    pub proxy_url: String,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub ephemeral: bool,
    /// Display title for the attachment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Duration of the audio file in seconds (voice messages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    /// Base64-encoded sampled waveform preview (voice messages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform: Option<String>,
    /// Attachment flags bitfield.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

impl Attachment {
    /// Decode the raw `flags` bitfield into typed [`AttachmentFlags`], if present.
    pub fn flag_bits(&self) -> Option<crate::types::enums::AttachmentFlags> {
        self.flags.and_then(|raw| crate::types::enums::AttachmentFlags::from_bits(raw))
    }
}

/// Message embed
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Embed {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, rename = "type")]
    pub embed_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub color: Option<u32>,
    #[serde(default)]
    pub footer: Option<EmbedFooter>,
    #[serde(default)]
    pub image: Option<EmbedMedia>,
    #[serde(default)]
    pub thumbnail: Option<EmbedMedia>,
    #[serde(default)]
    pub video: Option<EmbedMedia>,
    #[serde(default)]
    pub provider: Option<EmbedProvider>,
    #[serde(default)]
    pub author: Option<EmbedAuthor>,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
    /// Embed flags bitfield (1<<5 = IS_CONTENT_INVENTORY_ENTRY).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbedFooter {
    pub text: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub proxy_icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbedMedia {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub width: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbedProvider {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbedAuthor {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub proxy_icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

/// Message reaction
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Reaction {
    pub count: u32,
    #[serde(default)]
    pub me: bool,
    pub emoji: Emoji,
    /// Breakdown of normal vs super (burst) reaction counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_details: Option<ReactionCountDetails>,
    /// Whether the current user super-reacted using this emoji.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub me_burst: Option<bool>,
    /// Hex color codes used for super-reaction burst animation (e.g. "#FF0000").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst_colors: Option<Vec<String>>,
}

/// Breakdown of reaction counts split between normal and super (burst) reactions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReactionCountDetails {
    /// Number of super (burst) reactions.
    pub burst: u64,
    /// Number of normal reactions.
    pub normal: u64,
}

/// Discord emoji
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Emoji {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub animated: bool,
}

/// Message reference for replies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReference {
    /// Reference type: 0 = DEFAULT (reply), 1 = FORWARD.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
}

/// Request payload for sending a message
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest<'a> {
    pub content: &'a str,
    pub tts: bool,
    pub flags: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<&'a str>,
    /// Mobile-client telemetry field. **Must be `None` for real bot tokens**
    /// — Discord's bot endpoint rejects POST /channels/{id}/messages with
    /// HTTP 403 `code 40333 "internal network error"` when this field is
    /// present (it's a user-account-only fingerprint and bot endpoints
    /// validate strictly). For self-bot user tokens you may pass
    /// `Some("unknown")`, `Some("wifi")` etc. to mimic the Discord mobile
    /// client; the `send_message` helper does not set it, so use the
    /// builder if you specifically need it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile_network_type: Option<&'a str>,
}

impl<'a> Default for SendMessageRequest<'a> {
    fn default() -> Self {
        Self { content: "", tts: false, flags: 0, message_reference: None, nonce: None, mobile_network_type: None }
    }
}

/// Request payload for editing a message
#[derive(Debug, Clone, Serialize, Default)]
pub struct EditMessageRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

/// A single pinned message entry from `GET /channels/{id}/messages/pins`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePin {
    /// ISO 8601 timestamp recording when this message was pinned.
    pub pinned_at: String,
    /// The pinned message itself.
    pub message: Message,
}

/// Paginated response wrapper for `GET /channels/{id}/messages/pins`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePinsResponse {
    /// Pin entries for the current page, newest pin first.
    pub items: Vec<MessagePin>,
    /// Whether additional pages of pinned messages remain.
    pub has_more: bool,
}

/// Forwarded-message snapshot (immutable copy of the source message).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSnapshot {
    /// The snapshotted (frozen) message contents.
    pub message: MessageSnapshotInner,
}

/// Inner payload of a [`MessageSnapshot`] — a subset of the [`Message`] fields
/// that Discord captures at forward time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSnapshotInner {
    /// Numeric Discord message type at the moment of the snapshot.
    #[serde(rename = "type")]
    pub message_type: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Vec<crate::types::user::User>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mention_roles: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stickers: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker_items: Option<Vec<StickerItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<serde_json::Value>>,
}
