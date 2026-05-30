//! `WriteApi` extensions: advanced message operations.
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::cli::api::WriteApi;
use crate::route::Route;

impl WriteApi {
    /// POST /channels/{channel_id}/messages/bulk-delete — delete up to 100 messages at once.
    pub async fn msg_bulk_delete(&self, channel_id: &str, ids: &[String]) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        let body = json!({ "messages": ids });
        self.raw()
            .post_no_response(
                Route::Custom(format!("channels/{}/messages/bulk-delete", channel_id).into()),
                body,
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// POST /channels/{channel_id}/messages/{message_id}/crosspost — publish to followers.
    pub async fn msg_crosspost(&self, channel_id: &str, message_id: &str) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        self.raw()
            .post::<Value, _>(
                Route::Custom(
                    format!("channels/{}/messages/{}/crosspost", channel_id, message_id).into(),
                ),
                json!({}),
            )
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /channels/{channel_id}/messages/{message_id}/reactions — clear all reactions.
    pub async fn msg_remove_all_reactions(&self, channel_id: &str, message_id: &str) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        self.raw()
            .delete(Route::Custom(
                format!("channels/{}/messages/{}/reactions", channel_id, message_id).into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /channels/{channel_id}/messages/{message_id}/reactions/{emoji} — clear one emoji.
    pub async fn msg_clear_emoji(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        let encoded = urlencoding::encode(emoji).into_owned();
        self.raw()
            .delete(Route::Custom(
                format!(
                    "channels/{}/messages/{}/reactions/{}",
                    channel_id, message_id, encoded
                )
                .into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// DELETE /channels/{channel_id}/messages/{message_id}/reactions/{emoji}/{user_id} — remove one user's reaction.
    pub async fn msg_remove_user_reaction(
        &self,
        channel_id: &str,
        message_id: &str,
        user_id: &str,
        emoji: &str,
    ) -> Result<()> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        crate::cli::api::parse_snowflake(user_id)?;
        let encoded = urlencoding::encode(emoji).into_owned();
        self.raw()
            .delete(Route::Custom(
                format!(
                    "channels/{}/messages/{}/reactions/{}/{}",
                    channel_id, message_id, encoded, user_id
                )
                .into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// GET /channels/{channel_id}/messages/{message_id}/reactions/{emoji}?limit=N — list reactors.
    pub async fn msg_get_reactors(
        &self,
        channel_id: &str,
        message_id: &str,
        emoji: &str,
        limit: u32,
    ) -> Result<Value> {
        crate::cli::api::parse_snowflake(channel_id)?;
        crate::cli::api::parse_snowflake(message_id)?;
        let encoded = urlencoding::encode(emoji).into_owned();
        self.raw()
            .get::<Value>(Route::Custom(
                format!(
                    "channels/{}/messages/{}/reactions/{}?limit={}",
                    channel_id, message_id, encoded, limit
                )
                .into(),
            ))
            .await
            .map_err(|e| anyhow!(e.to_string()))
    }
}
