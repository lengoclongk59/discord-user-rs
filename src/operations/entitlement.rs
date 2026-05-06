//! Entitlement operations for DiscordUser.
//!
//! Endpoints under `/applications/{application.id}/entitlements` plus the
//! one-shot consume endpoint.

use std::borrow::Cow;

use crate::{
    context::DiscordContext,
    error::Result,
    route::Route,
    types::{monetization::Entitlement, requests::CreateTestEntitlementRequest},
};

impl<T: DiscordContext + Send + Sync> EntitlementOps for T {}

/// Optional query parameters for [`EntitlementOps::list_entitlements`].
///
/// Mirrors the documented filter set on
/// `GET /applications/{application.id}/entitlements`.
#[derive(Debug, Default, Clone)]
pub struct EntitlementListQuery {
    /// Filter to a specific user's entitlements.
    pub user_id: Option<u64>,
    /// Comma-separated list of SKU snowflake IDs to filter by.
    pub sku_ids: Option<String>,
    /// Return entitlements before this snowflake (newer-than cursor).
    pub before: Option<u64>,
    /// Return entitlements after this snowflake (older-than cursor).
    pub after: Option<u64>,
    /// Page size (1-100, default 100).
    pub limit: Option<u32>,
    /// Filter to a specific guild's entitlements.
    pub guild_id: Option<u64>,
    /// When `true`, omits entitlements whose subscription period has ended.
    pub exclude_ended: Option<bool>,
    /// When `true`, omits entitlements that have been deleted.
    pub exclude_deleted: Option<bool>,
}

/// Extension trait providing entitlement CRUD plus the test-mode helpers
/// (`create_test_entitlement` / `delete_test_entitlement`) and the
/// consumable-entitlement consume endpoint.
#[allow(async_fn_in_trait)]
pub trait EntitlementOps: DiscordContext {
    /// List entitlements for an application, optionally filtered.
    ///
    /// Targets `GET /applications/{application.id}/entitlements`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn list_entitlements(&self, application_id: u64, query: Option<EntitlementListQuery>) -> Result<Vec<Entitlement>> {
        let q = query.unwrap_or_default();
        self.http()
            .get(Route::ApplicationEntitlements {
                application_id,
                user_id: q.user_id,
                sku_ids: q.sku_ids.map(Cow::Owned),
                before: q.before,
                after: q.after,
                limit: q.limit,
                guild_id: q.guild_id,
                exclude_ended: q.exclude_ended,
                exclude_deleted: q.exclude_deleted,
            })
            .await
    }

    /// Fetch a single entitlement by ID.
    ///
    /// Targets `GET /applications/{application.id}/entitlements/{entitlement.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the entitlement is
    /// not found.
    async fn get_entitlement(&self, application_id: u64, entitlement_id: u64) -> Result<Entitlement> {
        self.http().get(Route::ApplicationEntitlement { application_id, entitlement_id }).await
    }

    /// Create a test-mode entitlement that grants access without an actual
    /// purchase. Useful during development.
    ///
    /// Targets `POST /applications/{application.id}/entitlements`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn create_test_entitlement(&self, application_id: u64, body: CreateTestEntitlementRequest) -> Result<Entitlement> {
        self.http()
            .post(
                Route::ApplicationEntitlements {
                    application_id,
                    user_id: None,
                    sku_ids: None,
                    before: None,
                    after: None,
                    limit: None,
                    guild_id: None,
                    exclude_ended: None,
                    exclude_deleted: None,
                },
                body,
            )
            .await
    }

    /// Delete a test-mode entitlement created via
    /// [`create_test_entitlement`](Self::create_test_entitlement).
    ///
    /// Targets `DELETE /applications/{application.id}/entitlements/{entitlement.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn delete_test_entitlement(&self, application_id: u64, entitlement_id: u64) -> Result<()> {
        self.http().delete(Route::ApplicationEntitlement { application_id, entitlement_id }).await
    }

    /// Mark a one-time-purchase (consumable) entitlement as consumed.
    ///
    /// Targets `POST /applications/{application.id}/entitlements/{entitlement.id}/consume`.
    /// Discord returns 204 No Content on success.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn consume_entitlement(&self, application_id: u64, entitlement_id: u64) -> Result<()> {
        self.http()
            .post_no_response(Route::ConsumeEntitlement { application_id, entitlement_id }, serde_json::Value::Null)
            .await
    }
}
