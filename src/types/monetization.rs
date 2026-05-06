use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sku {
    pub id: String,
    #[serde(rename = "type")]
    pub sku_type: u8, // 2=Durable, 3=Consumable, 5=Subscription, 6=SubscriptionGroup
    pub application_id: String,
    pub name: String,
    pub slug: String,
    pub flags: u64,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct SkuFlags: u64 {
        const AVAILABLE = 1 << 2;
        const GUILD_SUBSCRIPTION = 1 << 7;
        const USER_SUBSCRIPTION = 1 << 8;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entitlement {
    pub id: String,
    pub sku_id: String,
    pub application_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(rename = "type")]
    pub entitlement_type: u8, // 1=Purchase, 2=PremiumSubscription, ..., 8=TestModePurchase, etc.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub sku_ids: Vec<String>,
    pub entitlement_ids: Vec<String>,
    #[serde(default)]
    pub renewal_sku_ids: Option<Vec<String>>,
    pub current_period_start: String,
    pub current_period_end: String,
    pub status: u8, // 0=Active, 1=Ending, 2=Inactive
    #[serde(default)]
    pub canceled_at: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRoleConnectionMetadata {
    #[serde(rename = "type")]
    pub metadata_type: u8, // 1..=8
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub name_localizations: Option<std::collections::HashMap<String, String>>,
    pub description: String,
    #[serde(default)]
    pub description_localizations: Option<std::collections::HashMap<String, String>>,
}
