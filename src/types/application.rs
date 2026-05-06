use serde::{Deserialize, Serialize};
use crate::types::user::User;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Application {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub rpc_origins: Option<Vec<String>>,
    #[serde(default)]
    pub bot_public: Option<bool>,
    #[serde(default)]
    pub bot_require_code_grant: Option<bool>,
    #[serde(default)]
    pub bot: Option<User>,
    #[serde(default)]
    pub terms_of_service_url: Option<String>,
    #[serde(default)]
    pub privacy_policy_url: Option<String>,
    #[serde(default)]
    pub owner: Option<User>,
    #[serde(default)]
    pub verify_key: Option<String>,
    #[serde(default)]
    pub team: Option<serde_json::Value>,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub guild: Option<serde_json::Value>,
    #[serde(default)]
    pub primary_sku_id: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub cover_image: Option<String>,
    #[serde(default)]
    pub flags: Option<u64>,
    #[serde(default)]
    pub approximate_guild_count: Option<u64>,
    #[serde(default)]
    pub approximate_user_install_count: Option<u64>,
    #[serde(default)]
    pub redirect_uris: Option<Vec<String>>,
    #[serde(default)]
    pub interactions_endpoint_url: Option<String>,
    #[serde(default)]
    pub role_connections_verification_url: Option<String>,
    #[serde(default)]
    pub event_webhooks_url: Option<String>,
    #[serde(default)]
    pub event_webhooks_status: Option<u8>,
    #[serde(default)]
    pub event_webhooks_types: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub install_params: Option<serde_json::Value>,
    #[serde(default)]
    pub integration_types_config: Option<serde_json::Value>,
    #[serde(default)]
    pub custom_install_url: Option<String>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ApplicationFlags: u64 {
        const APPLICATION_AUTO_MODERATION_RULE_CREATE_BADGE = 1 << 6;
        const GATEWAY_PRESENCE = 1 << 12;
        const GATEWAY_PRESENCE_LIMITED = 1 << 13;
        const GATEWAY_GUILD_MEMBERS = 1 << 14;
        const GATEWAY_GUILD_MEMBERS_LIMITED = 1 << 15;
        const VERIFICATION_PENDING_GUILD_LIMIT = 1 << 16;
        const EMBEDDED = 1 << 17;
        const GATEWAY_MESSAGE_CONTENT = 1 << 18;
        const GATEWAY_MESSAGE_CONTENT_LIMITED = 1 << 19;
        const APPLICATION_COMMAND_BADGE = 1 << 23;
    }
}
