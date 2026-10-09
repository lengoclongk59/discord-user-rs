//! Thread operations for DiscordUser

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{context::DiscordContext, error::Result, route::Route, types::*};

impl<T: DiscordContext + Send + Sync> ThreadOps for T {}

/// A thread member entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMember {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub join_timestamp: String,
    pub flags: u64,
}

/// Response from GET /guilds/{id}/threads/active
#[derive(Debug, Clone, Deserialize)]
pub struct ActiveThreads {
    pub threads: Vec<Channel>,
    pub members: Vec<ThreadMember>,
}

/// Response from the archived-threads listing endpoints.
///
/// Used by:
/// - `GET /channels/{channel.id}/threads/archived/public`
/// - `GET /channels/{channel.id}/threads/archived/private`
/// - `GET /channels/{channel.id}/users/@me/threads/archived/private`
#[derive(Debug, Clone, Deserialize)]
pub struct ArchivedThreadsResponse {
    /// The archived threads returned for this page.
    #[serde(default)]
    pub threads: Vec<Channel>,
    /// Thread-member entries for the calling user, one per thread the user
    /// has joined. Empty when the user is not a member of any returned
    /// thread.
    #[serde(default)]
    pub members: Vec<ThreadMember>,
    /// Whether more archived threads exist beyond this page. Use the
    /// earliest `archive_timestamp`/`id` from `threads` as the next `before`
    /// cursor.
    #[serde(default)]
    pub has_more: bool,
}

/// Response from `POST /channels/{channel.id}/threads` when the channel is a
/// forum or media channel.
///
/// Discord returns a [`Channel`] object for the new thread plus the seed
/// [`Message`] sent into it. The seed-message field is left as
/// [`serde_json::Value`] for forward compatibility with Discord's evolving
/// message schema.
#[derive(Debug, Clone, Deserialize)]
pub struct ForumThreadResponse {
    /// The newly created forum/media thread channel.
    #[serde(flatten)]
    pub channel: Channel,
    /// The first message posted to the thread, if returned by the API.
    #[serde(default)]
    pub message: Option<serde_json::Value>,
}

/// Extension trait providing thread management operations
#[allow(async_fn_in_trait)]
pub trait ThreadOps: DiscordContext {
    /// Create a thread not attached to a message.
    ///
    /// Set `req.channel_type` to 11 (PUBLIC_THREAD) or 12 (PRIVATE_THREAD).
    /// Use [`CreateThreadRequest::public`] / [`CreateThreadRequest::private`]
    /// helpers.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires CREATE_PUBLIC_THREADS or CREATE_PRIVATE_THREADS as appropriate.
    async fn create_thread(
        &self,
        channel_id: &ChannelId,
        req: CreateThreadRequest,
    ) -> Result<Channel> {
        self.http()
            .post(
                Route::CreateThread {
                    channel_id: channel_id.get(),
                },
                req,
            )
            .await
    }

    /// Create a thread attached to an existing message.
    ///
    /// The thread type is PUBLIC_THREAD by default (no `type` field needed).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires CREATE_PUBLIC_THREADS permission.
    async fn create_thread_from_message(
        &self,
        channel_id: &ChannelId,
        message_id: &MessageId,
        req: CreateThreadRequest,
    ) -> Result<Channel> {
        self.http()
            .post(
                Route::CreateThreadFromMessage {
                    channel_id: channel_id.get(),
                    message_id: message_id.get(),
                },
                req,
            )
            .await
    }

    /// Edit a thread's settings (name, archived state, locked, auto-archive
    /// duration, slowmode).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_THREADS permission (or be the thread creator for private
    /// threads).
    async fn edit_thread(&self, channel_id: &ChannelId, req: EditThreadRequest) -> Result<Channel> {
        self.http()
            .patch(
                Route::EditChannel {
                    channel_id: channel_id.get(),
                },
                req,
            )
            .await
    }

    /// Join a thread as the current user.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn join_thread(&self, channel_id: &ChannelId) -> Result<()> {
        self.http()
            .put(
                Route::JoinThread {
                    channel_id: channel_id.get(),
                },
                EMPTY_REQUEST,
            )
            .await
    }

    /// Leave a thread as the current user.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn leave_thread(&self, channel_id: &ChannelId) -> Result<()> {
        self.http()
            .delete(Route::LeaveThread {
                channel_id: channel_id.get(),
            })
            .await
    }

    /// Add a member to a thread.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_THREADS or be the thread creator.
    async fn add_thread_member(&self, channel_id: &ChannelId, user_id: &UserId) -> Result<()> {
        self.http()
            .put(
                Route::AddThreadMember {
                    channel_id: channel_id.get(),
                    user_id: user_id.get(),
                },
                EMPTY_REQUEST,
            )
            .await
    }

    /// Remove a member from a thread.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_THREADS or be the thread creator.
    async fn remove_thread_member(&self, channel_id: &ChannelId, user_id: &UserId) -> Result<()> {
        self.http()
            .delete(Route::RemoveThreadMember {
                channel_id: channel_id.get(),
                user_id: user_id.get(),
            })
            .await
    }

    /// Get all members of a thread.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_thread_members(&self, channel_id: &ChannelId) -> Result<Vec<ThreadMember>> {
        self.http()
            .get(Route::GetThreadMembers {
                channel_id: channel_id.get(),
            })
            .await
    }

    /// Get all active (non-archived) threads in a guild.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn get_active_threads(&self, guild_id: &GuildId) -> Result<ActiveThreads> {
        self.http()
            .get(Route::GetActiveThreads {
                guild_id: guild_id.get(),
            })
            .await
    }

    /// Archive a thread by setting `archived: true`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_THREADS or be the thread creator.
    async fn archive_thread(&self, channel_id: &ChannelId) -> Result<Channel> {
        self.edit_thread(
            channel_id,
            EditThreadRequest {
                archived: Some(true),
                ..Default::default()
            },
        )
        .await
    }

    /// Unarchive a thread by setting `archived: false`.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires MANAGE_THREADS or be the thread creator.
    async fn unarchive_thread(&self, channel_id: &ChannelId) -> Result<Channel> {
        self.edit_thread(
            channel_id,
            EditThreadRequest {
                archived: Some(false),
                ..Default::default()
            },
        )
        .await
    }

    /// Get a single thread member.
    ///
    /// `GET /channels/{channel.id}/thread-members/{user.id}?with_member=true`
    ///
    /// When `with_member` is `true`, Discord includes the corresponding
    /// guild-member object on the response so callers can render a name and
    /// avatar without a follow-up request.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure (e.g. 404 if the user is
    /// not a member of the thread).
    async fn get_thread_member(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
        with_member: bool,
    ) -> Result<ThreadMember> {
        self.http()
            .get(Route::GetThreadMember {
                channel_id: channel_id.get(),
                user_id: user_id.get(),
                with_member,
            })
            .await
    }

    /// List members of a thread (single page).
    ///
    /// `GET /channels/{channel.id}/thread-members?with_member={bool}&after={snowflake}&limit={1-100}`
    ///
    /// Returns up to `limit` (max 100) thread-member entries ordered by
    /// `user_id` ascending. Use the last returned member's `user_id` as the
    /// `after` cursor on the next call. Pass `limit: None` to use Discord's
    /// default (100).
    ///
    /// For automatic pagination across all pages, use
    /// [`list_all_thread_members`](Self::list_all_thread_members).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    async fn list_thread_members(
        &self,
        channel_id: &ChannelId,
        with_member: bool,
        after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<ThreadMember>> {
        self.http()
            .get(Route::ListThreadMembers {
                channel_id: channel_id.get(),
                with_member,
                after,
                limit,
            })
            .await
    }

    /// List every member of a thread, automatically paginating.
    ///
    /// Repeatedly calls [`list_thread_members`](Self::list_thread_members)
    /// with `limit=100`, advancing the `after` cursor by the highest returned
    /// `user_id` until Discord returns fewer than 100 members (signaling the
    /// final page).
    ///
    /// `with_member` is forwarded to each underlying call. Members lacking a
    /// `user_id` (which would otherwise be unable to advance the cursor) are
    /// kept in the result but do not influence pagination — pagination stops
    /// either when a short page is returned or when no member on the page
    /// carries a numeric `user_id`.
    ///
    /// # Errors
    /// Returns the first [`DiscordError::Http`] encountered while paging.
    async fn list_all_thread_members(
        &self,
        channel_id: &ChannelId,
        with_member: bool,
    ) -> Result<Vec<ThreadMember>> {
        let mut all: Vec<ThreadMember> = Vec::new();
        let mut after: Option<u64> = None;
        loop {
            let page = self
                .list_thread_members(channel_id, with_member, after, Some(100))
                .await?;
            let page_len = page.len();

            // Find the largest numeric user_id on this page to use as the
            // next cursor. Discord returns members ordered by user_id
            // ascending, so the last entry usually carries the max — but we
            // scan to be defensive against unordered responses.
            let next_after: Option<u64> = page
                .iter()
                .filter_map(|m| m.user_id.as_deref().and_then(|s| s.parse::<u64>().ok()))
                .max();

            all.extend(page);

            // Termination conditions:
            //   1. Fewer than 100 results → final page reached.
            //   2. No advanceable cursor → would loop forever otherwise.
            if page_len < 100 || next_after.is_none() {
                break;
            }
            after = next_after;
        }
        Ok(all)
    }

    /// List public archived threads in a channel.
    ///
    /// `GET /channels/{channel.id}/threads/archived/public?before={iso8601}&limit={n}`
    ///
    /// Threads are returned in descending order by `archive_timestamp`. Use
    /// the earliest archive timestamp from the response as the next `before`
    /// cursor (ISO 8601 format).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires READ_MESSAGE_HISTORY in the parent channel.
    async fn list_public_archived_threads(
        &self,
        channel_id: &ChannelId,
        before: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ArchivedThreadsResponse> {
        self.http()
            .get(Route::PublicArchivedThreads {
                channel_id: channel_id.get(),
                before: before.map(|s| Cow::Owned(s.to_string())),
                limit,
            })
            .await
    }

    /// List private archived threads in a channel.
    ///
    /// `GET /channels/{channel.id}/threads/archived/private?before={iso8601}&limit={n}`
    ///
    /// Same response shape and pagination rules as
    /// [`list_public_archived_threads`](Self::list_public_archived_threads).
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires READ_MESSAGE_HISTORY *and* MANAGE_THREADS on the parent
    /// channel.
    async fn list_private_archived_threads(
        &self,
        channel_id: &ChannelId,
        before: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ArchivedThreadsResponse> {
        self.http()
            .get(Route::PrivateArchivedThreads {
                channel_id: channel_id.get(),
                before: before.map(|s| Cow::Owned(s.to_string())),
                limit,
            })
            .await
    }

    /// List private archived threads the current user has joined.
    ///
    /// `GET /channels/{channel.id}/users/@me/threads/archived/private?before={snowflake}&limit={n}`
    ///
    /// Note: unlike the other archived-thread listings, the `before` cursor
    /// here is a thread *snowflake ID*, not an ISO 8601 timestamp.
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires READ_MESSAGE_HISTORY on the parent channel. MANAGE_THREADS is
    /// not required because the listing is scoped to the current user's
    /// memberships.
    async fn list_joined_private_archived_threads(
        &self,
        channel_id: &ChannelId,
        before: Option<u64>,
        limit: Option<u32>,
    ) -> Result<ArchivedThreadsResponse> {
        self.http()
            .get(Route::JoinedPrivateArchivedThreads {
                channel_id: channel_id.get(),
                before,
                limit,
            })
            .await
    }

    /// Start a thread in a forum or media channel.
    ///
    /// `POST /channels/{channel.id}/threads`
    ///
    /// Forum/media-channel threads require an initial message; the `message`
    /// argument carries that payload (content, embeds, components, etc.).
    /// Tags from the forum's `available_tags` may be selected via
    /// `applied_tags`.
    ///
    /// Returns the new thread [`Channel`] together with the seed message
    /// (when included by Discord) wrapped in [`ForumThreadResponse`].
    ///
    /// # Errors
    /// Returns [`DiscordError::Http`] on HTTP failure.
    ///
    /// # Permissions
    /// Requires SEND_MESSAGES in the parent forum/media channel.
    async fn start_thread_in_forum_channel(
        &self,
        channel_id: &ChannelId,
        name: String,
        auto_archive_duration: Option<u32>,
        rate_limit_per_user: Option<u32>,
        message: ForumThreadMessage,
        applied_tags: Option<Vec<String>>,
    ) -> Result<ForumThreadResponse> {
        // We build an inline JSON body rather than reusing CreateThreadRequest
        // so the typed `message: ForumThreadMessage` survives serialization
        // (CreateThreadRequest declares `message` as `serde_json::Value`,
        // which would force an extra round-trip through serde_json::to_value
        // to use here).
        #[derive(Serialize)]
        struct StartForumThreadBody<'a> {
            name: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            auto_archive_duration: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            rate_limit_per_user: Option<u32>,
            message: &'a ForumThreadMessage,
            #[serde(skip_serializing_if = "Option::is_none")]
            applied_tags: Option<Vec<String>>,
        }

        let body = StartForumThreadBody {
            name,
            auto_archive_duration,
            rate_limit_per_user,
            message: &message,
            applied_tags,
        };

        self.http()
            .post(
                Route::StartThreadInForumChannel {
                    channel_id: channel_id.get(),
                },
                body,
            )
            .await
    }
}
