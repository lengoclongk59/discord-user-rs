//! Message component builders — buttons, select menus, and action rows.
//!
//! Discord message components allow interactive UI elements inside messages.
//! They are arranged in [`CreateActionRow`] containers, each holding either
//! a row of [`CreateButton`]s or a single [`CreateSelectMenu`].
//!
//! Pass the assembled rows to [`MessageBuilder::components`].
//!
//! # Example
//! ```ignore
//! use discord_user::components::{CreateActionRow, CreateButton, ButtonStyle};
//!
//! let row = CreateActionRow::buttons(vec![
//!     CreateButton::new("confirm", ButtonStyle::Success).label("Confirm"),
//!     CreateButton::new("cancel",  ButtonStyle::Danger).label("Cancel"),
//! ]);
//! user.message()
//!     .channel(channel_id)
//!     .content("Are you sure?")
//!     .components(vec![row])
//!     .send()
//!     .await?;
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_repr::{Deserialize_repr, Serialize_repr};

// ── Button style ─────────────────────────────────────────────────────────────

/// Visual style of a [`CreateButton`].
///
/// Mirrors serenity's `ButtonStyle` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ButtonStyle {
    /// Blurple — a standard action button (requires `custom_id`).
    Primary = 1,
    /// Grey — a secondary action button (requires `custom_id`).
    Secondary = 2,
    /// Green — a positive/confirm button (requires `custom_id`).
    Success = 3,
    /// Red — a danger/destructive button (requires `custom_id`).
    Danger = 4,
    /// Grey link button that navigates to a URL (requires `url`, no
    /// `custom_id`).
    Link = 5,
}

// ── Button ───────────────────────────────────────────────────────────────────

/// A clickable button component.
///
/// Mirrors serenity's `CreateButton`.
///
/// # Example
/// ```ignore
/// use discord_user::components::{CreateButton, ButtonStyle};
/// let btn = CreateButton::new("my_btn", ButtonStyle::Primary)
///     .label("Click me")
///     .disabled(false);
/// ```
#[derive(Debug, Clone)]
pub struct CreateButton {
    style: ButtonStyle,
    custom_id: Option<String>,
    url: Option<String>,
    label: Option<String>,
    emoji: Option<Value>,
    disabled: bool,
}

impl CreateButton {
    /// Create a non-link button with the given `custom_id` and style.
    pub fn new(custom_id: impl Into<String>, style: ButtonStyle) -> Self {
        Self { style, custom_id: Some(custom_id.into()), url: None, label: None, emoji: None, disabled: false }
    }

    /// Create a link button that navigates to `url`.
    pub fn link(url: impl Into<String>) -> Self {
        Self { style: ButtonStyle::Link, custom_id: None, url: Some(url.into()), label: None, emoji: None, disabled: false }
    }

    /// Set the visible label text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a Unicode or partial custom emoji.
    ///
    /// `emoji` should be a JSON object matching Discord's emoji partial:
    /// `{"id": null, "name": "👍"}` or `{"id": "123", "name": "upvote",
    /// "animated": false}`.
    pub fn emoji(mut self, emoji: Value) -> Self {
        self.emoji = Some(emoji);
        self
    }

    /// Whether the button appears greyed-out and cannot be clicked.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Serialize to a Discord component JSON object.
    pub fn to_json(&self) -> Value {
        let mut obj = json!({
            "type": 2,
            "style": self.style as u8,
            "disabled": self.disabled,
        });
        if let Some(ref id) = self.custom_id {
            obj["custom_id"] = json!(id);
        }
        if let Some(ref url) = self.url {
            obj["url"] = json!(url);
        }
        if let Some(ref label) = self.label {
            obj["label"] = json!(label);
        }
        if let Some(ref emoji) = self.emoji {
            obj["emoji"] = emoji.clone();
        }
        obj
    }
}

// ── Select menu option
// ────────────────────────────────────────────────────────

/// A single option within a [`CreateSelectMenu`].
///
/// Mirrors serenity's `CreateSelectMenuOption`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateSelectMenuOption {
    /// Displayed text.
    pub label: String,
    /// Value sent to the application when this option is chosen.
    pub value: String,
    /// Optional description shown below the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional partial emoji shown next to the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Value>,
    /// If true, this option is pre-selected as the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl CreateSelectMenuOption {
    /// Create an option with the required label and value fields.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into(), description: None, emoji: None, default: None }
    }

    /// Add a description line.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark this option as the default selection.
    pub fn default_selection(mut self, is_default: bool) -> Self {
        self.default = Some(is_default);
        self
    }
}

// ── Select menu
// ───────────────────────────────────────────────────────────────

/// A dropdown select menu component.
///
/// Mirrors serenity's `CreateSelectMenu`.
///
/// # Example
/// ```ignore
/// use discord_user::components::{CreateSelectMenu, CreateSelectMenuOption};
/// let menu = CreateSelectMenu::new("pick_color")
///     .placeholder("Choose a color")
///     .add_option(CreateSelectMenuOption::new("Red", "red"))
///     .add_option(CreateSelectMenuOption::new("Blue", "blue"));
/// ```
#[derive(Debug, Clone)]
pub struct CreateSelectMenu {
    custom_id: String,
    placeholder: Option<String>,
    min_values: Option<u8>,
    max_values: Option<u8>,
    disabled: bool,
    options: Vec<CreateSelectMenuOption>,
}

impl CreateSelectMenu {
    /// Create a new string-select menu with the given `custom_id`.
    pub fn new(custom_id: impl Into<String>) -> Self {
        Self {
            custom_id: custom_id.into(),
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: false,
            options: Vec::new(),
        }
    }

    /// Set the placeholder text shown when nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Minimum number of values the user must select.
    pub fn min_values(mut self, min: u8) -> Self {
        self.min_values = Some(min);
        self
    }

    /// Maximum number of values the user can select (up to 25).
    pub fn max_values(mut self, max: u8) -> Self {
        self.max_values = Some(max);
        self
    }

    /// Whether the menu is disabled (greyed out, unclickable).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add an option to the menu.
    pub fn add_option(mut self, option: CreateSelectMenuOption) -> Self {
        self.options.push(option);
        self
    }

    /// Replace all options at once.
    pub fn options(mut self, options: Vec<CreateSelectMenuOption>) -> Self {
        self.options = options;
        self
    }

    /// Serialize to a Discord component JSON object.
    pub fn to_json(&self) -> Value {
        let mut obj = json!({
            "type": 3,            // STRING_SELECT
            "custom_id": self.custom_id,
            "options": self.options,
            "disabled": self.disabled,
        });
        if let Some(ref ph) = self.placeholder {
            obj["placeholder"] = json!(ph);
        }
        if let Some(min) = self.min_values {
            obj["min_values"] = json!(min);
        }
        if let Some(max) = self.max_values {
            obj["max_values"] = json!(max);
        }
        obj
    }
}

// ── Action row
// ────────────────────────────────────────────────────────────────

/// A container for up to 5 buttons or 1 select menu.
///
/// Mirrors serenity's `CreateActionRow`.
#[derive(Debug, Clone)]
pub enum CreateActionRow {
    /// A row containing 1–5 buttons.
    Buttons(Vec<CreateButton>),
    /// A row containing a single select menu.
    SelectMenu(CreateSelectMenu),
}

impl CreateActionRow {
    /// Convenience constructor for a button row.
    pub fn buttons(buttons: Vec<CreateButton>) -> Self {
        Self::Buttons(buttons)
    }

    /// Convenience constructor for a select-menu row.
    pub fn select_menu(menu: CreateSelectMenu) -> Self {
        Self::SelectMenu(menu)
    }

    /// Serialize to a Discord action-row component JSON object.
    pub fn to_json(&self) -> Value {
        match self {
            Self::Buttons(buttons) => json!({
                "type": 1,
                "components": buttons.iter().map(|b| b.to_json()).collect::<Vec<_>>(),
            }),
            Self::SelectMenu(menu) => json!({
                "type": 1,
                "components": [menu.to_json()],
            }),
        }
    }
}

// ── Component type tag ───────────────────────────────────────────────────────

/// Numeric component type tag, mirroring Discord's `component.type` field.
///
/// Covers classic v1 components (action row, button, string select, text
/// input), the auto-population select menu variants (user/role/mentionable/
/// channel select), and the Components V2 layout primitives (section, text
/// display, thumbnail, media gallery, file, separator, container).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ComponentType {
    /// Container for other components.
    ActionRow = 1,
    /// Clickable button.
    Button = 2,
    /// Select menu of pre-defined string options.
    StringSelect = 3,
    /// Multi-line text input inside a modal.
    TextInput = 4,
    /// Auto-populated select of users.
    UserSelect = 5,
    /// Auto-populated select of roles.
    RoleSelect = 6,
    /// Auto-populated select of users **or** roles.
    MentionableSelect = 7,
    /// Auto-populated select of channels.
    ChannelSelect = 8,
    /// V2: section combining text components with an optional accessory.
    Section = 9,
    /// V2: standalone block of markdown text.
    TextDisplay = 10,
    /// V2: small image accessory used inside a section.
    Thumbnail = 11,
    /// V2: 1–10 image gallery.
    MediaGallery = 12,
    /// V2: attached file reference.
    File = 13,
    /// V2: visual separator with optional divider line and spacing.
    Separator = 14,
    /// V2: grouping container with optional accent color and spoiler.
    Container = 17,
}

// ── Default value for auto-populated selects ─────────────────────────────────

/// A pre-selected entry on an auto-populating select menu.
///
/// `type_` must be one of `"user"`, `"role"`, or `"channel"`.  The field is
/// serialized as `"type"` on the wire — `type` is a reserved keyword in Rust.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefaultValue {
    /// Snowflake ID of the user, role, or channel.
    pub id: String,
    /// Discriminator for the snowflake — `"user"`, `"role"`, or `"channel"`.
    #[serde(rename = "type")]
    pub type_: String,
}

impl DefaultValue {
    /// Construct a default user reference.
    pub fn user(id: impl Into<String>) -> Self {
        Self { id: id.into(), type_: "user".into() }
    }

    /// Construct a default role reference.
    pub fn role(id: impl Into<String>) -> Self {
        Self { id: id.into(), type_: "role".into() }
    }

    /// Construct a default channel reference.
    pub fn channel(id: impl Into<String>) -> Self {
        Self { id: id.into(), type_: "channel".into() }
    }
}

// ── Components V2 helpers ────────────────────────────────────────────────────

/// Reference to a media asset, used by Components V2 layout primitives.
///
/// May be either a regular CDN URL or an `attachment://filename.ext`
/// reference resolved against the message's attachments list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnfurledMediaItem {
    /// HTTPS URL (or `attachment://…` reference).
    pub url: String,
}

impl UnfurledMediaItem {
    /// Convenience constructor.
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

/// A single tile inside a [`Component::MediaGallery`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaGalleryItem {
    /// The image/video to display.
    pub media: UnfurledMediaItem,
    /// Optional alt text (max 1024 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the tile should be blurred behind a spoiler veil.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl MediaGalleryItem {
    /// Construct a gallery tile from a media URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self { media: UnfurledMediaItem::new(url), description: None, spoiler: None }
    }

    /// Set alt text.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark as spoiler.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }
}

// ── Type-tagged Component enum ───────────────────────────────────────────────

/// Tagged-union representation of a Discord message component.
///
/// Serializes/deserializes using Discord's **integer** `type` discriminator
/// (see [`ComponentType`]).  This is the canonical type for **Components V2**
/// payloads (sections, text displays, galleries, containers, …) and the
/// auto-populated select menu variants introduced alongside it.
///
/// The classic v1 builders ([`CreateButton`], [`CreateSelectMenu`],
/// [`CreateActionRow`]) remain available — they emit the same wire format and
/// can be mixed freely inside [`Component::ActionRow`] payloads.
///
/// Custom `Serialize` / `Deserialize` impls handle the `type: <u8>` tag, since
/// serde's `#[serde(tag = "...")]` only supports string discriminators.
#[derive(Debug, Clone, PartialEq)]
pub enum Component {
    /// Layout container for child interactive components.
    ActionRow {
        /// Child components (buttons or a single select menu).
        components: Vec<Component>,
        /// Optional 32-bit identifier echoed back in submissions.
        id: Option<u32>,
    },

    /// Auto-populated select menu of users.
    UserSelect {
        /// Developer identifier echoed in interactions.
        custom_id: String,
        /// Placeholder text shown when nothing is selected.
        placeholder: Option<String>,
        /// Pre-selected default values (must all be type `"user"`).
        default_values: Option<Vec<DefaultValue>>,
        /// Minimum number of selections (default 1, range 0–25).
        min_values: Option<u8>,
        /// Maximum number of selections (default 1, range 1–25).
        max_values: Option<u8>,
        /// Whether the menu is greyed-out / unclickable.
        disabled: Option<bool>,
    },

    /// Auto-populated select menu of roles.
    RoleSelect {
        custom_id: String,
        placeholder: Option<String>,
        default_values: Option<Vec<DefaultValue>>,
        min_values: Option<u8>,
        max_values: Option<u8>,
        disabled: Option<bool>,
    },

    /// Auto-populated select menu of users **and** roles.
    MentionableSelect {
        custom_id: String,
        placeholder: Option<String>,
        default_values: Option<Vec<DefaultValue>>,
        min_values: Option<u8>,
        max_values: Option<u8>,
        disabled: Option<bool>,
    },

    /// Auto-populated select menu of channels.
    ChannelSelect {
        custom_id: String,
        placeholder: Option<String>,
        /// Restrict the dropdown to specific channel types (`0` = text,
        /// `2` = voice, etc.).
        channel_types: Option<Vec<u8>>,
        default_values: Option<Vec<DefaultValue>>,
        min_values: Option<u8>,
        max_values: Option<u8>,
        disabled: Option<bool>,
    },

    /// V2 — Section combining text components with an optional accessory.
    Section {
        /// Up to 3 [`Component::TextDisplay`] children.
        components: Vec<Component>,
        /// Optional accessory rendered to the side (button or thumbnail).
        accessory: Option<Box<Component>>,
        id: Option<u32>,
    },

    /// V2 — Standalone block of markdown text.
    TextDisplay {
        /// Markdown body (max 4000 chars).
        content: String,
        id: Option<u32>,
    },

    /// V2 — Image accessory used inside a [`Component::Section`].
    Thumbnail {
        media: UnfurledMediaItem,
        description: Option<String>,
        spoiler: Option<bool>,
        id: Option<u32>,
    },

    /// V2 — Gallery of 1–10 images / videos.
    MediaGallery {
        items: Vec<MediaGalleryItem>,
        id: Option<u32>,
    },

    /// V2 — Attached file reference.
    File {
        /// `attachment://filename.ext` reference into the message uploads.
        file: UnfurledMediaItem,
        spoiler: Option<bool>,
        id: Option<u32>,
    },

    /// V2 — Visual separator (optional divider line + spacing).
    Separator {
        /// If true, render a horizontal divider line.
        divider: Option<bool>,
        /// `1` = small padding, `2` = large padding.
        spacing: Option<u8>,
        id: Option<u32>,
    },

    /// V2 — Grouping container with optional accent color and spoiler.
    Container {
        components: Vec<Component>,
        /// RGB integer (`0xRRGGBB`) for the side accent bar.
        accent_color: Option<u32>,
        spoiler: Option<bool>,
        id: Option<u32>,
    },
}

// Custom (de)serialization for the integer `type` tag.
impl Serialize for Component {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Build a JSON object, then forward to the serializer.  This keeps the
        // implementation small at the cost of one intermediate allocation per
        // component — fine for the sizes Discord allows.
        let mut obj = serde_json::Map::new();
        let insert_opt = |obj: &mut serde_json::Map<String, Value>, k: &str, v: Option<Value>| {
            if let Some(v) = v {
                obj.insert(k.to_string(), v);
            }
        };
        match self {
            Component::ActionRow { components, id } => {
                obj.insert("type".into(), json!(ComponentType::ActionRow as u8));
                obj.insert("components".into(), serde_json::to_value(components).map_err(serde::ser::Error::custom)?);
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::UserSelect { custom_id, placeholder, default_values, min_values, max_values, disabled } => {
                obj.insert("type".into(), json!(ComponentType::UserSelect as u8));
                obj.insert("custom_id".into(), json!(custom_id));
                insert_opt(&mut obj, "placeholder", placeholder.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "default_values", default_values.as_ref().map(|x| serde_json::to_value(x).unwrap()));
                insert_opt(&mut obj, "min_values", min_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "max_values", max_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "disabled", disabled.as_ref().map(|x| json!(x)));
            }
            Component::RoleSelect { custom_id, placeholder, default_values, min_values, max_values, disabled } => {
                obj.insert("type".into(), json!(ComponentType::RoleSelect as u8));
                obj.insert("custom_id".into(), json!(custom_id));
                insert_opt(&mut obj, "placeholder", placeholder.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "default_values", default_values.as_ref().map(|x| serde_json::to_value(x).unwrap()));
                insert_opt(&mut obj, "min_values", min_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "max_values", max_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "disabled", disabled.as_ref().map(|x| json!(x)));
            }
            Component::MentionableSelect { custom_id, placeholder, default_values, min_values, max_values, disabled } => {
                obj.insert("type".into(), json!(ComponentType::MentionableSelect as u8));
                obj.insert("custom_id".into(), json!(custom_id));
                insert_opt(&mut obj, "placeholder", placeholder.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "default_values", default_values.as_ref().map(|x| serde_json::to_value(x).unwrap()));
                insert_opt(&mut obj, "min_values", min_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "max_values", max_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "disabled", disabled.as_ref().map(|x| json!(x)));
            }
            Component::ChannelSelect { custom_id, placeholder, channel_types, default_values, min_values, max_values, disabled } => {
                obj.insert("type".into(), json!(ComponentType::ChannelSelect as u8));
                obj.insert("custom_id".into(), json!(custom_id));
                insert_opt(&mut obj, "placeholder", placeholder.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "channel_types", channel_types.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "default_values", default_values.as_ref().map(|x| serde_json::to_value(x).unwrap()));
                insert_opt(&mut obj, "min_values", min_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "max_values", max_values.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "disabled", disabled.as_ref().map(|x| json!(x)));
            }
            Component::Section { components, accessory, id } => {
                obj.insert("type".into(), json!(ComponentType::Section as u8));
                obj.insert("components".into(), serde_json::to_value(components).map_err(serde::ser::Error::custom)?);
                if let Some(a) = accessory {
                    obj.insert("accessory".into(), serde_json::to_value(a.as_ref()).map_err(serde::ser::Error::custom)?);
                }
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::TextDisplay { content, id } => {
                obj.insert("type".into(), json!(ComponentType::TextDisplay as u8));
                obj.insert("content".into(), json!(content));
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::Thumbnail { media, description, spoiler, id } => {
                obj.insert("type".into(), json!(ComponentType::Thumbnail as u8));
                obj.insert("media".into(), serde_json::to_value(media).map_err(serde::ser::Error::custom)?);
                insert_opt(&mut obj, "description", description.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "spoiler", spoiler.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::MediaGallery { items, id } => {
                obj.insert("type".into(), json!(ComponentType::MediaGallery as u8));
                obj.insert("items".into(), serde_json::to_value(items).map_err(serde::ser::Error::custom)?);
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::File { file, spoiler, id } => {
                obj.insert("type".into(), json!(ComponentType::File as u8));
                obj.insert("file".into(), serde_json::to_value(file).map_err(serde::ser::Error::custom)?);
                insert_opt(&mut obj, "spoiler", spoiler.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::Separator { divider, spacing, id } => {
                obj.insert("type".into(), json!(ComponentType::Separator as u8));
                insert_opt(&mut obj, "divider", divider.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "spacing", spacing.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
            Component::Container { components, accent_color, spoiler, id } => {
                obj.insert("type".into(), json!(ComponentType::Container as u8));
                obj.insert("components".into(), serde_json::to_value(components).map_err(serde::ser::Error::custom)?);
                insert_opt(&mut obj, "accent_color", accent_color.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "spoiler", spoiler.as_ref().map(|x| json!(x)));
                insert_opt(&mut obj, "id", id.as_ref().map(|x| json!(x)));
            }
        }
        Value::Object(obj).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(deserializer)?;
        let obj = v.as_object().ok_or_else(|| serde::de::Error::custom("expected component object"))?;
        let ty = obj.get("type")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| serde::de::Error::custom("missing/invalid integer `type`"))? as u8;

        let take_str = |k: &str| -> Result<String, D::Error> {
            obj.get(k).and_then(|v| v.as_str()).map(String::from)
                .ok_or_else(|| serde::de::Error::custom(format!("missing string field `{k}`")))
        };
        let opt_str = |k: &str| obj.get(k).and_then(|v| v.as_str()).map(String::from);
        let opt_u8 = |k: &str| obj.get(k).and_then(|v| v.as_u64()).map(|x| x as u8);
        let opt_u32 = |k: &str| obj.get(k).and_then(|v| v.as_u64()).map(|x| x as u32);
        let opt_bool = |k: &str| obj.get(k).and_then(|v| v.as_bool());
        let opt_default_values = |k: &str| -> Result<Option<Vec<DefaultValue>>, D::Error> {
            obj.get(k).map(|v| serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)).transpose()
        };
        let opt_components = |k: &str| -> Result<Vec<Component>, D::Error> {
            obj.get(k)
                .map(|v| serde_json::from_value::<Vec<Component>>(v.clone()).map_err(serde::de::Error::custom))
                .transpose()?
                .ok_or_else(|| serde::de::Error::custom(format!("missing `{k}` array")))
        };
        let take_media = |k: &str| -> Result<UnfurledMediaItem, D::Error> {
            obj.get(k)
                .map(|v| serde_json::from_value::<UnfurledMediaItem>(v.clone()).map_err(serde::de::Error::custom))
                .ok_or_else(|| serde::de::Error::custom(format!("missing `{k}` object")))?
        };
        let opt_channel_types = |k: &str| -> Option<Vec<u8>> {
            obj.get(k).and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|x| x.as_u64().map(|y| y as u8)).collect()
            })
        };

        let comp = match ty {
            1 => Component::ActionRow { components: opt_components("components")?, id: opt_u32("id") },
            5 => Component::UserSelect {
                custom_id: take_str("custom_id")?,
                placeholder: opt_str("placeholder"),
                default_values: opt_default_values("default_values")?,
                min_values: opt_u8("min_values"),
                max_values: opt_u8("max_values"),
                disabled: opt_bool("disabled"),
            },
            6 => Component::RoleSelect {
                custom_id: take_str("custom_id")?,
                placeholder: opt_str("placeholder"),
                default_values: opt_default_values("default_values")?,
                min_values: opt_u8("min_values"),
                max_values: opt_u8("max_values"),
                disabled: opt_bool("disabled"),
            },
            7 => Component::MentionableSelect {
                custom_id: take_str("custom_id")?,
                placeholder: opt_str("placeholder"),
                default_values: opt_default_values("default_values")?,
                min_values: opt_u8("min_values"),
                max_values: opt_u8("max_values"),
                disabled: opt_bool("disabled"),
            },
            8 => Component::ChannelSelect {
                custom_id: take_str("custom_id")?,
                placeholder: opt_str("placeholder"),
                channel_types: opt_channel_types("channel_types"),
                default_values: opt_default_values("default_values")?,
                min_values: opt_u8("min_values"),
                max_values: opt_u8("max_values"),
                disabled: opt_bool("disabled"),
            },
            9 => {
                let accessory = obj.get("accessory")
                    .map(|v| serde_json::from_value::<Component>(v.clone()).map_err(serde::de::Error::custom))
                    .transpose()?
                    .map(Box::new);
                Component::Section {
                    components: opt_components("components")?,
                    accessory,
                    id: opt_u32("id"),
                }
            }
            10 => Component::TextDisplay { content: take_str("content")?, id: opt_u32("id") },
            11 => Component::Thumbnail {
                media: take_media("media")?,
                description: opt_str("description"),
                spoiler: opt_bool("spoiler"),
                id: opt_u32("id"),
            },
            12 => {
                let items = obj.get("items")
                    .map(|v| serde_json::from_value::<Vec<MediaGalleryItem>>(v.clone()).map_err(serde::de::Error::custom))
                    .transpose()?
                    .ok_or_else(|| serde::de::Error::custom("missing `items` array"))?;
                Component::MediaGallery { items, id: opt_u32("id") }
            }
            13 => Component::File { file: take_media("file")?, spoiler: opt_bool("spoiler"), id: opt_u32("id") },
            14 => Component::Separator { divider: opt_bool("divider"), spacing: opt_u8("spacing"), id: opt_u32("id") },
            17 => Component::Container {
                components: opt_components("components")?,
                accent_color: opt_u32("accent_color"),
                spoiler: opt_bool("spoiler"),
                id: opt_u32("id"),
            },
            other => return Err(serde::de::Error::custom(format!("unsupported component type tag: {other}"))),
        };
        Ok(comp)
    }
}

impl Component {
    /// Construct a [`Component::TextDisplay`] from a markdown string.
    pub fn text_display(content: impl Into<String>) -> Self {
        Self::TextDisplay { content: content.into(), id: None }
    }

    /// Construct a [`Component::Section`] from a list of children, with no
    /// accessory.  Use [`Component::section_with_accessory`] to attach a
    /// thumbnail or button.
    pub fn section(components: Vec<Component>) -> Self {
        Self::Section { components, accessory: None, id: None }
    }

    /// Construct a [`Component::Section`] with an accessory (typically a
    /// [`Component::Thumbnail`] or a button represented as a `Component`).
    pub fn section_with_accessory(components: Vec<Component>, accessory: Component) -> Self {
        Self::Section { components, accessory: Some(Box::new(accessory)), id: None }
    }

    /// Construct a [`Component::Thumbnail`] from a media URL.
    pub fn thumbnail(url: impl Into<String>) -> Self {
        Self::Thumbnail {
            media: UnfurledMediaItem::new(url),
            description: None,
            spoiler: None,
            id: None,
        }
    }

    /// Construct a [`Component::MediaGallery`] from a list of items.
    pub fn media_gallery(items: Vec<MediaGalleryItem>) -> Self {
        Self::MediaGallery { items, id: None }
    }

    /// Construct a [`Component::File`] from an `attachment://…` URL.
    pub fn file(url: impl Into<String>) -> Self {
        Self::File { file: UnfurledMediaItem::new(url), spoiler: None, id: None }
    }

    /// Construct a [`Component::Separator`] (defaults: divider line, small
    /// spacing).  Pass `None` for either argument to omit it.
    pub fn separator(divider: Option<bool>, spacing: Option<u8>) -> Self {
        Self::Separator { divider, spacing, id: None }
    }

    /// Construct a [`Component::Container`] holding the given children.
    pub fn container(components: Vec<Component>) -> Self {
        Self::Container { components, accent_color: None, spoiler: None, id: None }
    }
}

#[cfg(test)]
mod component_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn text_display_serializes_with_type_tag() {
        let c = Component::text_display("hello **world**");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], 10);
        assert_eq!(v["content"], "hello **world**");
    }

    #[test]
    fn user_select_round_trips() {
        let c = Component::UserSelect {
            custom_id: "pick_user".into(),
            placeholder: Some("Pick someone".into()),
            default_values: Some(vec![DefaultValue::user("123")]),
            min_values: Some(1),
            max_values: Some(3),
            disabled: Some(false),
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], 5);
        assert_eq!(v["custom_id"], "pick_user");
        assert_eq!(v["default_values"][0]["type"], "user");
        let back: Component = serde_json::from_value(v).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn channel_select_carries_channel_types() {
        let c = Component::ChannelSelect {
            custom_id: "chan".into(),
            placeholder: None,
            channel_types: Some(vec![0, 5]),
            default_values: None,
            min_values: None,
            max_values: None,
            disabled: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], 8);
        assert_eq!(v["channel_types"], json!([0, 5]));
    }

    #[test]
    fn section_with_accessory_thumbnail() {
        let s = Component::section_with_accessory(
            vec![Component::text_display("Hello there")],
            Component::thumbnail("https://example.com/x.png"),
        );
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], 9);
        assert_eq!(v["components"][0]["type"], 10);
        assert_eq!(v["accessory"]["type"], 11);
        assert_eq!(v["accessory"]["media"]["url"], "https://example.com/x.png");
    }

    #[test]
    fn container_with_separator_and_gallery() {
        let c = Component::container(vec![
            Component::text_display("Header"),
            Component::separator(Some(true), Some(2)),
            Component::media_gallery(vec![
                MediaGalleryItem::new("https://example.com/1.png").description("first"),
                MediaGalleryItem::new("https://example.com/2.png").spoiler(true),
            ]),
        ]);
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], 17);
        assert_eq!(v["components"][1]["type"], 14);
        assert_eq!(v["components"][1]["divider"], true);
        assert_eq!(v["components"][2]["type"], 12);
        assert_eq!(v["components"][2]["items"][1]["spoiler"], true);
    }

    #[test]
    fn component_type_repr_values() {
        assert_eq!(ComponentType::ActionRow as u8, 1);
        assert_eq!(ComponentType::UserSelect as u8, 5);
        assert_eq!(ComponentType::ChannelSelect as u8, 8);
        assert_eq!(ComponentType::Section as u8, 9);
        assert_eq!(ComponentType::Container as u8, 17);
    }
}
