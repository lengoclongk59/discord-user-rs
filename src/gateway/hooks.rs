//! Caller policy for an embedded gateway.
//!
//! The library owns the socket, heartbeat, framing, the RESUME URL and
//! decompression. A [`GatewayHooks`] implementation decides the rest: which
//! credential to present, what goes in IDENTIFY, what a close code means, and
//! how long to wait before reconnecting. Every method has a default that
//! reproduces the library's own behaviour, so an implementation overrides only
//! what it needs.

use std::time::Duration;

use futures::future::BoxFuture;
use serde_json::{json, Value};

use super::{client_build_number, machine::gateway_token, plan_reconnect, ReconnectCause};
use crate::types::UserStatus;

/// What to do after the gateway closes the socket with a close frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseVerdict {
    /// Reconnect and RESUME if a session is held. Waits the fast RESUME floor
    /// when the backoff ladder is at its floor.
    Resume,
    /// Drop the session and reconnect with IDENTIFY, after the jittered
    /// IDENTIFY window.
    Identify,
    /// Reconnect on the backoff ladder, keeping the session.
    Retry,
    /// Stop the supervisor. The string is reported in the final
    /// [`GatewayStatus::Stopped`](super::GatewayStatus::Stopped) event.
    Stop(String),
}

/// The library's reading of a Discord close code.
///
/// Codes in the 4000–4999 range are Discord-specific; codes < 4000 are
/// standard WebSocket codes (1000 normal, 1001 going away, 1006 abnormal).
/// See <https://discord.com/developers/docs/topics/opcodes-and-status-codes#gateway-gateway-close-event-codes>.
pub fn default_close_verdict(code: Option<u16>) -> CloseVerdict {
    let stop = |reason: &str| CloseVerdict::Stop(reason.to_string());
    match code {
        // 4000 Unknown error, 4001 Unknown opcode, 4002 Decode error,
        // 4005 Already authenticated — the session survives; RESUME.
        Some(4000 | 4001 | 4002 | 4005) => CloseVerdict::Resume,
        // 4003 Not authenticated — we sent a payload before IDENTIFY; the
        // session is gone.
        Some(4003) => CloseVerdict::Identify,
        Some(4004) => stop("Authentication failed"),
        // 4008 Rate limited — the session survives, but the reconnect waits
        // the backoff ladder rather than the fast RESUME floor.
        Some(4008) => CloseVerdict::Retry,
        // 4007 Invalid seq, 4009 Session timed out — the session is
        // unrecoverable, force re-IDENTIFY.
        Some(4007 | 4009) => CloseVerdict::Identify,
        Some(4010) => stop("Invalid shard"),
        Some(4011) => stop("Sharding required"),
        Some(4012) => stop("Invalid API version"),
        Some(4013) => stop("Invalid intents"),
        Some(4014) => stop("Disallowed intents"),
        Some(_) | None => CloseVerdict::Resume,
    }
}

/// Everything the default IDENTIFY body needs besides the token, as
/// configured on the [`Gateway`](super::Gateway).
#[derive(Debug, Clone)]
pub struct IdentifyDefaults {
    pub custom_status: UserStatus,
    pub capabilities: u32,
    pub intents: Option<u64>,
    pub large_threshold: u16,
    pub shard: Option<[u32; 2]>,
    /// Pinned client build number; `None` fetches it from discord.com once
    /// per process.
    pub client_build_number: Option<u32>,
}

impl IdentifyDefaults {
    /// The library's IDENTIFY `d` object for `token`.
    pub async fn payload(&self, token: &str) -> Value {
        let build_number = match self.client_build_number {
            Some(n) => n,
            None => client_build_number().await,
        };
        identify_payload(token, self.custom_status, self.capabilities, self.intents, self.large_threshold, self.shard, build_number)
    }
}

/// Build the IDENTIFY `d` object.
///
/// `intents` is honoured for bot-token connections — when `Some` we add an
/// `intents` field to the payload alongside the existing `capabilities`
/// field used for user tokens. `large_threshold` is forwarded as required
/// by the Discord docs (valid range 50–250; clamped here as belt-and-braces).
// TODO: route `large_threshold` through `DiscordUserBuilder` so callers can
// tune it without touching `Gateway` directly.
fn identify_payload(token: &str, status: UserStatus, capabilities: u32, intents: Option<u64>, large_threshold: u16, shard: Option<[u32; 2]>, build_number: u32) -> Value {
    let large_threshold = large_threshold.clamp(50, 250);
    let mut data = json!({
        "token": gateway_token(token),
        "capabilities": capabilities,
        "large_threshold": large_threshold,
        "properties": {
            "os": "Windows",
            "browser": "Chrome",
            "device": "",
            "system_locale": "en-US",
            "browser_user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "browser_version": "120.0.0.0",
            "os_version": "10",
            "referrer": "",
            "referring_domain": "",
            "referrer_current": "",
            "referring_domain_current": "",
            "release_channel": "stable",
            "client_build_number": build_number,
            "client_event_source": null
        },
        "presence": {
            "status": status.as_str(),
            "since": 0,
            "activities": [],
            "afk": false
        },
        "compress": false,
        "client_state": {
            "guild_versions": {},
            "highest_last_message_id": "0",
            "read_state_version": 0,
            "user_guild_settings_version": -1,
            "private_channels_version": "0",
            "api_code_version": 0
        }
    });

    // Bot-token mode: include `intents` alongside `capabilities`. User-token
    // mode (intents == None) keeps the existing payload shape unchanged.
    if let Some(intents) = intents {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("intents".to_string(), Value::from(intents));
        }
    }

    // Sharding: Discord requires sharding once a bot reaches 2 500 guilds and
    // emits close code 4011 ("sharding required") if absent. Shape: [id, count].
    if let Some([shard_id, shard_count]) = shard {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("shard".to_string(), json!([shard_id, shard_count]));
        }
    }

    data
}

/// Caller policy for a [`Gateway`](super::Gateway). Install one with
/// [`Gateway::with_hooks`](super::Gateway::with_hooks).
///
/// Every method has a default that reproduces the library's behaviour. The
/// supervisor calls the hooks from its own task, so a slow hook delays the
/// connection it is deciding about, and nothing else.
///
/// ```no_run
/// use std::sync::Arc;
///
/// use discord_user::gateway::{default_close_verdict, BoxFuture, CloseVerdict, FrameBuffer, Gateway, GatewayEvent, GatewayHooks};
/// use discord_user::UserStatus;
///
/// /// Re-reads the token on every connect, and asks for a fresh one after a
/// /// 4004 instead of stopping.
/// struct EnvToken;
///
/// impl GatewayHooks for EnvToken {
///     fn token<'a>(&'a self, _configured: &'a str) -> BoxFuture<'a, Option<String>> {
///         Box::pin(async { std::env::var("DISCORD_TOKEN").ok() })
///     }
///
///     fn on_close(&self, code: Option<u16>, _reason: &str) -> CloseVerdict {
///         match code {
///             Some(4004) => CloseVerdict::Identify,
///             other => default_close_verdict(other),
///         }
///     }
/// }
///
/// # async fn run() -> discord_user::Result<()> {
/// let (gateway, _dispatches) = Gateway::new(String::new(), UserStatus::Online, 256);
/// let mut gateway = gateway.with_hooks(Arc::new(EnvToken));
/// let mut events = gateway.frames(FrameBuffer::Bounded(4096)).expect("first call");
/// gateway.connect().await?;
/// while let Some(event) = events.recv().await {
///     match event {
///         GatewayEvent::Frame { op, s, raw, .. } => println!("op {op} s {s:?}: {} bytes", raw.len()),
///         GatewayEvent::Status { status, detail } => println!("{status:?} {detail:?}"),
///         GatewayEvent::Lagged { dropped } => eprintln!("dropped {dropped} events"),
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub trait GatewayHooks: Send + Sync + 'static {
    /// The credential for the next connection, asked for before every
    /// connect. `configured` is the token the gateway was built with.
    ///
    /// `None` means "no credential right now": the supervisor waits the
    /// backoff ladder and asks again; it does not stop. When the answer
    /// differs from the token of a held session, the session is dropped and
    /// the next connection IDENTIFYs, since a RESUME with another token would
    /// only earn an op 9.
    fn token<'a>(&'a self, configured: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move { Some(configured.to_string()) })
    }

    /// The current credential was rejected (close 4004). Called before
    /// [`Self::on_close`], so a caller that re-harvests can drop its cache
    /// here and return [`CloseVerdict::Identify`] from `on_close`.
    fn invalidate(&self) {}

    /// The IDENTIFY `d` object for `token` (the value returned by
    /// [`Self::token`], prefix included). `defaults` builds the library's
    /// own payload.
    fn identify<'a>(&'a self, token: &'a str, defaults: &'a IdentifyDefaults) -> BoxFuture<'a, Value> {
        Box::pin(defaults.payload(token))
    }

    /// What to do after a close frame. Defaults to [`default_close_verdict`].
    fn on_close(&self, code: Option<u16>, reason: &str) -> CloseVerdict {
        let _ = reason;
        default_close_verdict(code)
    }

    /// The wait before the next connect and the backoff to carry forward.
    /// Defaults to [`plan_reconnect`]. The supervisor resets the backoff to
    /// [`BACKOFF_MIN`](super::BACKOFF_MIN) once a connection has had a
    /// heartbeat acknowledged after READY or RESUMED.
    fn delay(&self, cause: ReconnectCause, backoff: Duration, session_held: bool, rand: f64) -> (Duration, Duration) {
        plan_reconnect(cause, backoff, session_held, rand)
    }
}

/// The library's own policy: every [`GatewayHooks`] default.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultHooks;

impl GatewayHooks for DefaultHooks {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_codes_classify() {
        for code in [4000, 4001, 4002, 4005, 1000, 1001, 1006, 4999] {
            assert_eq!(default_close_verdict(Some(code)), CloseVerdict::Resume, "code {code}");
        }
        assert_eq!(default_close_verdict(None), CloseVerdict::Resume);
        assert_eq!(default_close_verdict(Some(4008)), CloseVerdict::Retry);
        for code in [4003, 4007, 4009] {
            assert_eq!(default_close_verdict(Some(code)), CloseVerdict::Identify, "code {code}");
        }
        assert_eq!(default_close_verdict(Some(4004)), CloseVerdict::Stop("Authentication failed".to_string()));
        for code in [4010, 4011, 4012, 4013, 4014] {
            assert!(matches!(default_close_verdict(Some(code)), CloseVerdict::Stop(_)), "code {code}");
        }
    }

    #[test]
    fn identify_strips_the_token_prefix_and_keeps_the_default_fingerprint() {
        let d = identify_payload("Bot abc", UserStatus::Online, 16381, None, 500, None, 7);
        assert_eq!(d["token"], "abc");
        assert_eq!(d["capabilities"], 16381);
        assert_eq!(d["large_threshold"], 250);
        assert_eq!(d["properties"]["os"], "Windows");
        assert_eq!(d["properties"]["browser_version"], "120.0.0.0");
        assert_eq!(d["properties"]["client_build_number"], 7);
        assert!(d.get("intents").is_none());
        assert!(d.get("shard").is_none());
    }

    #[test]
    fn identify_adds_intents_and_shard_when_set() {
        let d = identify_payload("abc", UserStatus::Online, 0, Some(513), 50, Some([1, 4]), 7);
        assert_eq!(d["intents"], 513);
        assert_eq!(d["shard"], json!([1, 4]));
    }

    #[tokio::test]
    async fn default_hooks_reproduce_the_library_behaviour() {
        let hooks = DefaultHooks;
        assert_eq!(hooks.token("tok").await.as_deref(), Some("tok"));
        let defaults = IdentifyDefaults { custom_status: UserStatus::Online, capabilities: 16381, intents: None, large_threshold: 50, shard: None, client_build_number: Some(7) };
        assert_eq!(hooks.identify("tok", &defaults).await, identify_payload("tok", UserStatus::Online, 16381, None, 50, None, 7));
        assert_eq!(hooks.on_close(Some(4009), "x"), CloseVerdict::Identify);
        let s = Duration::from_secs;
        assert_eq!(hooks.delay(ReconnectCause::Failure, s(4), true, 0.5), (s(4), s(8)));
    }
}
