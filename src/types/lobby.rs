use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lobby {
    pub id: String,
    pub application_id: String,
    #[serde(default)]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub members: Vec<LobbyMember>,
    #[serde(default)]
    pub linked_channel: Option<serde_json::Value>, // partial Channel
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyMember {
    pub id: String,
    #[serde(default)]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub flags: Option<u64>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LobbyMemberFlags: u64 {
        const CAN_LINK_LOBBY = 1 << 0;
    }
}
