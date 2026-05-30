//! `WriteApi` extensions: channel permission overwrites.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// PUT /channels/{channel_id}/permissions/{overwrite_id}
    /// Sets a permission overwrite. `kind`: 0 = role, 1 = member.
    /// `allow` and `deny` are stringified permission bitfields.
    pub async fn perms_set(
        &self,
        channel_id: &str,
        overwrite_id: &str,
        kind: u8,
        allow: &str,
        deny: &str,
    ) -> Result<()> {
        let cid: u64 = channel_id
            .parse()
            .map_err(|_| anyhow!("invalid channel id: {}", channel_id))?;
        let oid: u64 = overwrite_id
            .parse()
            .map_err(|_| anyhow!("invalid overwrite id: {}", overwrite_id))?;
        let body = json!({ "type": kind, "allow": allow, "deny": deny });
        self.raw()
            .put::<Value, _>(
                Route::EditChannelPermissions {
                    channel_id: cid,
                    overwrite_id: oid,
                },
                body,
            )
            .await
            .map(|_| ())
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /channels/{channel_id}/permissions/{overwrite_id}
    /// Removes a permission overwrite.
    pub async fn perms_clear(&self, channel_id: &str, overwrite_id: &str) -> Result<()> {
        let cid: u64 = channel_id
            .parse()
            .map_err(|_| anyhow!("invalid channel id: {}", channel_id))?;
        let oid: u64 = overwrite_id
            .parse()
            .map_err(|_| anyhow!("invalid overwrite id: {}", overwrite_id))?;
        self.raw()
            .delete(Route::DeleteChannelPermission {
                channel_id: cid,
                overwrite_id: oid,
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// GET /channels/{channel_id} and return the `permission_overwrites` array.
    pub async fn perms_list(&self, channel_id: &str) -> Result<Value> {
        let cid: u64 = channel_id
            .parse()
            .map_err(|_| anyhow!("invalid channel id: {}", channel_id))?;
        let chan: Value = self
            .raw()
            .get::<Value>(Route::GetChannel { channel_id: cid })
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        Ok(chan
            .get("permission_overwrites")
            .cloned()
            .unwrap_or(Value::Array(vec![])))
    }
}
