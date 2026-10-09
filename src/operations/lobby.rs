//! Lobby operations for DiscordUser.
//!
//! Endpoints under `/lobbies` — Discord's app-managed group communication
//! primitive used by embedded activities.

use crate::{
    context::DiscordContext,
    error::Result,
    route::Route,
    types::{
        lobby::{Lobby, LobbyMember},
        requests::{
            AddLobbyMemberRequest, BulkUpdateLobbyMembersRequest, CreateLobbyRequest,
            LinkChannelToLobbyRequest, ModifyLobbyRequest,
        },
    },
};

impl<T: DiscordContext + Send + Sync> LobbyOps for T {}

/// Extension trait providing CRUD plus member management against Discord
/// lobbies.
#[allow(async_fn_in_trait)]
pub trait LobbyOps: DiscordContext {
    /// Create a new lobby.
    ///
    /// Targets `POST /lobbies`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn create_lobby(&self, body: CreateLobbyRequest) -> Result<Lobby> {
        self.http().post(Route::Lobbies, body).await
    }

    /// Fetch a lobby by ID.
    ///
    /// Targets `GET /lobbies/{lobby.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the lobby is not
    /// found.
    async fn get_lobby(&self, lobby_id: u64) -> Result<Lobby> {
        self.http().get(Route::Lobby { lobby_id }).await
    }

    /// Modify a lobby's metadata, members, or idle timeout.
    ///
    /// Targets `PATCH /lobbies/{lobby.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn modify_lobby(&self, lobby_id: u64, body: ModifyLobbyRequest) -> Result<Lobby> {
        self.http().patch(Route::Lobby { lobby_id }, body).await
    }

    /// Permanently delete a lobby.
    ///
    /// Targets `DELETE /lobbies/{lobby.id}`. Discord returns 204 No Content
    /// on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn delete_lobby(&self, lobby_id: u64) -> Result<()> {
        self.http().delete(Route::Lobby { lobby_id }).await
    }

    /// Add or update a single lobby member.
    ///
    /// Targets `PUT /lobbies/{lobby.id}/members/{user.id}`. If the user is
    /// already a member, their metadata and flags are updated in place.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn add_lobby_member(
        &self,
        lobby_id: u64,
        user_id: u64,
        body: AddLobbyMemberRequest,
    ) -> Result<LobbyMember> {
        self.http()
            .put(Route::LobbyMember { lobby_id, user_id }, body)
            .await
    }

    /// Remove a lobby member.
    ///
    /// Targets `DELETE /lobbies/{lobby.id}/members/{user.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn remove_lobby_member(&self, lobby_id: u64, user_id: u64) -> Result<()> {
        self.http()
            .delete(Route::LobbyMember { lobby_id, user_id })
            .await
    }

    /// Bulk replace the full lobby member set.
    ///
    /// Targets `PATCH /lobbies/{lobby.id}/members`. The supplied list becomes
    /// the new member roster — members not present are removed.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn bulk_update_lobby_members(
        &self,
        lobby_id: u64,
        body: BulkUpdateLobbyMembersRequest,
    ) -> Result<Lobby> {
        self.http()
            .patch(Route::LobbyMembers { lobby_id }, body)
            .await
    }

    /// Leave a lobby as the current user.
    ///
    /// Targets `DELETE /lobbies/{lobby.id}/members/@me`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn leave_lobby(&self, lobby_id: u64) -> Result<()> {
        self.http().delete(Route::LeaveLobby { lobby_id }).await
    }

    /// Link a guild channel to a lobby so chat is mirrored between them.
    ///
    /// Targets `PATCH /lobbies/{lobby.id}/channel-linking`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn link_channel_to_lobby(
        &self,
        lobby_id: u64,
        body: LinkChannelToLobbyRequest,
    ) -> Result<Lobby> {
        self.http()
            .patch(Route::LobbyChannelLink { lobby_id }, body)
            .await
    }

    /// Remove the channel link from a lobby.
    ///
    /// Targets `DELETE /lobbies/{lobby.id}/channel-linking`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn unlink_channel_from_lobby(&self, lobby_id: u64) -> Result<()> {
        self.http()
            .delete(Route::LobbyChannelLink { lobby_id })
            .await
    }
}
