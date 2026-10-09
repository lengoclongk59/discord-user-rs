//! HTTP client for Discord API requests

use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use reqwest::{header, Client, Method, Response};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

use crate::error::{DiscordError, Result};

/// Discord REST API version. v9 has been deprecated for several Discord
/// surfaces (profile, pinned-messages v2, etc.); v10 is the current stable
/// release. `cli/auth/windows.rs::validate_token` uses raw reqwest with the
/// same `/api/v10` path — keep them in sync if bumping.
pub const API_BASE: &str = "https://discord.com/api/v10";
const DEFAULT_RETRY_COUNT: u32 = 5;
const MAX_RETRY_AFTER_SECONDS: u64 = 30;

/// When the client may send a request again after an attempt failed.
///
/// A request that Discord already carried out is carried out again when it is
/// re-sent: a message is posted twice, a slash command fires twice. After a 429
/// Discord has carried out nothing, so waiting and sending again is safe. After
/// a 5xx or a transport failure (a timeout, a reset connection) nobody knows
/// whether it did, and re-sending a POST there is how one message becomes two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResendPolicy {
    /// The default. A 429 is waited out and the request sent again, whatever
    /// its method. After a 5xx or a transport failure only an idempotent method
    /// (GET, HEAD, OPTIONS, PUT, DELETE) is sent again; a POST or a PATCH
    /// returns the error.
    #[default]
    IdempotentOnly,
    /// Nothing is ever sent twice. A 429 returns [`DiscordError::RateLimited`]
    /// (the bucket is still recorded, so the next request to it waits for the
    /// reset); a 5xx or a transport failure returns its error. For an embedder
    /// that owns its retry decisions.
    Never,
}

impl ResendPolicy {
    /// Whether a request answered 429 is waited out and sent again.
    pub fn resends_after_rate_limit(self) -> bool {
        matches!(self, Self::IdempotentOnly)
    }

    /// Whether a request whose outcome is unknown (a 5xx, a transport
    /// failure) is sent again.
    pub fn resends_after_unknown_outcome(self, method: &Method) -> bool {
        matches!(self, Self::IdempotentOnly) && is_idempotent(method)
    }
}

/// HTTP's idempotent methods: sending one twice has the effect of sending it
/// once.
fn is_idempotent(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS | Method::PUT | Method::DELETE)
}

/// Read a response body as `T`. An empty body (a 204, or a 200 with nothing in
/// it) reads as JSON `null`, so `()` and `Option<_>` accept it: Discord answers
/// a pin, a reaction or a thread join with 204, and parsing the empty string
/// failed every one of them after Discord had carried it out.
fn parse_body<T: DeserializeOwned>(text: &str) -> Result<T> {
    let json = if text.trim().is_empty() { "null" } else { text };
    serde_json::from_str(json).map_err(|e| {
        error!(error = %e, body_len = text.len(), "Failed to parse response");
        DiscordError::Json(e)
    })
}

/// The credential a request is sent with. Shared by every clone of a client
/// and replaced whole, so one request never mixes an old token with new
/// headers.
struct Credentials {
    token: String,
    headers: HashMap<String, String>,
}

/// Cloudflare's invalid-request window (Discord docs: 10,000 in 10 minutes).
const INVALID_REQUEST_WINDOW: Duration = Duration::from_secs(10 * 60);
/// First soft warning threshold — caller still has plenty of headroom.
const INVALID_REQUEST_WARN_THRESHOLD: usize = 7000;
/// Hard danger threshold — IP ban is imminent.
const INVALID_REQUEST_ERROR_THRESHOLD: usize = 9500;

/// Sliding-window counter of "invalid" Discord responses (401/403/429).
///
/// Cloudflare temporarily IP-bans clients (error 1015) once they exceed
/// 10,000 invalid responses in any 10-minute window. We prune older entries
/// on every insert and emit `tracing` events when the count crosses the
/// configured warn / error thresholds.
#[derive(Clone, Default)]
pub struct InvalidRequestTracker {
    inner: Arc<RwLock<VecDeque<Instant>>>,
}

impl InvalidRequestTracker {
    /// Record a new invalid response (401/403/429) and prune entries older
    /// than `INVALID_REQUEST_WINDOW`. Returns the post-prune count or
    /// returns an error if the threshold is exceeded.
    pub async fn record(&self) -> std::result::Result<usize, DiscordError> {
        let now = Instant::now();
        let mut guard = self.inner.write().await;
        // Prune entries older than the sliding window.
        while let Some(&front) = guard.front() {
            if now.duration_since(front) > INVALID_REQUEST_WINDOW {
                guard.pop_front();
            } else {
                break;
            }
        }
        guard.push_back(now);
        let count = guard.len();
        drop(guard);  // Release lock before any logging or error handling
        
        if count >= INVALID_REQUEST_ERROR_THRESHOLD {
            tracing::error!(
                invalid_request_count = count,
                "invalid-request budget critical: Cloudflare 1015 IP ban imminent (>= {})",
                INVALID_REQUEST_ERROR_THRESHOLD
            );
            return Err(DiscordError::InvalidRequestThresholdReached { count });
        } else if count >= INVALID_REQUEST_WARN_THRESHOLD {
            tracing::warn!(
                invalid_request_count = count,
                "invalid-request budget elevated (>= {})",
                INVALID_REQUEST_WARN_THRESHOLD
            );
        }
        Ok(count)
    }

    /// Current count of invalid responses in the active 10-minute window.
    /// Performs a prune so callers see a fresh value.
    pub async fn count(&self) -> usize {
        let now = Instant::now();
        let mut guard = self.inner.write().await;
        while let Some(&front) = guard.front() {
            if now.duration_since(front) > INVALID_REQUEST_WINDOW {
                guard.pop_front();
            } else {
                break;
            }
        }
        guard.len()
    }
}

#[derive(Debug, Clone)]
pub struct RatelimitInfo {
    pub timeout: std::time::Duration,
    pub limit: u64,
    pub method: reqwest::Method,
    pub path: String,
    pub global: bool,
}

struct RateLimitInfo {
    retry_after: f64,
    bucket: Option<String>,
    global: bool,
    scope: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RatelimitingBucket {
    pub remaining: u64,
    pub limit: u64,
    pub reset_at: f64,
    /// Seconds until the bucket resets, parsed from `X-RateLimit-Reset-After`
    /// when present. Useful as a fallback when the absolute `reset_at` clock
    /// drifts from the server's clock.
    pub reset_after: Option<f64>,
}

#[derive(Clone, Default)]
pub struct Ratelimit {
    pub buckets: std::sync::Arc<dashmap::DashMap<String, RatelimitingBucket>>,
    pub callback: Option<std::sync::Arc<dyn Fn(RatelimitInfo) + Send + Sync>>,
    pub global: std::sync::Arc<tokio::sync::Mutex<()>>,
}

impl Ratelimit {
    pub fn get_route_key(method: &Method, endpoint: &str) -> String {
        let path = if let Some(stripped) = endpoint.split("/api/v10/").nth(1) { stripped } else { endpoint.trim_start_matches('/') };

        let path = path.split('?').next().unwrap_or(path);
        let parts: Vec<&str> = path.split('/').collect();
        let mut route = String::from(method.as_str());

        if parts.is_empty() {
            return route;
        }

        let mut iter = parts.iter().peekable();
        while let Some(&part) = iter.next() {
            route.push('/');
            match part {
                "channels" | "guilds" | "webhooks" => {
                    route.push_str(part);
                    if let Some(&id) = iter.next() {
                        route.push('/');
                        route.push_str(id);
                    }
                }
                _ => {
                    // Normalize ANY all-digit segment to `{id}` so per-route
                    // rate-limit buckets aren't fragmented by short numeric
                    // path params (e.g. positions, answer ids). The old
                    // `len() > 10` snowflake heuristic missed those.
                    if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                        route.push_str("{id}");
                    } else {
                        route.push_str(part);
                    }
                }
            }
        }

        route
    }

    pub async fn pre_hook(&self, method: &Method, endpoint: &str, route_key: &str) {
        let wait_info = {
            if let Some(bucket) = self.buckets.get(route_key) {
                if bucket.remaining == 0 {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();

                    if bucket.reset_at > now {
                        Some((bucket.reset_at - now, bucket.limit))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some((secs, limit)) = wait_info {
            if secs > 0.0 {
                tracing::debug!("Preemptive rate limit hit for {}, waiting {:.3}s", route_key, secs);
                if let Some(cb) = &self.callback {
                    cb(RatelimitInfo {
                        timeout: std::time::Duration::from_secs_f64(secs),
                        limit,
                        method: method.clone(),
                        path: endpoint.to_string(),
                        global: false,
                    });
                }
                tokio::time::sleep(std::time::Duration::from_secs_f64(secs)).await;
            }
        }
    }

    pub fn post_hook(&self, route_key: &str, headers: &reqwest::header::HeaderMap) {
        let remaining = headers.get("x-ratelimit-remaining").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<u64>().ok());

        let limit = headers.get("x-ratelimit-limit").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<u64>().ok());

        let reset_at = headers.get("x-ratelimit-reset").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<f64>().ok());

        let reset_after = headers.get("x-ratelimit-reset-after").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<f64>().ok());

        if let (Some(remaining), Some(limit), Some(reset_at)) = (remaining, limit, reset_at) {
            // Use entry API for atomic update to prevent race condition
            self.buckets.entry(route_key.to_string())
                .and_modify(|b| {
                    b.remaining = remaining;
                    b.limit = limit;
                    b.reset_at = reset_at;
                    b.reset_after = reset_after;
                })
                .or_insert_with(|| RatelimitingBucket { remaining, limit, reset_at, reset_after });
        }
    }

    /// Seed/refresh the bucket cache when Discord returns a 429.
    ///
    /// 429 responses still carry the standard `X-RateLimit-*` headers; we
    /// want future `pre_hook` calls to honour the new `reset` timestamp so
    /// they back off proactively rather than re-spamming the bucket. If
    /// `X-RateLimit-Remaining` is missing (typical on 429), force it to 0
    /// so `pre_hook` will sleep until reset.
    pub fn record_429(&self, route_key: &str, headers: &reqwest::header::HeaderMap, fallback_retry_after: f64) {
        let remaining = headers.get("x-ratelimit-remaining").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

        let limit = headers.get("x-ratelimit-limit").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<u64>().ok()).unwrap_or_else(|| self.buckets.get(route_key).map(|b| b.limit).unwrap_or(0));

        let reset_after = headers.get("x-ratelimit-reset-after").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<f64>().ok());

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();

        let reset_at = headers.get("x-ratelimit-reset").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<f64>().ok()).unwrap_or_else(|| now + reset_after.unwrap_or(fallback_retry_after));

        // Use entry API for atomic update to prevent race condition
        self.buckets.entry(route_key.to_string())
            .and_modify(|b| {
                b.remaining = remaining;
                b.limit = limit;
                b.reset_at = reset_at;
                b.reset_after = reset_after;
            })
            .or_insert_with(|| RatelimitingBucket { remaining, limit, reset_at, reset_after });
    }
}

/// HTTP client for making Discord API requests
#[derive(Clone)]
pub struct DiscordHttpClient {
    client: Client,
    /// The token and the extra headers, replaceable while the client is in
    /// use ([`Self::set_token`], [`Self::replace_headers`]).
    credentials: Arc<std::sync::RwLock<Arc<Credentials>>>,
    /// Where relative routes are sent. [`API_BASE`] unless
    /// [`Self::with_api_base`] says otherwise.
    api_base: Arc<str>,
    resend: ResendPolicy,
    ratelimit: Ratelimit,
    ratelimiter_disabled: bool,
    /// Optional base64-encoded super-properties string mirrored from the
    /// gateway IDENTIFY payload. When set, every REST request carries an
    /// `X-Super-Properties` header so HTTP fingerprinting matches the
    /// concurrent gateway connection.
    super_properties_b64: Option<String>,
    /// Discord's `X-Discord-Locale` header (default: `en-US`).
    discord_locale: Option<String>,
    /// Discord's `X-Discord-Timezone` header (e.g. `America/Los_Angeles`).
    discord_timezone: Option<String>,
    /// Sliding-window invalid-request counter (Cloudflare 1015 protection).
    invalid_requests: InvalidRequestTracker,
}

impl DiscordHttpClient {
    /// Create a new Discord HTTP client with a token
    pub fn new(token: impl Into<String>, proxy: Option<String>, ratelimiter_disabled: bool) -> Self {
        let mut builder = Client::builder().timeout(Duration::from_secs(30)).pool_max_idle_per_host(10).pool_idle_timeout(Duration::from_secs(90)).tcp_keepalive(Duration::from_secs(60)).tcp_nodelay(true).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36").http1_only();

        if let Some(proxy_url) = proxy {
            if let Ok(p) = reqwest::Proxy::all(&proxy_url) {
                builder = builder.proxy(p);
            }
        }

        let client = builder.build().unwrap_or_else(|e| panic!("Failed to create HTTP client: {}", e));

        Self {
            client,
            credentials: Arc::new(std::sync::RwLock::new(Arc::new(Credentials { token: token.into(), headers: HashMap::new() }))),
            api_base: Arc::from(API_BASE),
            resend: ResendPolicy::default(),
            ratelimit: Ratelimit::default(),
            ratelimiter_disabled,
            super_properties_b64: None,
            discord_locale: Some("en-US".to_string()),
            discord_timezone: None,
            invalid_requests: InvalidRequestTracker::default(),
        }
    }

    /// Create a new Discord HTTP client with custom headers
    pub fn with_headers(headers: HashMap<String, String>, proxy: Option<String>, ratelimiter_disabled: bool) -> Option<Self> {
        let token = headers.iter().find(|(k, _)| k.to_lowercase() == "authorization").map(|(_, v)| v.clone())?;

        let mut builder = Client::builder().timeout(Duration::from_secs(30)).pool_max_idle_per_host(10).pool_idle_timeout(Duration::from_secs(90)).tcp_keepalive(Duration::from_secs(60)).tcp_nodelay(true).user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36").http1_only();

        if let Some(proxy_url) = proxy {
            if let Ok(p) = reqwest::Proxy::all(&proxy_url) {
                builder = builder.proxy(p);
            }
        }

        let client = builder.build().unwrap_or_else(|e| panic!("Failed to create HTTP client: {}", e));

        Some(Self {
            client,
            credentials: Arc::new(std::sync::RwLock::new(Arc::new(Credentials { token, headers }))),
            api_base: Arc::from(API_BASE),
            resend: ResendPolicy::default(),
            ratelimit: Ratelimit::default(),
            ratelimiter_disabled,
            super_properties_b64: None,
            discord_locale: Some("en-US".to_string()),
            discord_timezone: None,
            invalid_requests: InvalidRequestTracker::default(),
        })
    }

    /// Set the base64-encoded super-properties string mirrored from the
    /// gateway IDENTIFY payload. Discord considers a session more "real"
    /// when REST and gateway agree, so this value should match what the
    /// gateway sends in the IDENTIFY `properties` field.
    pub fn set_super_properties_b64(&mut self, value: Option<String>) {
        self.super_properties_b64 = value;
    }

    /// Configure the `X-Discord-Locale` header (e.g. `en-US`). Pass `None`
    /// to suppress it.
    pub fn set_discord_locale(&mut self, locale: Option<String>) {
        self.discord_locale = locale;
    }

    /// Configure the `X-Discord-Timezone` header (e.g.
    /// `America/Los_Angeles`). Pass `None` to suppress it.
    pub fn set_discord_timezone(&mut self, tz: Option<String>) {
        self.discord_timezone = tz;
    }

    /// Current count of invalid responses (HTTP 401/403/429) recorded in
    /// the active 10-minute Cloudflare window. Use this to back off
    /// voluntarily before Cloudflare returns error 1015.
    ///
    /// The check is async because it acquires a write lock to prune stale
    /// entries first; reading a stale snapshot would defeat the purpose of
    /// the sliding window.
    pub async fn invalid_request_count(&self) -> usize {
        self.invalid_requests.count().await
    }

    /// The token requests are sent with now.
    pub fn token(&self) -> String {
        self.credentials().token.clone()
    }

    /// The credential as it stands. A request takes one snapshot per attempt.
    fn credentials(&self) -> Arc<Credentials> {
        Arc::clone(&self.credentials.read().unwrap_or_else(|poisoned| poisoned.into_inner()))
    }

    fn store_credentials(&self, next: Credentials) {
        *self.credentials.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(next);
    }

    /// Replace the token for every later request, on this client and every
    /// clone of it. The extra headers are kept. The rate-limit buckets are
    /// kept too, which is the point of replacing the token rather than
    /// building a new client.
    pub fn set_token(&self, token: impl Into<String>) {
        let headers = self.credentials().headers.clone();
        self.store_credentials(Credentials { token: token.into(), headers });
    }

    /// Replace the extra headers and the token together, as
    /// [`Self::with_headers`] reads them: the `authorization` entry (any
    /// case) is the token. Returns `false` and changes nothing when there is
    /// no `authorization` entry.
    pub fn replace_headers(&self, headers: HashMap<String, String>) -> bool {
        let Some(token) = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("authorization")).map(|(_, v)| v.clone()) else {
            return false;
        };
        self.store_credentials(Credentials { token, headers });
        true
    }

    /// Send relative routes to `base` instead of [`API_BASE`]: a stand-in
    /// server in a test, or a recording proxy. A route that is already an
    /// absolute URL is sent where it says.
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = Arc::from(base.into().trim_end_matches('/'));
        self
    }

    /// Where relative routes are sent.
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// Choose when a failed request may be sent again. See [`ResendPolicy`].
    pub fn with_resend_policy(mut self, policy: ResendPolicy) -> Self {
        self.resend = policy;
        self
    }

    /// The policy in force. See [`ResendPolicy`].
    pub fn resend_policy(&self) -> ResendPolicy {
        self.resend
    }

    fn url_for(&self, endpoint: &str) -> String {
        if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}/{}", self.api_base, endpoint.trim_start_matches('/'))
        }
    }

    /// The extra headers, minus the token (sent by
    /// [`Self::apply_common_headers`]). A multipart body names its own content
    /// type with its boundary, so a configured `content-type` is left out there.
    fn apply_custom_headers(request: reqwest::RequestBuilder, creds: &Credentials, multipart: bool) -> reqwest::RequestBuilder {
        let mut request = request;
        for (key, value) in &creds.headers {
            if key.eq_ignore_ascii_case("authorization") || (multipart && key.eq_ignore_ascii_case("content-type")) {
                continue;
            }
            request = request.header(key.as_str(), value.as_str());
        }
        request
    }

    /// Returns `true` when the configured token represents a Discord
    /// **application identity** — either a real bot token
    /// (`Authorization: Bot <token>`) or an OAuth2 access token
    /// (`Authorization: Bearer <token>`). Returns `false` for
    /// user-account self-bot tokens (bare token, no prefix).
    ///
    /// Operations that send body fields specific to the user-account
    /// identity (e.g. the `mobile_network_type` mobile-client fingerprint)
    /// use this hook to omit those fields under application identities —
    /// Discord's bot/OAuth2 endpoints reject them with HTTP 403 code
    /// 40333 "internal network error".
    pub fn is_application_token(&self) -> bool {
        is_application_token(&self.credentials().token)
    }

    /// Set a callback to execute when rate limited
    pub fn set_ratelimit_callback(&mut self, callback: std::sync::Arc<dyn Fn(RatelimitInfo) + Send + Sync>) {
        self.ratelimit.callback = Some(callback);
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, route: crate::route::Route<'_>) -> Result<T> {
        self.request(Method::GET, route, None::<()>).await
    }

    /// Make a POST request
    pub async fn post<T: DeserializeOwned, B: Serialize>(&self, route: crate::route::Route<'_>, body: B) -> Result<T> {
        self.request(Method::POST, route, Some(body)).await
    }

    /// Make a PATCH request
    pub async fn patch<T: DeserializeOwned, B: Serialize>(&self, route: crate::route::Route<'_>, body: B) -> Result<T> {
        self.request(Method::PATCH, route, Some(body)).await
    }

    /// Make a PUT request
    pub async fn put<T: DeserializeOwned, B: Serialize>(&self, route: crate::route::Route<'_>, body: B) -> Result<T> {
        self.request(Method::PUT, route, Some(body)).await
    }

    /// Make a DELETE request
    pub async fn delete(&self, route: crate::route::Route<'_>) -> Result<()> {
        self.request_no_response(Method::DELETE, route, None::<()>).await
    }

    /// Make a DELETE request and deserialize the response body.
    pub async fn delete_with_response<T: DeserializeOwned>(&self, route: crate::route::Route<'_>) -> Result<T> {
        self.request(Method::DELETE, route, None::<()>).await
    }

    /// Make a POST request with no body and no response body (e.g. typing
    /// indicator)
    pub async fn post_empty(&self, route: crate::route::Route<'_>) -> Result<()> {
        self.request_no_response(Method::POST, route, None::<()>).await
    }

    /// Make a PUT request with no body and no response body (e.g. add
    /// reaction — `PUT /channels/{c}/messages/{m}/reactions/{e}/@me` returns
    /// 204 with no payload).
    pub async fn put_empty(&self, route: crate::route::Route<'_>) -> Result<()> {
        self.request_no_response(Method::PUT, route, None::<()>).await
    }

    /// Make a POST request with a body but discard the response body.
    pub async fn post_no_response<B: Serialize>(&self, route: crate::route::Route<'_>, body: B) -> Result<()> {
        self.request_no_response(Method::POST, route, Some(body)).await
    }

    /// Make a PATCH request with a body but discard the response body.
    pub async fn patch_no_response<B: Serialize>(&self, route: crate::route::Route<'_>, body: B) -> Result<()> {
        self.request_no_response(Method::PATCH, route, Some(body)).await
    }

    /// Make a POST request with custom Referer header
    pub async fn post_with_referer<T: DeserializeOwned, B: Serialize>(&self, route: crate::route::Route<'_>, body: B, referer: &str) -> Result<T> {
        self.request_with_referer(Method::POST, route, Some(body), Some(referer)).await
    }

    /// POST a multipart message with file attachments.
    ///
    /// `payload_json` carries the message body (content, embeds, etc.).
    /// Each attachment is sent as a `files[N]` part.
    pub async fn post_multipart<T: DeserializeOwned>(&self, route: crate::route::Route<'_>, payload_json: serde_json::Value, attachments: Vec<crate::types::CreateAttachment>) -> Result<T> {
        use reqwest::multipart::{Form, Part};

        let path_cow = route.path();
        let endpoint = path_cow.as_ref();
        let url = self.url_for(endpoint);
        let route_key = Ratelimit::get_route_key(&Method::POST, endpoint);

        for attempt in 0..DEFAULT_RETRY_COUNT {
            if !self.ratelimiter_disabled {
                drop(self.ratelimit.global.lock().await);
                self.ratelimit.pre_hook(&Method::POST, endpoint, &route_key).await;
            }

            let mut form = Form::new().part("payload_json", Part::text(payload_json.to_string()).mime_str("application/json").unwrap_or_else(|_| Part::text(payload_json.to_string())));

            for (i, att) in attachments.iter().enumerate() {
                let part = Part::bytes(att.data.clone()).file_name(att.filename.clone()).mime_str(&att.mime_type).unwrap_or_else(|_| Part::bytes(att.data.clone()).file_name(att.filename.clone()));
                form = form.part(format!("files[{}]", i), part);
            }

            let creds = self.credentials();
            let mut request = self.client.post(&url).multipart(form);
            request = Self::apply_common_headers(request, &creds, self.fingerprint(), None);
            request = Self::apply_custom_headers(request, &creds, true);

            debug!("Multipart request attempt {}/{}: POST {}", attempt + 1, DEFAULT_RETRY_COUNT, url);
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !self.ratelimiter_disabled {
                        self.ratelimit.post_hook(&route_key, response.headers());
                    }
                    if status.is_success() {
                        let text = response.text().await?;
                        return parse_body(&text);
                    }
                    if status.as_u16() == 429 {
                        // Cloudflare 1015 budget tracker.
                        self.invalid_requests.record().await?;

                        let info = Self::extract_rate_limit_info(response.headers());
                        if !self.ratelimiter_disabled {
                            self.ratelimit.record_429(&route_key, response.headers(), info.retry_after);
                        }
                        if !self.resend.resends_after_rate_limit() {
                            return Err(DiscordError::RateLimited { retry_after: info.retry_after, bucket: info.bucket, global: info.global, scope: info.scope });
                        }
                        let wait_time = if info.retry_after < 2.0 { 2.0 } else { info.retry_after };
                        debug!("Rate limited on multipart, waiting {:.2}s", wait_time);
                        tokio::time::sleep(Duration::from_secs_f64(wait_time)).await;
                        continue;
                    }
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        self.invalid_requests.record().await?;
                    }
                    let error_body = response.text().await.unwrap_or_default();
                    return Err(if status.is_server_error() { DiscordError::ServiceError { status: status.as_u16(), body: error_body } } else { DiscordError::UnexpectedStatusCode { status: status.as_u16(), body: error_body } });
                }
                // A POST whose outcome is unknown: it may have been posted, so
                // it is never sent again (`ResendPolicy`).
                Err(e) => return Err(DiscordError::Http(e)),
            }
        }
        Err(DiscordError::MaxRetriesExceeded)
    }

    /// POST a pre-built multipart form to a path string and deserialize the
    /// response.
    ///
    /// Use this for endpoints like sticker creation that need custom multipart
    /// fields (not the `files[N]` + `payload_json` structure of message
    /// attachments).
    pub async fn post_raw_multipart<T: DeserializeOwned>(&self, path: String, form: reqwest::multipart::Form) -> Result<T> {
        let url = self.url_for(&path);
        let route_key = Ratelimit::get_route_key(&Method::POST, &path);

        // Form is consumed on first use; rebuild is not possible, so we
        // accept a single attempt (multipart bodies cannot be cloned cheaply).
        if !self.ratelimiter_disabled {
            drop(self.ratelimit.global.lock().await);
            self.ratelimit.pre_hook(&Method::POST, &path, &route_key).await;
        }

        let creds = self.credentials();
        let mut request = self.client.post(&url).multipart(form);
        request = Self::apply_common_headers(request, &creds, self.fingerprint(), None);
        request = Self::apply_custom_headers(request, &creds, true);
        debug!("Raw multipart POST: {}", url);
        let response = request.send().await?;
        let status = response.status();
        if !self.ratelimiter_disabled {
            self.ratelimit.post_hook(&route_key, response.headers());
        }
        if status.is_success() {
            let text = response.text().await?;
            return parse_body(&text);
        }
        let code = status.as_u16();
        if code == 401 || code == 403 || code == 429 {
            self.invalid_requests.record().await?;
            if code == 429 && !self.ratelimiter_disabled {
                let info = Self::extract_rate_limit_info(response.headers());
                self.ratelimit.record_429(&route_key, response.headers(), info.retry_after);
            }
        }
        let error_body = response.text().await.unwrap_or_default();
        Err(if status.is_server_error() { DiscordError::ServiceError { status: status.as_u16(), body: error_body } } else { DiscordError::UnexpectedStatusCode { status: status.as_u16(), body: error_body } })
    }

    /// Make a request and return the response
    async fn request<T: DeserializeOwned, B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>) -> Result<T> {
        self.request_with_referer(method, route, body, None).await
    }

    /// Make a request with optional referer header
    async fn request_with_referer<T: DeserializeOwned, B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>, referer: Option<&str>) -> Result<T> {
        let response = self.do_request_with_referer(method, route, body, referer, None).await?;
        let text = response.text().await?;
        parse_body(&text)
    }

    /// Make a request without expecting a response body
    async fn request_no_response<B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>) -> Result<()> {
        self.do_request_with_referer(method, route, body, None, None).await?;
        Ok(())
    }

    /// Make a request that may return either a JSON body or 204 No Content.
    /// Returns `Ok(None)` on 204 (or empty body); otherwise parses the body
    /// as `T` and returns `Ok(Some(_))`.
    pub async fn request_optional<T: DeserializeOwned, B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>) -> Result<Option<T>> {
        let response = self.do_request_with_referer(method, route, body, None, None).await?;
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let text = response.text().await?;
        if text.is_empty() {
            return Ok(None);
        }
        serde_json::from_str(&text).map(Some).map_err(|e| {
            error!(error = %e, body = %text, "Failed to parse response");
            DiscordError::Json(e)
        })
    }

    /// PUT variant of [`request_optional`] for the common `add_guild_member`
    /// shape (201 with `Member`, or 204 if already a member).
    pub async fn put_optional<T: DeserializeOwned, B: Serialize>(&self, route: crate::route::Route<'_>, body: B) -> Result<Option<T>> {
        self.request_optional(Method::PUT, route, Some(body)).await
    }

    /// GET request returning the raw response bytes. Used for binary
    /// endpoints such as guild widget PNGs and CDN downloads.
    pub async fn get_bytes(&self, route: crate::route::Route<'_>) -> Result<Vec<u8>> {
        let response = self.do_request_with_referer(Method::GET, route, None::<()>, None, None).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Make a request with an `X-Audit-Log-Reason` header.
    ///
    /// Discord audit-log reasons are surfaced in the audit log of the target
    /// guild. The header value must be URL-encoded; this method handles the
    /// encoding so callers pass the raw reason string. Pass `None` to fall
    /// back to the standard request path with no reason header — semantics
    /// are identical to [`request`].
    pub async fn request_with_reason<T: DeserializeOwned, B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>, reason: Option<&str>) -> Result<T> {
        let response = self.do_request_with_referer(method, route, body, None, reason).await?;
        let text = response.text().await?;
        parse_body(&text)
    }

    /// Variant of [`request_with_reason`] that ignores the response body.
    pub async fn request_with_reason_no_response<B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>, reason: Option<&str>) -> Result<()> {
        self.do_request_with_referer(method, route, body, None, reason).await?;
        Ok(())
    }

    /// Compute the value to send in the `Authorization` header.
    ///
    /// Discord accepts three token shapes:
    /// - `Bot <token>` — application bot tokens
    /// - `Bearer <token>` — OAuth2 access tokens
    /// - `<token>` — user / self-bot tokens (bare)
    ///
    /// If the configured token already carries a `Bot `/`Bearer ` prefix we
    /// send it verbatim; otherwise we treat it as a user token and send it
    /// bare. The result is the same (we always send `self.token`) but the
    /// branches make the contract explicit and give us a hook for future
    /// per-flavor behaviour (e.g. dropping super-properties on bot tokens).
    fn auth_header_value(token: &str) -> &str {
        token
    }

    /// The client-fingerprint headers a user token carries, as configured.
    fn fingerprint(&self) -> Fingerprint<'_> {
        Fingerprint { super_properties_b64: self.super_properties_b64.as_deref(), locale: self.discord_locale.as_deref(), timezone: self.discord_timezone.as_deref() }
    }

    /// Apply Discord-specific shared headers (auth, super-properties,
    /// locale, timezone, and optionally the audit-log reason) to a
    /// `RequestBuilder`.
    ///
    /// Header set differs by token flavour:
    ///
    /// - **User / self-bot tokens** carry the full client fingerprint
    ///   (`X-Super-Properties`, `X-Discord-Locale`, `X-Discord-Timezone`)
    ///   plus the Chrome `User-Agent` baked into the client builder, so the
    ///   gateway and REST traffic look like a real Discord client.
    /// - **Bot / Bearer tokens** suppress every client-fingerprint header
    ///   AND override `User-Agent` to the Discord-mandated
    ///   `DiscordBot (URL, version)` form. Sending Chrome-style headers
    ///   under a bot token routes the request into Discord's Cloudflare WAF
    ///   and returns HTTP 403 with body `{"message":"internal network error","code":40333}`
    ///   — visible to operators as "Missing permission" even when the bot
    ///   has Administrator.
    fn apply_common_headers(mut request: reqwest::RequestBuilder, creds: &Credentials, fingerprint: Fingerprint<'_>, reason: Option<&str>) -> reqwest::RequestBuilder {
        // Authorization. Bot/Bearer tokens are passed through verbatim;
        // user tokens stay bare. Identical wire shape; the predicate below
        // gates the rest of the header set.
        request = request.header(header::AUTHORIZATION, Self::auth_header_value(&creds.token));

        if is_application_token(&creds.token) {
            // Bot path: override User-Agent and skip the fingerprint headers.
            request = request.header(header::USER_AGENT, concat!("DiscordBot (https://github.com/lengoclongk59/discord-user-rs, ", env!("CARGO_PKG_VERSION"), ")"));
        } else {
            // User-token path: full client fingerprint.
            if let Some(sp) = fingerprint.super_properties_b64 {
                request = request.header("X-Super-Properties", sp);
            }
            if let Some(loc) = fingerprint.locale {
                request = request.header("X-Discord-Locale", loc);
            }
            if let Some(tz) = fingerprint.timezone {
                request = request.header("X-Discord-Timezone", tz);
            }
        }

        if let Some(raw_reason) = reason {
            // Discord requires the reason be URL-encoded so non-ASCII and
            // header-illegal characters survive the wire.
            let encoded = urlencoding::encode(raw_reason).into_owned();
            request = request.header("X-Audit-Log-Reason", encoded);
        }
        request
    }

    /// Internal request method with retry logic and optional referer
    async fn do_request_with_referer<B: Serialize>(&self, method: Method, route: crate::route::Route<'_>, body: Option<B>, referer: Option<&str>, reason: Option<&str>) -> Result<Response> {
        let path_cow = route.path();
        let endpoint = path_cow.as_ref();
        let url = self.url_for(endpoint);
        let route_key = Ratelimit::get_route_key(&method, endpoint);

        for attempt in 0..DEFAULT_RETRY_COUNT {
            if !self.ratelimiter_disabled {
                // Wait for any active global rate limits to expire
                drop(self.ratelimit.global.lock().await);

                self.ratelimit.pre_hook(&method, endpoint, &route_key).await;
            }

            let creds = self.credentials();
            let mut request = self.client.request(method.clone(), &url);

            // Add authorization, super-properties, locale, timezone, and
            // optional audit-log reason headers.
            request = Self::apply_common_headers(request, &creds, self.fingerprint(), reason);

            // Add referer header if provided
            if let Some(ref_url) = referer {
                request = request.header(header::REFERER, ref_url);
            }

            // Add custom headers
            request = Self::apply_custom_headers(request, &creds, false);

            // Add body if present
            if let Some(ref b) = body {
                request = request.json(b);
            }

            debug!("Request attempt {}/{}: {} {}", attempt + 1, DEFAULT_RETRY_COUNT, method, url);

            let result = request.send().await;

            match result {
                Ok(response) => {
                    let status = response.status();
                    if !self.ratelimiter_disabled {
                        self.ratelimit.post_hook(&route_key, response.headers());
                    }

                    if status.is_success() {
                        return Ok(response);
                    }

                    // Handle rate limiting
                    if status.as_u16() == 429 {
                        // Cloudflare 1015 budget: 401 / 403 / 429 each
                        // count toward the 10k/10min ban window.
                        self.invalid_requests.record().await?;

                        let info = Self::extract_rate_limit_info(response.headers());

                        // Refresh the bucket cache so the next pre_hook
                        // honours the new reset timestamp.  Without this,
                        // a 429 with a known retry-after would be
                        // forgotten the moment we leave this function and
                        // future calls on the same route would re-spam
                        // the bucket.
                        if !self.ratelimiter_disabled {
                            self.ratelimit.record_429(&route_key, response.headers(), info.retry_after);
                        }

                        if self.ratelimiter_disabled || !self.resend.resends_after_rate_limit() {
                            return Err(DiscordError::RateLimited { retry_after: info.retry_after, bucket: info.bucket, global: info.global, scope: info.scope });
                        }

                        if info.retry_after > MAX_RETRY_AFTER_SECONDS as f64 {
                            return Err(DiscordError::RateLimited { retry_after: info.retry_after, bucket: info.bucket, global: info.global, scope: info.scope });
                        }

                        // Enforce minimum wait time of 2.0s to avoid spamming if server sends 0 or
                        // missing header
                        let wait_time = if info.retry_after < 2.0 { 2.0 } else { info.retry_after };
                        debug!("Rate limited (global: {}), waiting {:.2} seconds", info.global, wait_time);

                        // If it's a global rate limit, acquire the global lock across all routes
                        // This prevents any other requests from proceeding while we sleep
                        let _global_guard = if info.global { Some(self.ratelimit.global.lock().await) } else { None };

                        if let Some(cb) = &self.ratelimit.callback {
                            let limit = response.headers().get("x-ratelimit-limit").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

                            cb(RatelimitInfo {
                                timeout: Duration::from_secs_f64(wait_time),
                                limit,
                                method: method.clone(),
                                path: endpoint.to_string(),
                                global: info.global,
                            });
                        }

                        tokio::time::sleep(Duration::from_secs_f64(wait_time)).await;
                        // global_guard is dropped here so other routes can proceed
                        continue;
                    }

                    // Cloudflare 1015 budget: track 401 / 403 alongside 429
                    // so callers can throttle voluntarily before the ban hits.
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        self.invalid_requests.record().await?;
                    }

                    // Parse error response body; if reading fails, surface the read error directly
                    let error_body = match response.text().await {
                        Ok(text) => text,
                        Err(e) => {
                            return Err(DiscordError::Http(e));
                        }
                    };

                    // Diagnostic: log every non-success, non-rate-limit Discord response
                    // so we can see exactly what the API replied with when classification
                    // happens.
                    warn!(
                        discord.status = status.as_u16(),
                        discord.method = %method,
                        discord.url = %url,
                        discord.body_len = error_body.len(),
                        discord.body = %error_body.chars().take(500).collect::<String>(),
                        "Discord API non-success response"
                    );

                    // Check for specific error messages
                    if error_body.contains("verify your account") {
                        return Err(DiscordError::VerificationRequired);
                    }

                    if error_body.contains("captcha_key") {
                        let service = serde_json::from_str::<serde_json::Value>(&error_body).ok().and_then(|v| v["captcha_service"].as_str().map(|s| s.to_string())).unwrap_or_else(|| "unknown".to_string());
                        return Err(DiscordError::CaptchaRequired { service });
                    }

                    if status.as_u16() == 401 {
                        // Check if the error body indicates an invalid token
                        if error_body.contains("401: Unauthorized") || error_body.contains("token") {
                            warn!(discord.body = %error_body, "Mapped 401 -> InvalidToken");
                            return Err(DiscordError::InvalidToken);
                        }
                        warn!(discord.body = %error_body, "Mapped 401 -> AuthenticationFailed");
                        return Err(DiscordError::AuthenticationFailed);
                    }

                    // Handle 403 Forbidden - permission denied
                    if status.as_u16() == 403 {
                        let permission = serde_json::from_str::<serde_json::Value>(&error_body).ok().and_then(|v| v["message"].as_str().map(|s| s.to_string())).unwrap_or_else(|| "unknown".to_string());
                        return Err(DiscordError::PermissionDenied { permission });
                    }

                    // Handle 404 Not Found
                    if status.as_u16() == 404 {
                        // Try to extract resource info from the URL
                        let resource_type = Self::extract_resource_type(&url);
                        let id = Self::extract_resource_id(&url);
                        return Err(DiscordError::NotFound { resource_type, id });
                    }

                    // Handle 400 Bad Request
                    if status.as_u16() == 400 {
                        return Err(DiscordError::InvalidRequest(error_body));
                    }

                    error!(status = %status, error_body = %error_body, "HTTP error");

                    // Retry on server errors, when the outcome may be repeated
                    if status.is_server_error() && attempt < DEFAULT_RETRY_COUNT - 1 && self.resend.resends_after_unknown_outcome(&method) {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }

                    if status.is_server_error() {
                        return Err(DiscordError::ServiceError { status: status.as_u16(), body: error_body });
                    }

                    return Err(DiscordError::UnexpectedStatusCode { status: status.as_u16(), body: error_body });
                }
                Err(e) => {
                    error!(error = %e, "Request error");
                    if attempt < DEFAULT_RETRY_COUNT - 1 && self.resend.resends_after_unknown_outcome(&method) {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(DiscordError::Http(e));
                }
            }
        }

        Err(DiscordError::MaxRetriesExceeded)
    }

    /// Extract rate limit info from headers
    fn extract_rate_limit_info(headers: &header::HeaderMap) -> RateLimitInfo {
        let retry_after = headers.get("retry-after").and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(5.0);

        let bucket = headers.get("x-ratelimit-bucket").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

        let global = headers.get("x-ratelimit-global").and_then(|h| h.to_str().ok()).map(|s| s == "true").unwrap_or(false);

        let scope = headers.get("x-ratelimit-scope").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

        RateLimitInfo { retry_after, bucket, global, scope }
    }

    /// Extract resource type from Discord API URL
    fn extract_resource_type(url: &str) -> String {
        // Parse URL to find resource type (channels, guilds, users, messages, etc.)
        let parts: Vec<&str> = url.split('/').collect();
        for part in &parts {
            match *part {
                "channels" => return "channel".to_string(),
                "guilds" => return "guild".to_string(),
                "users" => return "user".to_string(),
                "messages" => return "message".to_string(),
                "members" => return "member".to_string(),
                "roles" => return "role".to_string(),
                "invites" => return "invite".to_string(),
                "webhooks" => return "webhook".to_string(),
                "emojis" => return "emoji".to_string(),
                _ => continue,
            }
        }
        "resource".to_string()
    }

    /// Extract resource ID from Discord API URL
    fn extract_resource_id(url: &str) -> String {
        // Find the last numeric ID in the URL path
        let parts: Vec<&str> = url.split('/').collect();
        for part in parts.iter().rev() {
            // Discord IDs are snowflakes (large integers)
            if part.chars().all(|c| c.is_ascii_digit()) && part.len() > 10 {
                return (*part).to_string();
            }
        }
        "unknown".to_string()
    }
}

/// The client-fingerprint headers a user-token request carries.
#[derive(Clone, Copy)]
struct Fingerprint<'a> {
    super_properties_b64: Option<&'a str>,
    locale: Option<&'a str>,
    timezone: Option<&'a str>,
}

/// A bot (`Bot `) or OAuth2 (`Bearer `) token, as opposed to a user token.
fn is_application_token(token: &str) -> bool {
    let t = token.trim_start();
    t.starts_with("Bot ") || t.starts_with("Bearer ")
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn test_extract_rate_limit_info() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("12.5"));
        headers.insert("x-ratelimit-bucket", HeaderValue::from_static("test-bucket"));
        headers.insert("x-ratelimit-global", HeaderValue::from_static("true"));
        headers.insert("x-ratelimit-scope", HeaderValue::from_static("shared"));

        let info = DiscordHttpClient::extract_rate_limit_info(&headers);

        assert_eq!(info.retry_after, 12.5);
        assert_eq!(info.bucket.unwrap(), "test-bucket");
        assert!(info.global);
        assert_eq!(info.scope.unwrap(), "shared");
    }

    #[test]
    fn test_extract_rate_limit_info_defaults() {
        let headers = HeaderMap::new();
        let info = DiscordHttpClient::extract_rate_limit_info(&headers);

        assert_eq!(info.retry_after, 5.0);
        assert!(info.bucket.is_none());
        assert!(!info.global);
        assert!(info.scope.is_none());
    }

    #[tokio::test]
    async fn invalid_request_tracker_counts_and_prunes() {
        let tracker = InvalidRequestTracker::default();
        assert_eq!(tracker.count().await, 0);
        let n1 = tracker.record().await;
        let n2 = tracker.record().await;
        assert_eq!(n1, 1);
        assert_eq!(n2, 2);
        assert_eq!(tracker.count().await, 2);
    }

    #[test]
    fn the_default_policy_resends_only_what_cannot_be_carried_out_twice() {
        let policy = ResendPolicy::default();
        assert_eq!(policy, ResendPolicy::IdempotentOnly);
        assert!(policy.resends_after_rate_limit());
        for method in [Method::GET, Method::HEAD, Method::OPTIONS, Method::PUT, Method::DELETE] {
            assert!(policy.resends_after_unknown_outcome(&method), "{method}");
        }
        for method in [Method::POST, Method::PATCH] {
            assert!(!policy.resends_after_unknown_outcome(&method), "{method}");
        }
    }

    #[test]
    fn the_never_policy_resends_nothing() {
        let policy = ResendPolicy::Never;
        assert!(!policy.resends_after_rate_limit());
        for method in [Method::GET, Method::PUT, Method::DELETE, Method::POST, Method::PATCH] {
            assert!(!policy.resends_after_unknown_outcome(&method), "{method}");
        }
    }

    #[test]
    fn an_empty_body_reads_as_null() {
        parse_body::<()>("").unwrap();
        parse_body::<()>("  ").unwrap();
        assert_eq!(parse_body::<Option<u8>>("").unwrap(), None);
        assert_eq!(parse_body::<u8>("7").unwrap(), 7);
        assert!(parse_body::<u8>("").is_err());
    }

    #[test]
    fn a_replaced_credential_reaches_every_clone() {
        let client = DiscordHttpClient::new("first", None, false);
        let clone = client.clone();
        client.set_token("second");
        assert_eq!(clone.token(), "second");

        let headers = HashMap::from([("Authorization".to_string(), "third".to_string()), ("cookie".to_string(), "a=b".to_string())]);
        assert!(clone.replace_headers(headers));
        assert_eq!(client.token(), "third");
        assert_eq!(client.credentials().headers.get("cookie").map(String::as_str), Some("a=b"));

        assert!(!client.replace_headers(HashMap::from([("cookie".to_string(), "c=d".to_string())])));
        assert_eq!(client.token(), "third", "a refused replacement changes nothing");
    }

    #[test]
    fn a_relative_route_goes_to_the_configured_base() {
        let client = DiscordHttpClient::new("t", None, false).with_api_base("http://127.0.0.1:9/api/v10/");
        assert_eq!(client.url_for("channels/1/messages"), "http://127.0.0.1:9/api/v10/channels/1/messages");
        assert_eq!(client.url_for("https://discord.com/api/v9/interactions"), "https://discord.com/api/v9/interactions");
        assert_eq!(DiscordHttpClient::new("t", None, false).url_for("/users/@me"), format!("{API_BASE}/users/@me"));
    }

    #[test]
    fn record_429_seeds_bucket_with_reset() {
        let rl = Ratelimit::default();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("5"));
        headers.insert("x-ratelimit-reset-after", HeaderValue::from_static("3.5"));
        rl.record_429("GET /test", &headers, 1.0);
        let bucket = rl.buckets.get("GET /test").unwrap();
        assert_eq!(bucket.remaining, 0);
        assert_eq!(bucket.limit, 5);
        assert!(bucket.reset_after.unwrap() > 3.0);
    }
}
