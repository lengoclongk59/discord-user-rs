//! Per-connection gateway state machine, free of I/O.
//!
//! [`GatewayCore`] turns inbound frames and clock ticks into [`Action`]s for the
//! supervisor in `gateway.rs` to carry out. It never touches the socket, never
//! reads a clock and never draws a random number: the supervisor passes `now`
//! and `rand` in with every input. That is what lets the unit tests at the
//! bottom of this file drive every transition with exact timings and no socket.

use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tracing::{debug, info, warn};

use super::{GatewayPayload, Opcode};
use crate::types::ConnectionStage;

/// Heartbeat interval assumed before HELLO arrives (Discord's usual value).
pub(crate) const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(41_250);

/// Cap on a HELLO-supplied interval, so a malformed HELLO cannot stretch the
/// read timeout without bound.
const MAX_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(300);

/// Floor of the reconnect backoff ladder.
pub const BACKOFF_MIN: Duration = Duration::from_secs(1);
/// Ceiling of the reconnect backoff ladder.
pub const BACKOFF_MAX: Duration = Duration::from_secs(60);
/// Wait before a RESUME that Discord itself asked for, when the ladder is at
/// its floor. Not zero: a connection that is closed straight after opening
/// must not spin.
pub const FAST_RECONNECT: Duration = Duration::from_millis(250);
/// Window Discord specifies before re-IDENTIFYing after a session is lost.
/// IDENTIFY is budgeted per day, so this wait is kept and jittered.
pub const INVALID_SESSION_MIN: Duration = Duration::from_secs(1);
/// Upper end of the IDENTIFY window; see [`INVALID_SESSION_MIN`].
pub const INVALID_SESSION_MAX: Duration = Duration::from_secs(5);

/// Why a connection is being replaced. Drives how long the supervisor waits
/// before the next connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectCause {
    /// Discord asked this connection to move (op 7, a resumable op 9), or
    /// closed it with a code that keeps the session. The next connect RESUMEs
    /// if a session is held.
    Resume,
    /// The session is gone; the next connect must IDENTIFY.
    Identify,
    /// An ordinary failure: a dropped socket, an unacknowledged heartbeat, a
    /// read timeout, a refused connect.
    Failure,
}

/// One reconnect's wait, and the backoff to carry into the next one.
///
/// The ladder advances on every cause, so a burst of reconnects slows down
/// whatever caused them. Only the caller resets it, once a connection has
/// proven stable.
///
/// - [`ReconnectCause::Resume`] with a session held, while the ladder is at
///   its floor: [`FAST_RECONNECT`]. A RESUME costs no IDENTIFY.
/// - [`ReconnectCause::Identify`]: a jittered wait in
///   [`INVALID_SESSION_MIN`]..[`INVALID_SESSION_MAX`], or the current backoff
///   if that is longer.
/// - Anything else: the current backoff.
///
/// `rand` is a draw in `0.0..=1.0`, passed in so the result is deterministic.
pub fn plan_reconnect(cause: ReconnectCause, backoff: Duration, session_held: bool, rand: f64) -> (Duration, Duration) {
    let next = (backoff * 2).min(BACKOFF_MAX);
    let wait = match cause {
        ReconnectCause::Identify => {
            let span = INVALID_SESSION_MAX - INVALID_SESSION_MIN;
            (INVALID_SESSION_MIN + span.mul_f64(rand.clamp(0.0, 1.0))).max(backoff)
        }
        ReconnectCause::Resume if session_held && backoff <= BACKOFF_MIN => FAST_RECONNECT,
        ReconnectCause::Resume | ReconnectCause::Failure => backoff,
    };
    (wait, next)
}

/// One thing the supervisor must do on the core's behalf.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Action {
    /// Write this text frame to the socket.
    Send(String),
    /// Build an IDENTIFY payload and send it. The payload needs the client
    /// build number, which may require a fetch, so the supervisor builds it.
    Identify,
    /// Broadcast a dispatch to [`DispatchEvent`](crate::events::DispatchEvent)
    /// subscribers.
    Dispatch { event_type: String, data: Value },
    /// End this connection and open a new one.
    Reconnect { cause: ReconnectCause, reason: &'static str },
}

/// The state of one gateway session across its connections.
#[derive(Debug)]
pub(crate) struct GatewayCore {
    base_url: String,
    token: String,
    session_id: Option<String>,
    seq: Option<u64>,
    /// READY's `resume_gateway_url`, with the base URL's query appended.
    resume_url: Option<String>,
    stage: ConnectionStage,
    heartbeat_interval: Duration,
    next_heartbeat: Option<Instant>,
    awaiting_ack: bool,
    last_heartbeat_sent: Option<Instant>,
    latency: Option<Duration>,
    /// When the current connection last showed a sign of life (opened, or a
    /// frame arrived). The read timeout counts from here.
    last_frame_at: Option<Instant>,
    /// Guilds from READY still awaiting their GUILD_CREATE.
    pending_guilds: usize,
    /// The current connection reached READY or RESUMED.
    live: bool,
    /// The current connection has had a heartbeat acknowledged since going
    /// live. Only this resets the backoff ladder.
    stable: bool,
    backoff: Duration,
}

impl GatewayCore {
    pub(crate) fn new(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            session_id: None,
            seq: None,
            resume_url: None,
            stage: ConnectionStage::Disconnected,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            next_heartbeat: None,
            awaiting_ack: false,
            last_heartbeat_sent: None,
            latency: None,
            last_frame_at: None,
            pending_guilds: 0,
            live: false,
            stable: false,
            backoff: BACKOFF_MIN,
        }
    }

    pub(crate) fn stage(&self) -> ConnectionStage {
        self.stage
    }

    pub(crate) fn latency(&self) -> Option<Duration> {
        self.latency
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }

    /// Whether a RESUME is possible: both a session id and a sequence are held.
    pub(crate) fn has_session(&self) -> bool {
        self.session_id.is_some() && self.seq.is_some()
    }

    pub(crate) fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// The URL the next connection should open: READY's resume URL while a
    /// session is held, the base URL otherwise.
    pub(crate) fn connect_url(&self) -> &str {
        match &self.resume_url {
            Some(url) if self.has_session() => url,
            _ => &self.base_url,
        }
    }

    /// The supervisor is about to open a new socket.
    pub(crate) fn connecting(&mut self) {
        self.stage = ConnectionStage::Connecting;
    }

    /// A socket is open. Resets the per-connection state and starts the read
    /// timeout.
    pub(crate) fn opened(&mut self, now: Instant) {
        self.reset_connection();
        self.stage = ConnectionStage::Connecting;
        self.last_frame_at = Some(now);
    }

    /// The current connection is over. `clear_session` drops the session, so
    /// the next connection IDENTIFYs instead of RESUMEing.
    pub(crate) fn connection_lost(&mut self, clear_session: bool) {
        self.reset_connection();
        self.stage = ConnectionStage::Disconnected;
        if clear_session {
            self.clear_session();
        }
    }

    /// How long to wait before the next connect, advancing the ladder.
    /// `planner` has the shape of [`plan_reconnect`].
    pub(crate) fn reconnect_delay(&mut self, cause: ReconnectCause, rand: f64, planner: impl FnOnce(ReconnectCause, Duration, bool, f64) -> (Duration, Duration)) -> Duration {
        let (wait, next) = planner(cause, self.backoff, self.has_session(), rand);
        self.backoff = next;
        wait
    }

    fn reset_connection(&mut self) {
        self.next_heartbeat = None;
        self.awaiting_ack = false;
        self.last_heartbeat_sent = None;
        self.last_frame_at = None;
        self.live = false;
        self.stable = false;
    }

    fn clear_session(&mut self) {
        self.session_id = None;
        self.seq = None;
        self.resume_url = None;
    }

    /// The earliest instant at which [`Self::tick`] has work to do.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        [self.read_deadline(), self.next_heartbeat].into_iter().flatten().min()
    }

    fn read_deadline(&self) -> Option<Instant> {
        self.last_frame_at.map(|at| at + self.heartbeat_interval * 2)
    }

    /// Handle one inbound text frame.
    pub(crate) fn frame(&mut self, text: &str, now: Instant, rand: f64) -> Vec<Action> {
        self.last_frame_at = Some(now);
        let payload: GatewayPayload = match serde_json::from_str(text) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "Undecodable gateway frame ignored");
                return Vec::new();
            }
        };
        if let Some(s) = payload.s {
            self.seq = Some(s);
        }

        let mut actions = Vec::new();
        match Opcode::from(payload.op) {
            Opcode::Hello => {
                self.on_hello(payload.d.as_ref(), now, rand);
                actions.push(self.handshake());
            }
            Opcode::HeartbeatAck => {
                if let Some(sent) = self.last_heartbeat_sent {
                    self.latency = Some(now.saturating_duration_since(sent));
                }
                self.awaiting_ack = false;
                if self.live && !self.stable {
                    self.stable = true;
                    self.backoff = BACKOFF_MIN;
                }
                debug!("Received heartbeat ACK");
            }
            Opcode::Dispatch => {
                if let Some(event_type) = payload.t {
                    self.on_dispatch(event_type, payload.d, &mut actions);
                }
            }
            Opcode::Reconnect => {
                warn!("Received reconnect request (op 7)");
                actions.push(Action::Reconnect { cause: ReconnectCause::Resume, reason: "op 7 reconnect" });
            }
            Opcode::InvalidSession => {
                let resumable = payload.d.as_ref().and_then(Value::as_bool).unwrap_or(false);
                warn!(resumable, "Received Invalid Session (op 9)");
                if resumable && self.has_session() {
                    actions.push(Action::Reconnect { cause: ReconnectCause::Resume, reason: "op 9 resumable" });
                } else {
                    if resumable {
                        warn!("Op 9 said resumable but no session is held; identifying instead");
                    }
                    self.clear_session();
                    actions.push(Action::Reconnect { cause: ReconnectCause::Identify, reason: "op 9 not resumable" });
                }
            }
            opcode => debug!("Received opcode: {:?}", opcode),
        }
        actions
    }

    fn on_hello(&mut self, d: Option<&Value>, now: Instant, rand: f64) {
        debug!("Received Hello");
        self.stage = ConnectionStage::Handshake;
        let Some(interval_ms) = d.and_then(|d| d["heartbeat_interval"].as_u64()) else {
            return;
        };
        self.heartbeat_interval = Duration::from_millis(interval_ms).min(MAX_HEARTBEAT_INTERVAL);
        self.awaiting_ack = false;
        self.last_heartbeat_sent = None;
        // Jittered first heartbeat, per the gateway spec.
        self.next_heartbeat = Some(now + self.heartbeat_interval.mul_f64(rand.clamp(0.0, 1.0)));
    }

    fn on_dispatch(&mut self, event_type: String, d: Option<Value>, actions: &mut Vec<Action>) {
        debug!("Dispatch event: {}", event_type);
        match event_type.as_str() {
            "READY" => {
                self.stage = ConnectionStage::Connected;
                self.live = true;
                if let Some(d) = &d {
                    if let Some(sid) = d["session_id"].as_str() {
                        // A session id lets its holder resume this gateway, so
                        // it stays out of info-level logs.
                        debug!("Captured session_id: {}", sid);
                        self.session_id = Some(sid.to_string());
                    }
                    self.resume_url = d["resume_gateway_url"].as_str().map(|url| with_query_of(url, &self.base_url));
                    self.pending_guilds = d["guilds"].as_array().map_or(0, Vec::len);
                    debug!("READY: waiting for {} GUILD_CREATE(s) before CACHE_READY", self.pending_guilds);
                    if self.pending_guilds == 0 {
                        actions.push(cache_ready());
                    }
                }
            }
            // GUILD_CREATE before READY (counter at 0) is ignored on purpose:
            // READY sets the counter.
            "GUILD_CREATE" if self.pending_guilds > 0 => {
                self.pending_guilds -= 1;
                if self.pending_guilds == 0 {
                    info!("All guilds received — firing CACHE_READY");
                    actions.push(cache_ready());
                }
            }
            "RESUMED" => {
                self.stage = ConnectionStage::Connected;
                self.live = true;
                info!("Session successfully resumed");
            }
            _ => {}
        }
        actions.push(Action::Dispatch { event_type, data: d.unwrap_or(Value::Null) });
    }

    /// Advance the clock. Fires whatever is due at `now`.
    pub(crate) fn tick(&mut self, now: Instant, _rand: f64) -> Vec<Action> {
        if self.read_deadline().is_some_and(|at| now >= at) {
            warn!(read_timeout = ?self.heartbeat_interval * 2, "No gateway frame within the read timeout; the connection may be dead");
            return vec![Action::Reconnect { cause: ReconnectCause::Failure, reason: "read timeout" }];
        }

        let mut actions = Vec::new();
        if let Some(due) = self.next_heartbeat {
            if now >= due {
                if self.awaiting_ack {
                    // Discord's zombie rule: no ACK since the last heartbeat
                    // means the connection is dead, whatever the socket says.
                    warn!("Previous heartbeat was never acknowledged; treating the connection as a zombie");
                    return vec![Action::Reconnect { cause: ReconnectCause::Failure, reason: "heartbeat not acknowledged" }];
                }
                actions.push(Action::Send(json!({ "op": 1, "d": self.seq }).to_string()));
                self.last_heartbeat_sent = Some(now);
                self.awaiting_ack = true;
                let next = due + self.heartbeat_interval;
                self.next_heartbeat = Some(if next > now { next } else { now + self.heartbeat_interval });
            }
        }
        actions
    }

    /// RESUME when a session is held, IDENTIFY otherwise.
    fn handshake(&mut self) -> Action {
        match self.resume_payload() {
            Some(resume) => {
                self.stage = ConnectionStage::Resuming;
                Action::Send(resume)
            }
            None => {
                self.stage = ConnectionStage::Identifying;
                Action::Identify
            }
        }
    }

    fn resume_payload(&self) -> Option<String> {
        let (sid, seq) = (self.session_id.as_ref()?, self.seq?);
        Some(json!({ "op": 6, "d": { "token": gateway_token(&self.token), "session_id": sid, "seq": seq } }).to_string())
    }
}

/// The token as the gateway expects it.
///
/// Discord's gateway expects the raw token — no `Bot ` / `Bearer ` prefix. The
/// same token in `DiscordHttpClient` carries the prefix verbatim for the HTTP
/// `Authorization` header (see `client::DiscordHttpClient::auth_header_value`),
/// so it is stripped here for gateway payloads only.
pub(crate) fn gateway_token(token: &str) -> &str {
    token.strip_prefix("Bot ").or_else(|| token.strip_prefix("Bearer ")).unwrap_or(token)
}

/// `url` with `base`'s query string, unless `url` already has one.
///
/// READY's `resume_gateway_url` is a bare host, and a connection to it must
/// carry the same `encoding` / `v` / `compress` parameters as the first one.
fn with_query_of(url: &str, base: &str) -> String {
    let query = url::Url::parse(base).ok().and_then(|b| b.query().map(str::to_owned));
    match (url::Url::parse(url), query) {
        (Ok(mut parsed), Some(query)) if parsed.query().is_none() => {
            parsed.set_query(Some(&query));
            parsed.to_string()
        }
        _ => url.to_string(),
    }
}

fn cache_ready() -> Action {
    Action::Dispatch { event_type: "CACHE_READY".to_string(), data: Value::Null }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "wss://gateway.example/?encoding=json&v=9";

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn hello(interval_ms: u64) -> String {
        json!({ "op": 10, "d": { "heartbeat_interval": interval_ms } }).to_string()
    }

    fn ack() -> String {
        json!({ "op": 11 }).to_string()
    }

    fn dispatch(t: &str, s: u64, d: Value) -> String {
        json!({ "op": 0, "t": t, "s": s, "d": d }).to_string()
    }

    fn ready(session: &str) -> String {
        dispatch("READY", 1, json!({ "session_id": session, "resume_gateway_url": "wss://resume.example", "guilds": [] }))
    }

    /// A core that has opened a socket at `t0` and received HELLO, with the
    /// first heartbeat jittered to land a full interval later.
    fn after_hello(interval_ms: u64) -> (GatewayCore, Instant) {
        let t0 = Instant::now();
        let mut core = GatewayCore::new(URL.to_string(), "tok".to_string());
        core.connecting();
        core.opened(t0);
        assert_eq!(core.frame(&hello(interval_ms), t0, 1.0), vec![Action::Identify]);
        (core, t0)
    }

    fn sends_op(action: &Action, op: u64) -> bool {
        matches!(action, Action::Send(text) if serde_json::from_str::<Value>(text).unwrap()["op"] == op)
    }

    #[test]
    fn hello_is_answered_at_once() {
        let (core, _) = after_hello(45_000);
        assert_eq!(core.stage(), ConnectionStage::Identifying);
    }

    #[test]
    fn ready_records_session_and_fires_cache_ready_for_no_guilds() {
        let (mut core, t0) = after_hello(45_000);
        let actions = core.frame(&ready("sess"), t0, 0.5);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], cache_ready());
        assert!(matches!(&actions[1], Action::Dispatch { event_type, .. } if event_type == "READY"));
        assert_eq!(core.stage(), ConnectionStage::Connected);
        assert_eq!(core.session_id(), Some("sess"));
        assert!(core.has_session());
    }

    #[test]
    fn cache_ready_waits_for_every_guild() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&dispatch("READY", 1, json!({ "session_id": "s", "guilds": [{ "id": "1" }, { "id": "2" }] })), t0, 0.5);
        let first = core.frame(&dispatch("GUILD_CREATE", 2, json!({ "id": "1" })), t0, 0.5);
        assert_eq!(first.len(), 1);
        let second = core.frame(&dispatch("GUILD_CREATE", 3, json!({ "id": "2" })), t0, 0.5);
        assert_eq!(second[0], cache_ready());
        assert_eq!(second.len(), 2);
    }

    #[test]
    fn hello_resumes_when_a_session_is_held() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(false);
        let t1 = t0 + ms(10_000);
        core.opened(t1);
        let actions = core.frame(&hello(45_000), t1, 1.0);
        assert_eq!(actions.len(), 1);
        let Action::Send(text) = &actions[0] else { panic!("expected RESUME, got {actions:?}") };
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(v["op"], 6);
        assert_eq!(v["d"]["session_id"], "sess");
        assert_eq!(v["d"]["seq"], 1);
        assert_eq!(v["d"]["token"], "tok");
        assert_eq!(core.stage(), ConnectionStage::Resuming);
    }

    #[test]
    fn resume_strips_the_token_prefix() {
        let t0 = Instant::now();
        let mut core = GatewayCore::new(URL.to_string(), "Bot tok".to_string());
        core.opened(t0);
        core.frame(&hello(45_000), t0, 1.0);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(false);
        core.opened(t0);
        let Action::Send(text) = &core.frame(&hello(45_000), t0, 1.0)[0] else { panic!("expected RESUME") };
        assert_eq!(serde_json::from_str::<Value>(text).unwrap()["d"]["token"], "tok");
    }

    #[test]
    fn connection_lost_with_clear_session_forces_identify() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(true);
        assert!(!core.has_session());
        assert_eq!(core.stage(), ConnectionStage::Disconnected);
        assert_eq!(core.connect_url(), URL);
    }

    #[test]
    fn heartbeat_carries_the_latest_sequence() {
        let (mut core, t0) = after_hello(1_000);
        core.frame(&dispatch("MESSAGE_CREATE", 42, json!({})), t0, 0.5);
        assert!(core.tick(t0 + ms(999), 0.5).is_empty());
        assert_eq!(core.tick(t0 + ms(1_000), 0.5), vec![Action::Send(json!({ "op": 1, "d": 42 }).to_string())]);
    }

    #[test]
    fn heartbeat_ack_measures_latency() {
        let (mut core, t0) = after_hello(1_000);
        core.tick(t0 + ms(1_000), 0.5);
        core.frame(&ack(), t0 + ms(1_040), 0.5);
        assert_eq!(core.latency(), Some(ms(40)));
    }

    #[test]
    fn acknowledged_heartbeats_keep_the_connection() {
        let (mut core, t0) = after_hello(1_000);
        for beat in 1..=5 {
            let at = t0 + ms(1_000 * beat);
            let actions = core.tick(at, 0.5);
            assert!(sends_op(&actions[0], 1), "beat {beat}: {actions:?}");
            core.frame(&ack(), at + ms(30), 0.5);
        }
    }

    #[test]
    fn op_7_reconnects_with_a_resume_cause() {
        let (mut core, t0) = after_hello(45_000);
        let actions = core.frame(&json!({ "op": 7 }).to_string(), t0, 0.5);
        assert_eq!(actions, vec![Action::Reconnect { cause: ReconnectCause::Resume, reason: "op 7 reconnect" }]);
    }

    #[test]
    fn read_timeout_is_two_intervals_of_silence_before_hello() {
        let t0 = Instant::now();
        let mut core = GatewayCore::new(URL.to_string(), "tok".to_string());
        core.opened(t0);
        let timeout = DEFAULT_HEARTBEAT_INTERVAL * 2;
        assert_eq!(core.next_deadline(), Some(t0 + timeout));
        assert!(core.tick(t0 + timeout - ms(1), 0.5).is_empty());
        assert_eq!(core.tick(t0 + timeout, 0.5), vec![Action::Reconnect { cause: ReconnectCause::Failure, reason: "read timeout" }]);
    }

    #[test]
    fn undecodable_frame_is_ignored() {
        let (mut core, t0) = after_hello(45_000);
        assert!(core.frame("not json", t0, 0.5).is_empty());
    }

    #[test]
    fn hello_interval_is_capped() {
        let (core, t0) = after_hello(10_000_000);
        // First heartbeat is jittered by rand = 1.0, so it lands one capped
        // interval after HELLO.
        assert_eq!(core.next_heartbeat, Some(t0 + MAX_HEARTBEAT_INTERVAL));
    }

    // ── G-1: RESUME goes to READY's resume_gateway_url ──

    #[test]
    fn resume_targets_resume_gateway_url_with_the_base_query() {
        let (mut core, t0) = after_hello(45_000);
        assert_eq!(core.connect_url(), URL);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(false);
        assert_eq!(core.connect_url(), "wss://resume.example/?encoding=json&v=9");
    }

    #[test]
    fn resume_url_keeps_its_own_query() {
        assert_eq!(with_query_of("ws://127.0.0.1:9/?v=10", URL), "ws://127.0.0.1:9/?v=10");
        assert_eq!(with_query_of("not a url", URL), "not a url");
    }

    // ── G-2: an unacknowledged heartbeat ends the connection at the next tick ──

    #[test]
    fn unacked_heartbeat_reconnects_at_the_next_tick() {
        let (mut core, t0) = after_hello(1_000);
        assert!(sends_op(&core.tick(t0 + ms(1_000), 0.5)[0], 1));
        // A dispatch keeps the read timeout away, but no ACK comes.
        core.frame(&dispatch("TYPING_START", 2, json!({})), t0 + ms(1_500), 0.5);
        assert!(core.tick(t0 + ms(1_999), 0.5).is_empty());
        assert_eq!(core.tick(t0 + ms(2_000), 0.5), vec![Action::Reconnect { cause: ReconnectCause::Failure, reason: "heartbeat not acknowledged" }]);
    }

    // ── G-4: op 9 reconnects instead of waiting on the reader ──

    #[test]
    fn resumable_op_9_reconnects_and_keeps_the_session() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        let actions = core.frame(&json!({ "op": 9, "d": true }).to_string(), t0, 0.5);
        assert_eq!(actions, vec![Action::Reconnect { cause: ReconnectCause::Resume, reason: "op 9 resumable" }]);
        assert!(core.has_session());
    }

    #[test]
    fn non_resumable_op_9_drops_the_session() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        let actions = core.frame(&json!({ "op": 9, "d": false }).to_string(), t0, 0.5);
        assert_eq!(actions, vec![Action::Reconnect { cause: ReconnectCause::Identify, reason: "op 9 not resumable" }]);
        assert!(!core.has_session());
        assert_eq!(core.connect_url(), URL);
    }

    #[test]
    fn resumable_op_9_without_a_session_identifies() {
        let (mut core, t0) = after_hello(45_000);
        let actions = core.frame(&json!({ "op": 9, "d": true }).to_string(), t0, 0.5);
        assert_eq!(actions, vec![Action::Reconnect { cause: ReconnectCause::Identify, reason: "op 9 not resumable" }]);
    }

    // ── G-5: the ladder applies to every cause, and resets only once stable ──

    #[test]
    fn plan_reconnect_table() {
        use ReconnectCause::*;
        let s = Duration::from_secs;
        // (cause, backoff, session_held, rand) -> (wait, next backoff)
        let cases = [
            ((Resume, s(1), true, 0.5), (FAST_RECONNECT, s(2))),
            ((Resume, s(2), true, 0.5), (s(2), s(4))),
            ((Resume, s(1), false, 0.5), (s(1), s(2))),
            ((Identify, s(1), false, 0.0), (s(1), s(2))),
            ((Identify, s(1), false, 1.0), (s(5), s(2))),
            ((Identify, s(1), false, 0.5), (s(3), s(2))),
            ((Identify, s(16), false, 0.5), (s(16), s(32))),
            ((Failure, s(1), true, 0.5), (s(1), s(2))),
            ((Failure, s(60), true, 0.5), (s(60), s(60))),
            ((Failure, s(40), true, 0.5), (s(40), s(60))),
        ];
        for ((cause, backoff, held, rand), want) in cases {
            assert_eq!(plan_reconnect(cause, backoff, held, rand), want, "{cause:?} {backoff:?} {held} {rand}");
        }
    }

    /// One connection that IDENTIFYs, reaches READY, and is closed with a
    /// session-ending code before any heartbeat is acknowledged.
    fn ready_then_close(core: &mut GatewayCore, at: Instant) -> Duration {
        core.opened(at);
        core.frame(&hello(45_000), at, 1.0);
        core.frame(&ready("sess"), at, 0.5);
        core.connection_lost(true);
        core.reconnect_delay(ReconnectCause::Identify, 0.0, plan_reconnect)
    }

    #[test]
    fn ready_then_close_loop_climbs_the_ladder() {
        let t0 = Instant::now();
        let mut core = GatewayCore::new(URL.to_string(), "tok".to_string());
        let waits: Vec<u64> = (0..8).map(|_| ready_then_close(&mut core, t0).as_secs()).collect();
        assert_eq!(waits, [1, 2, 4, 8, 16, 32, 60, 60]);
    }

    #[test]
    fn first_ack_after_going_live_resets_the_ladder() {
        let t0 = Instant::now();
        let mut core = GatewayCore::new(URL.to_string(), "tok".to_string());
        for _ in 0..4 {
            ready_then_close(&mut core, t0);
        }
        core.opened(t0);
        core.frame(&hello(1_000), t0, 1.0);
        // An ACK before READY does not count.
        core.tick(t0 + ms(1_000), 0.5);
        core.frame(&ack(), t0 + ms(1_010), 0.5);
        core.frame(&ready("sess"), t0 + ms(1_020), 0.5);
        core.connection_lost(false);
        assert_eq!(core.reconnect_delay(ReconnectCause::Failure, 0.5, plan_reconnect), Duration::from_secs(16));

        core.opened(t0);
        core.frame(&hello(1_000), t0, 1.0);
        core.frame(&dispatch("RESUMED", 2, json!({})), t0, 0.5);
        core.tick(t0 + ms(1_000), 0.5);
        core.frame(&ack(), t0 + ms(1_010), 0.5);
        core.connection_lost(false);
        assert_eq!(core.reconnect_delay(ReconnectCause::Resume, 0.5, plan_reconnect), FAST_RECONNECT);
    }
}
