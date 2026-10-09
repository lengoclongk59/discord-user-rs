//! Application operations for DiscordUser.
//!
//! Endpoints under `/applications/@me` and
//! `/applications/{application.id}/activity-instances/{instance.id}`.

use std::borrow::Cow;

use serde_json::Value;

use crate::{
    context::DiscordContext, error::Result, route::Route, types::application::Application,
};

impl<T: DiscordContext + Send + Sync> ApplicationOps for T {}

/// Extension trait providing operations against the current application
/// resource and its activity instances.
#[allow(async_fn_in_trait)]
pub trait ApplicationOps: DiscordContext {
    /// Fetch the application owned by the current bot/user.
    ///
    /// Targets `GET /applications/@me`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_current_application(&self) -> Result<Application> {
        self.http().get(Route::CurrentApplication).await
    }

    /// Patch the current application's metadata (description, tags, install
    /// params, role-connection URL, event webhook config, etc.).
    ///
    /// Targets `PATCH /applications/@me`. `body` accepts the documented
    /// editable fields and is left untyped to avoid coupling to a particular
    /// subset of Discord's evolving schema.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn edit_current_application(&self, body: Value) -> Result<Application> {
        self.http().patch(Route::CurrentApplication, body).await
    }

    /// Fetch a live application activity instance by its embedded-activity
    /// instance ID.
    ///
    /// Targets `GET /applications/{application.id}/activity-instances/{instance_id}`.
    /// The response shape is currently undocumented for typed deserialization,
    /// so callers receive the raw JSON [`Value`].
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure or if the activity
    /// instance has already terminated.
    async fn get_application_activity_instance(
        &self,
        application_id: u64,
        instance_id: &str,
    ) -> Result<Value> {
        self.http()
            .get(Route::ApplicationActivityInstance {
                application_id,
                instance_id: Cow::Owned(instance_id.to_string()),
            })
            .await
    }
}
