//! SKU operations for DiscordUser.
//!
//! Endpoints under `/applications/{application.id}/skus`.

use crate::{context::DiscordContext, error::Result, route::Route, types::monetization::Sku};

impl<T: DiscordContext + Send + Sync> SkuOps for T {}

/// Extension trait providing read access to application SKUs (the monetization
/// products users can purchase).
#[allow(async_fn_in_trait)]
pub trait SkuOps: DiscordContext {
    /// List all SKUs published by an application.
    ///
    /// Targets `GET /applications/{application.id}/skus`. Discord returns the
    /// full SKU set on every call — there is no pagination cursor.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn list_skus(&self, application_id: u64) -> Result<Vec<Sku>> {
        self.http().get(Route::ApplicationSkus { application_id }).await
    }
}
