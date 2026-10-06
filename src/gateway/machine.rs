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

/// Pause between HELLO and the IDENTIFY or RESUME that answers it.
const HANDSHAKE_DELAY: Duration = Duration::from_millis(500);

/// Why a connection is being replaced. Drives how long the supervisor waits
/// before the next connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectCause {
    /// Discord asked this connection to move (op 7), or closed it with a code
    /// that keeps the session. The next connect RESUMEs if a session is held.
    Resume,
    /// The session is gone; the next connect must IDENTIFY.
    Identify,
    /// An ordinary failure: a dropped socket, a read timeout, a refused
    /// connect.
    Failure,
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
    stage: ConnectionStage,
    heartbeat_interval: Duration,
    next_heartbeat: Option<Instant>,
    awaiting_ack: bool,
    last_heartbeat_sent: Option<Instant>,
    latency: Option<Duration>,
    /// When the current connection last showed a sign of life (opened, or a
    /// frame arrived). The read timeout counts from here.
    last_frame_at: Option<Instant>,
    /// When to answer HELLO with IDENTIFY or RESUME.
    handshake_at: Option<Instant>,
    /// An op 9 waiting out its delay: when to answer, and whether Discord
    /// said the session is resumable.
    invalid_session_at: Option<(Instant, bool)>,
    /// Guilds from READY still awaiting their GUILD_CREATE.
    pending_guilds: usize,
}

impl GatewayCore {
    pub(crate) fn new(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            session_id: None,
            seq: None,
            stage: ConnectionStage::Disconnected,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            next_heartbeat: None,
            awaiting_ack: false,
            last_heartbeat_sent: None,
            latency: None,
            last_frame_at: None,
            handshake_at: None,
            invalid_session_at: None,
            pending_guilds: 0,
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

    /// The URL the next connection should open.
    pub(crate) fn connect_url(&self) -> &str {
        &self.base_url
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

    fn reset_connection(&mut self) {
        self.next_heartbeat = None;
        self.awaiting_ack = false;
        self.last_heartbeat_sent = None;
        self.last_frame_at = None;
        self.handshake_at = None;
        self.invalid_session_at = None;
    }

    fn clear_session(&mut self) {
        self.session_id = None;
        self.seq = None;
    }

    /// The earliest instant at which [`Self::tick`] has work to do.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        [self.read_deadline(), self.handshake_at, self.invalid_session_at.map(|(at, _)| at), self.next_heartbeat].into_iter().flatten().min()
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
            Opcode::Hello => self.on_hello(payload.d.as_ref(), now, rand),
            Opcode::HeartbeatAck => {
                if let Some(sent) = self.last_heartbeat_sent {
                    self.latency = Some(now.saturating_duration_since(sent));
                }
                self.awaiting_ack = false;
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
                // Discord asks for a randomised 1-5 s wait before answering, so
                // a herd of invalidated clients does not return in lockstep.
                let delay = Duration::from_millis(1000 + (rand * 4000.0) as u64);
                self.invalid_session_at = Some((now + delay, resumable));
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
        self.handshake_at = Some(now + HANDSHAKE_DELAY);
    }

    fn on_dispatch(&mut self, event_type: String, d: Option<Value>, actions: &mut Vec<Action>) {
        debug!("Dispatch event: {}", event_type);
        match event_type.as_str() {
            "READY" => {
                self.stage = ConnectionStage::Connected;
                if let Some(d) = &d {
                    if let Some(sid) = d["session_id"].as_str() {
                        // A session id lets its holder resume this gateway, so
                        // it stays out of info-level logs.
                        debug!("Captured session_id: {}", sid);
                        self.session_id = Some(sid.to_string());
                    }
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
                info!("Session successfully resumed");
            }
            _ => {}
        }
        actions.push(Action::Dispatch { event_type, data: d.unwrap_or(Value::Null) });
    }

    /// Advance the clock. Fires whatever is due at `now`.
    pub(crate) fn tick(&mut self, now: Instant, rand: f64) -> Vec<Action> {
        let _ = rand;
        let mut actions = Vec::new();

        if self.read_deadline().is_some_and(|at| now >= at) {
            warn!(read_timeout = ?self.heartbeat_interval * 2, "No gateway frame within the read timeout; the connection may be dead");
            actions.push(Action::Reconnect { cause: ReconnectCause::Failure, reason: "read timeout" });
            return actions;
        }

        if self.handshake_at.is_some_and(|at| now >= at) {
            self.handshake_at = None;
            actions.push(self.handshake());
        }

        if let Some((at, resumable)) = self.invalid_session_at {
            if now >= at {
                self.invalid_session_at = None;
                if resumable && self.has_session() {
                    actions.push(self.handshake());
                } else {
                    if resumable {
                        warn!("Op 9 said resumable but no session is held; identifying instead");
                    }
                    self.clear_session();
                    actions.push(self.handshake());
                }
            }
        }

        if let Some(due) = self.next_heartbeat {
            if now >= due {
                if self.awaiting_ack {
                    warn!("Still awaiting heartbeat ACK; sending heartbeat anyway");
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
        Some(json!({ "op": 6, "d": { "token": self.token, "session_id": sid, "seq": seq } }).to_string())
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
        assert!(core.frame(&hello(interval_ms), t0, 1.0).is_empty());
        (core, t0)
    }

    /// Run ticks every `step` from `from` to `to` inclusive, collecting actions.
    fn run(core: &mut GatewayCore, from: Instant, to: Instant, step: Duration) -> Vec<(Duration, Action)> {
        let mut out = Vec::new();
        let mut now = from;
        while now <= to {
            for a in core.tick(now, 0.5) {
                out.push((now - from, a));
            }
            now += step;
        }
        out
    }

    fn sends_op(action: &Action, op: u64) -> bool {
        matches!(action, Action::Send(text) if serde_json::from_str::<Value>(text).unwrap()["op"] == op)
    }

    #[test]
    fn hello_waits_before_identifying() {
        let (mut core, t0) = after_hello(45_000);
        assert_eq!(core.stage(), ConnectionStage::Handshake);
        assert!(core.tick(t0 + ms(499), 0.5).is_empty());
        assert_eq!(core.tick(t0 + ms(500), 0.5), vec![Action::Identify]);
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
    fn handshake_resumes_when_a_session_is_held() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(false);
        let t1 = t0 + ms(10_000);
        core.opened(t1);
        core.frame(&hello(45_000), t1, 1.0);
        let actions = core.tick(t1 + ms(500), 0.5);
        assert_eq!(actions.len(), 1);
        let Action::Send(text) = &actions[0] else { panic!("expected RESUME, got {actions:?}") };
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(v["op"], 6);
        assert_eq!(v["d"]["session_id"], "sess");
        assert_eq!(v["d"]["seq"], 1);
        assert_eq!(core.stage(), ConnectionStage::Resuming);
    }

    #[test]
    fn connection_lost_with_clear_session_forces_identify() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(true);
        assert!(!core.has_session());
        assert_eq!(core.stage(), ConnectionStage::Disconnected);
    }

    #[test]
    fn heartbeat_carries_the_latest_sequence() {
        let (mut core, t0) = after_hello(1_000);
        core.frame(&dispatch("MESSAGE_CREATE", 42, json!({})), t0, 0.5);
        let beats: Vec<_> = run(&mut core, t0 + ms(600), t0 + ms(1_000), ms(100)).into_iter().filter(|(_, a)| sends_op(a, 1)).collect();
        assert_eq!(beats.len(), 1);
        assert_eq!(beats[0].1, Action::Send(json!({ "op": 1, "d": 42 }).to_string()));
    }

    #[test]
    fn heartbeat_ack_measures_latency() {
        let (mut core, t0) = after_hello(1_000);
        core.tick(t0 + ms(1_000), 0.5);
        core.frame(&json!({ "op": 11 }).to_string(), t0 + ms(1_040), 0.5);
        assert_eq!(core.latency(), Some(ms(40)));
    }

    #[test]
    fn op_7_reconnects_with_a_resume_cause() {
        let (mut core, t0) = after_hello(45_000);
        let actions = core.frame(&json!({ "op": 7 }).to_string(), t0, 0.5);
        assert_eq!(actions, vec![Action::Reconnect { cause: ReconnectCause::Resume, reason: "op 7 reconnect" }]);
    }

    #[test]
    fn read_timeout_is_two_intervals_of_silence() {
        let (mut core, t0) = after_hello(1_000);
        core.frame(&json!({ "op": 11 }).to_string(), t0 + ms(500), 0.5);
        assert_eq!(core.next_deadline(), Some(t0 + ms(500)));
        assert!(!core.tick(t0 + ms(2_499), 0.5).iter().any(|a| matches!(a, Action::Reconnect { .. })));
        assert_eq!(core.tick(t0 + ms(2_500), 0.5), vec![Action::Reconnect { cause: ReconnectCause::Failure, reason: "read timeout" }]);
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

    // ── Behaviour pinned as found in 0.6.1. The tests in this block describe
    // what the code does, not what it should do; the fixes flip them. ──

    #[test]
    fn pinned_unacked_heartbeat_is_resent_without_reconnecting() {
        let (mut core, t0) = after_hello(1_000);
        let actions = run(&mut core, t0 + ms(500), t0 + ms(1_400), ms(100));
        assert!(actions.iter().any(|(_, a)| sends_op(a, 1)));
        // A dispatch keeps the read timeout away, but no ACK comes. The next
        // heartbeat tick neither reconnects nor holds back the heartbeat.
        core.frame(&dispatch("TYPING_START", 2, json!({})), t0 + ms(1_500), 0.5);
        assert!(core.tick(t0 + ms(1_900), 0.5).is_empty());
        let actions = core.tick(t0 + ms(2_000), 0.5);
        assert_eq!(actions.len(), 1);
        assert!(sends_op(&actions[0], 1));
    }

    #[test]
    fn pinned_invalid_session_waits_then_answers_on_the_same_socket() {
        let (mut core, t0) = after_hello(45_000);
        core.tick(t0 + ms(500), 0.5);
        core.frame(&ready("sess"), t0 + ms(600), 0.5);
        // rand 0.5 → 1000 + 2000 ms.
        assert!(core.frame(&json!({ "op": 9, "d": true }).to_string(), t0 + ms(1_000), 0.5).is_empty());
        assert!(core.tick(t0 + ms(3_999), 0.5).is_empty());
        let actions = core.tick(t0 + ms(4_000), 0.5);
        assert!(sends_op(&actions[0], 6), "resumable op 9 answers with RESUME: {actions:?}");
    }

    #[test]
    fn pinned_non_resumable_invalid_session_identifies_after_the_wait() {
        let (mut core, t0) = after_hello(45_000);
        core.tick(t0 + ms(500), 0.5);
        core.frame(&ready("sess"), t0 + ms(500), 0.5);
        core.frame(&json!({ "op": 9, "d": false }).to_string(), t0 + ms(500), 0.0);
        assert!(core.has_session());
        assert_eq!(core.tick(t0 + ms(1_500), 0.5), vec![Action::Identify]);
        assert!(!core.has_session());
    }

    #[test]
    fn pinned_resume_goes_to_the_base_url() {
        let (mut core, t0) = after_hello(45_000);
        core.frame(&ready("sess"), t0, 0.5);
        core.connection_lost(false);
        assert_eq!(core.connect_url(), URL);
    }
}
