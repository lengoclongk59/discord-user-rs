//! Application role-connection metadata operations for DiscordUser.
//!
//! Endpoints under `/applications/{application.id}/role-connections/metadata`.

use crate::{
    context::DiscordContext,
    error::Result,
    route::Route,
    types::monetization::ApplicationRoleConnectionMetadata,
};

impl<T: DiscordContext + Send + Sync> RoleConnectionOps for T {}

/// Extension trait for managing the metadata records that drive Discord
/// role-connection (linked-role) verification flows for an application.
#[allow(async_fn_in_trait)]
pub trait RoleConnectionOps: DiscordContext {
    /// Fetch the metadata record set the application currently advertises.
    ///
    /// Targets `GET /applications/{application.id}/role-connections/metadata`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_application_role_connection_metadata_records(&self, application_id: u64) -> Result<Vec<ApplicationRoleConnectionMetadata>> {
        self.http().get(Route::ApplicationRoleConnectionMetadata { application_id }).await
    }

    /// Replace the application's role-connection metadata records.
    ///
    /// Targets `PUT /applications/{application.id}/role-connections/metadata`.
    /// The full record set is replaced atomically — pass every record you wish
    /// to keep.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn update_application_role_connection_metadata_records(&self, application_id: u64, records: Vec<ApplicationRoleConnectionMetadata>) -> Result<Vec<ApplicationRoleConnectionMetadata>> {
        self.http().put(Route::ApplicationRoleConnectionMetadata { application_id }, records).await
    }
}
