//! `WriteApi` extensions: relationships (friends / block).

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// PUT users/@me/relationships/{user_id} {} — add / accept a friend request by user id.
    pub async fn friend_add(&self, user_id: &str) -> Result<()> {
        let uid: u64 = user_id
            .parse()
            .map_err(|_| anyhow!("invalid user id: {}", user_id))?;
        self.raw()
            .put_optional::<serde_json::Value, _>(Route::AddRelationship { user_id: uid }, json!({}))
            .await
            .map(|_| ())
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST users/@me/relationships {username} — send a friend request by global username (pomelo).
    pub async fn friend_request(&self, username: &str) -> Result<()> {
        let body = json!({ "username": username, "discriminator": null });
        self.raw()
            .post_no_response::<_>(Route::GetRelationships, body)
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE users/@me/relationships/{user_id} — remove a friend or cancel a pending request.
    pub async fn friend_remove(&self, user_id: &str) -> Result<()> {
        let uid: u64 = user_id
            .parse()
            .map_err(|_| anyhow!("invalid user id: {}", user_id))?;
        self.raw()
            .delete(Route::RemoveRelationship { user_id: uid })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// PUT users/@me/relationships/{user_id} {type:2} — block a user.
    pub async fn friend_block(&self, user_id: &str) -> Result<()> {
        let uid: u64 = user_id
            .parse()
            .map_err(|_| anyhow!("invalid user id: {}", user_id))?;
        self.raw()
            .put_optional::<serde_json::Value, _>(
                Route::AddRelationship { user_id: uid },
                json!({ "type": 2 }),
            )
            .await
            .map(|_| ())
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE users/@me/relationships/{user_id} — unblock a user.
    pub async fn friend_unblock(&self, user_id: &str) -> Result<()> {
        let uid: u64 = user_id
            .parse()
            .map_err(|_| anyhow!("invalid user id: {}", user_id))?;
        self.raw()
            .delete(Route::RemoveRelationship { user_id: uid })
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
