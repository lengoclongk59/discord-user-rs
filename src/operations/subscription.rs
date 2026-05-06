//! SKU subscription operations for DiscordUser.
//!
//! Endpoints under `/skus/{sku.id}/subscriptions`.

use crate::{context::DiscordContext, error::Result, route::Route, types::monetization::Subscription};

impl<T: DiscordContext + Send + Sync> SubscriptionOps for T {}

/// Optional query parameters for [`SubscriptionOps::list_sku_subscriptions`].
///
/// Mirrors the documented filter set on
/// `GET /skus/{sku.id}/subscriptions`.
#[derive(Debug, Default, Clone)]
pub struct SkuSubscriptionListQuery {
    /// Return subscriptions before this snowflake.
    pub before: Option<u64>,
    /// Return subscriptions after this snowflake.
    pub after: Option<u64>,
    /// Page size (1-100, default 100).
    pub limit: Option<u32>,
    /// Filter to a specific user's subscriptions. Required for non-OAuth bot
    /// tokens; OAuth user-token requests can omit it to receive their own.
    pub user_id: Option<u64>,
}

/// Extension trait providing read access to SKU subscriptions.
#[allow(async_fn_in_trait)]
pub trait SubscriptionOps: DiscordContext {
    /// List subscriptions for a SKU.
    ///
    /// Targets `GET /skus/{sku.id}/subscriptions`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn list_sku_subscriptions(&self, sku_id: u64, query: Option<SkuSubscriptionListQuery>) -> Result<Vec<Subscription>> {
        let q = query.unwrap_or_default();
        self.http()
            .get(Route::SkuSubscriptions {
                sku_id,
                before: q.before,
                after: q.after,
                limit: q.limit,
                user_id: q.user_id,
            })
            .await
    }

    /// Fetch a single SKU subscription by ID.
    ///
    /// Targets `GET /skus/{sku.id}/subscriptions/{subscription.id}`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the subscription is
    /// not found.
    async fn get_sku_subscription(&self, sku_id: u64, subscription_id: u64) -> Result<Subscription> {
        self.http().get(Route::SkuSubscription { sku_id, subscription_id }).await
    }
}
