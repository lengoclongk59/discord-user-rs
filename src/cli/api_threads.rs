//! `WriteApi` extensions: threads.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// POST /channels/{channel_id}/threads — create a standalone thread (type 11 = PUBLIC_THREAD).
    pub async fn thread_create_standalone(
        &self,
        channel_id: &str,
        name: &str,
        auto_archive: u32,
    ) -> Result<Value> {
        let cid: u64 = channel_id
            .parse()
            .map_err(|_| anyhow!("invalid channel id: {}", channel_id))?;
        let body = json!({
            "name": name,
            "type": 11,
            "auto_archive_duration": auto_archive,
        });
        self.raw()
            .post::<Value, _>(Route::CreateThread { channel_id: cid }, body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST /channels/{channel_id}/messages/{message_id}/threads — create a thread from a message.
    pub async fn thread_create_from_message(
        &self,
        channel_id: &str,
        message_id: &str,
        name: &str,
        auto_archive: u32,
    ) -> Result<Value> {
        let cid: u64 = channel_id
            .parse()
            .map_err(|_| anyhow!("invalid channel id: {}", channel_id))?;
        let mid: u64 = message_id
            .parse()
            .map_err(|_| anyhow!("invalid message id: {}", message_id))?;
        let body = json!({
            "name": name,
            "auto_archive_duration": auto_archive,
        });
        self.raw()
            .post::<Value, _>(
                Route::CreateThreadFromMessage {
                    channel_id: cid,
                    message_id: mid,
                },
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PUT /channels/{thread_id}/thread-members/@me — join a thread.
    pub async fn thread_join(&self, thread_id: &str) -> Result<()> {
        let tid: u64 = thread_id
            .parse()
            .map_err(|_| anyhow!("invalid thread id: {}", thread_id))?;
        self.raw()
            .put_empty(Route::JoinThread { channel_id: tid })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /channels/{thread_id}/thread-members/@me — leave a thread.
    pub async fn thread_leave(&self, thread_id: &str) -> Result<()> {
        let tid: u64 = thread_id
            .parse()
            .map_err(|_| anyhow!("invalid thread id: {}", thread_id))?;
        self.raw()
            .delete(Route::LeaveThread { channel_id: tid })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PUT /channels/{thread_id}/thread-members/{user_id} — add a member to a thread.
    pub async fn thread_add_member(&self, thread_id: &str, user_id: &str) -> Result<()> {
        let tid: u64 = thread_id
            .parse()
            .map_err(|_| anyhow!("invalid thread id: {}", thread_id))?;
        let uid: u64 = user_id
            .parse()
            .map_err(|_| anyhow!("invalid user id: {}", user_id))?;
        self.raw()
            .put_empty(Route::AddThreadMember {
                channel_id: tid,
                user_id: uid,
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /channels/{thread_id}/thread-members/{user_id} — remove a member from a thread.
    pub async fn thread_remove_member(&self, thread_id: &str, user_id: &str) -> Result<()> {
        let tid: u64 = thread_id
            .parse()
            .map_err(|_| anyhow!("invalid thread id: {}", thread_id))?;
        let uid: u64 = user_id
            .parse()
            .map_err(|_| anyhow!("invalid user id: {}", user_id))?;
        self.raw()
            .delete(Route::RemoveThreadMember {
                channel_id: tid,
                user_id: uid,
            })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PATCH /channels/{thread_id} — archive (and optionally lock) a thread.
    pub async fn thread_archive(&self, thread_id: &str, locked: bool) -> Result<Value> {
        let tid: u64 = thread_id
            .parse()
            .map_err(|_| anyhow!("invalid thread id: {}", thread_id))?;
        let body = json!({
            "archived": true,
            "locked": locked,
        });
        self.raw()
            .patch::<Value, _>(Route::EditChannel { channel_id: tid }, body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PATCH /channels/{thread_id} — unarchive a thread.
    pub async fn thread_unarchive(&self, thread_id: &str) -> Result<Value> {
        let tid: u64 = thread_id
            .parse()
            .map_err(|_| anyhow!("invalid thread id: {}", thread_id))?;
        let body = json!({
            "archived": false,
        });
        self.raw()
            .patch::<Value, _>(Route::EditChannel { channel_id: tid }, body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
