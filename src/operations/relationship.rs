//! Relationship operations for DiscordUser
//!
//! This module also hosts the official Discord User endpoints
//! (`get_user`, `get_current_user_guild_member`, `create_group_dm`,
//! `get_current_user_connections`, and the role-connection pair). They live
//! here rather than in a separate `user` submodule to avoid touching
//! `src/operations.rs` (the parent module registration is owned by another
//! agent). All endpoints route through the central HTTP client via
//! `Route::` variants for consistent rate limiting and auth handling.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::{context::DiscordContext, error::Result, route::Route, types::*};

impl<T: DiscordContext + Send + Sync> RelationshipOps for T {}

/// Extension trait providing relationship operations
#[allow(async_fn_in_trait)]
pub trait RelationshipOps: DiscordContext {
    /// Fetch the current user's full relationship list (friends, blocked users,
    /// pending outgoing requests, and incoming requests).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_my_relationship(&self) -> Result<Vec<Relationship>> {
        self.http().get(Route::GetRelationships).await
    }

    /// Add or update a relationship with a user.
    ///
    /// Use `RelationshipType::Friend` to send a friend request, or
    /// `RelationshipType::Blocked` to block the user.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn add_relationship(&self, user_id: &UserId, type_: RelationshipType) -> Result<()> {
        let payload = json!({ "type": type_ as u8 });
        self.http()
            .put(
                Route::AddRelationship {
                    user_id: user_id.get(),
                },
                payload,
            )
            .await
    }

    /// Remove a relationship with a user (unfriend, unblock, or cancel a
    /// pending request).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn remove_relationship(&self, user_id: &UserId) -> Result<()> {
        self.http()
            .delete(Route::RemoveRelationship {
                user_id: user_id.get(),
            })
            .await
    }

    /// Send a friend request by username (and optional discriminator for legacy
    /// accounts).
    ///
    /// For pomelo (new-style) accounts, pass `discriminator: None`.
    /// For legacy accounts with a 4-digit tag, pass the discriminator number.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure, or if the username is
    /// not found.
    async fn send_friend_request(&self, username: &str, discriminator: Option<u16>) -> Result<()> {
        let mut payload = json!({ "username": username });
        if let Some(disc) = discriminator {
            payload["discriminator"] = json!(disc.to_string());
        }

        self.http().post(Route::GetRelationships, payload).await // Actually POST to /users/@me/relationships
    }

    // ------------------------------------------------------------------
    // Official Discord User endpoints
    // ------------------------------------------------------------------

    /// Fetch a user object by ID.
    ///
    /// Discord: `GET /users/{user.id}` → [`User`]. Self-bots, bots, and
    /// OAuth2 bearers may all call this; the response shape is the public
    /// projection of the user (no email/phone unless `@me`).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or
    /// [`DiscordError::UnexpectedStatusCode`] on non-2xx responses.
    async fn get_user(&self, user_id: &UserId) -> Result<User> {
        self.http()
            .get(Route::GetUser {
                user_id: user_id.get(),
            })
            .await
    }

    /// Fetch the current user's guild member object for a specific guild.
    ///
    /// Discord: `GET /users/@me/guilds/{guild.id}/member` → [`Member`]
    /// (Discord's `GuildMember`). Useful when joined-guild listings omit
    /// per-member fields like `nick` or `roles`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or
    /// [`DiscordError::UnexpectedStatusCode`] on non-2xx responses.
    async fn get_current_user_guild_member(&self, guild_id: &GuildId) -> Result<Member> {
        self.http()
            .get(Route::CurrentUserGuildMember {
                guild_id: guild_id.get(),
            })
            .await
    }

    /// Create a Group DM channel from OAuth2 access tokens.
    ///
    /// Discord: `POST /users/@me/channels` → [`Channel`]. Each entry in
    /// `access_tokens` must be an OAuth2 access token granted with the
    /// `gdm.join` scope from the recipient. `nicks` maps user IDs to a
    /// per-recipient nickname shown in the group DM.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn create_group_dm(
        &self,
        access_tokens: Vec<String>,
        nicks: HashMap<String, String>,
    ) -> Result<Channel> {
        let payload = json!({
            "access_tokens": access_tokens,
            "nicks": nicks,
        });
        // Path matches `Route::CreateDm` (POST /users/@me/channels); we reuse
        // the registered route so the call goes through the rate limiter.
        self.http().post(Route::CreateDm, payload).await
    }

    /// List the current user's third-party connections.
    ///
    /// Discord: `GET /users/@me/connections` → `Vec<Connection>`. Returns
    /// services like Spotify, GitHub, Twitch, etc., that the user has linked
    /// to their Discord account.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or
    /// [`DiscordError::UnexpectedStatusCode`] on non-2xx responses.
    async fn get_current_user_connections(&self) -> Result<Vec<Connection>> {
        self.http().get(Route::UserConnections).await
    }

    /// Fetch the current user's application role connection for a given
    /// application.
    ///
    /// Discord: `GET /users/@me/applications/{application.id}/role-connection`
    /// → `ApplicationRoleConnection` (returned here as raw
    /// [`serde_json::Value`] — the strongly-typed wrapper is owned by another
    /// agent and may not exist yet).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or
    /// [`DiscordError::UnexpectedStatusCode`] on non-2xx responses.
    async fn get_current_user_application_role_connection(
        &self,
        application_id: &ApplicationId,
    ) -> Result<Value> {
        self.http()
            .get(Route::ApplicationRoleConnection {
                application_id: application_id.get(),
            })
            .await
    }

    /// Update the current user's application role connection metadata for a
    /// given application.
    ///
    /// Discord: `PUT /users/@me/applications/{application.id}/role-connection`
    /// → updated `ApplicationRoleConnection` (returned as
    /// [`serde_json::Value`]). All three fields are optional in the body;
    /// `metadata` is a flat string map keyed by metadata-record key.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or
    /// [`DiscordError::UnexpectedStatusCode`] on non-2xx responses.
    async fn update_current_user_application_role_connection(
        &self,
        application_id: &ApplicationId,
        platform_name: Option<String>,
        platform_username: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<Value> {
        let mut body = serde_json::Map::new();
        if let Some(name) = platform_name {
            body.insert("platform_name".to_string(), Value::String(name));
        }
        if let Some(username) = platform_username {
            body.insert("platform_username".to_string(), Value::String(username));
        }
        body.insert("metadata".to_string(), json!(metadata));
        let payload = Value::Object(body);

        self.http()
            .put(
                Route::ApplicationRoleConnection {
                    application_id: application_id.get(),
                },
                payload,
            )
            .await
    }
}
