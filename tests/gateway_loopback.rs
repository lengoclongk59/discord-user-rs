//! Drives `Gateway` against a scripted WebSocket server on 127.0.0.1.
//!
//! Every test here talks to a local listener only. The gateway URL is pointed
//! at it with `with_url`, and the client build number is pinned so IDENTIFY
//! never fetches it from discord.com.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use discord_user::{
    gateway::{default_close_verdict, CloseVerdict, FrameBuffer, Gateway, GatewayEvent, GatewayEvents, GatewayHooks, GatewayStatus, IdentifyDefaults, ReconnectCause},
    ConnectionStage, DiscordError, DispatchEvent, ReconnectType, UserStatus,
};
use futures::{future::BoxFuture, SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
    time::timeout,
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
    WebSocketStream,
};

const TOKEN: &str = "test-token";
const WAIT: Duration = Duration::from_secs(5);
/// HELLO interval long enough that no heartbeat lands inside a test unless
/// the test asks for one.
const QUIET_INTERVAL_MS: u64 = 60_000;

struct Server {
    listener: TcpListener,
    url: String,
}

/// A request URI recorded during the WebSocket upgrade.
type SeenUri = Arc<Mutex<String>>;

impl Server {
    async fn bind() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/?encoding=json&v=9", listener.local_addr().unwrap());
        Self { listener, url }
    }

    /// The listener's URL with no path or query, the shape of READY's
    /// `resume_gateway_url`.
    fn bare_url(&self) -> String {
        format!("ws://{}", self.listener.local_addr().unwrap())
    }

    async fn accept(&self) -> Conn {
        self.accept_within(WAIT).await.expect("gateway did not connect")
    }

    // The upgrade callback's error type is tungstenite's, not ours to shrink.
    #[allow(clippy::result_large_err)]
    async fn accept_within(&self, wait: Duration) -> Option<Conn> {
        let (stream, _) = timeout(wait, self.listener.accept()).await.ok()?.unwrap();
        let seen = SeenUri::default();
        let record = seen.clone();
        let ws = accept_hdr_async(stream, move |req: &Request, resp: Response| {
            *record.lock().unwrap() = req.uri().to_string();
            Ok(resp)
        })
        .await
        .unwrap();
        let uri = seen.lock().unwrap().clone();
        Some(Conn { ws, uri })
    }
}

struct Conn {
    ws: WebSocketStream<TcpStream>,
    /// Path and query the client asked for.
    uri: String,
}

impl Conn {
    async fn send(&mut self, frame: Value) {
        self.ws.send(Message::Text(frame.to_string().into())).await.unwrap();
    }

    async fn hello(&mut self, interval_ms: u64) {
        self.send(json!({ "op": 10, "d": { "heartbeat_interval": interval_ms } })).await;
    }

    async fn dispatch(&mut self, t: &str, s: u64, d: Value) {
        self.send(json!({ "op": 0, "t": t, "s": s, "d": d })).await;
    }

    async fn ready(&mut self, session_id: &str, resume_url: &str) {
        self.dispatch("READY", 1, json!({ "session_id": session_id, "resume_gateway_url": resume_url, "guilds": [] })).await;
    }

    /// The next frame the client sent with opcode `op`, skipping heartbeats.
    async fn recv_op(&mut self, op: u64) -> Value {
        timeout(WAIT, async {
            loop {
                match self.ws.next().await {
                    Some(Ok(Message::Text(text))) => {
                        let frame: Value = serde_json::from_str(text.as_str()).unwrap();
                        if frame["op"] == op {
                            return frame;
                        }
                        assert_eq!(frame["op"], 1, "unexpected frame while waiting for op {op}: {frame}");
                    }
                    Some(Ok(_)) => {}
                    other => panic!("socket ended while waiting for op {op}: {other:?}"),
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("no op {op} within {WAIT:?}"))
    }

    async fn close(&mut self, code: u16) {
        let frame = CloseFrame { code: CloseCode::from(code), reason: "test".into() };
        self.ws.close(Some(frame)).await.unwrap();
    }

    /// HELLO, then wait for the IDENTIFY that answers it, then READY.
    async fn identify_and_ready(&mut self, session_id: &str, resume_url: &str) -> Value {
        self.hello(QUIET_INTERVAL_MS).await;
        let identify = self.recv_op(2).await;
        self.ready(session_id, resume_url).await;
        identify
    }
}

fn gateway(url: &str) -> (Gateway, broadcast::Receiver<DispatchEvent>) {
    let (gw, rx) = Gateway::new(TOKEN.to_string(), UserStatus::Online, 64);
    (gw.with_url(url).with_client_build_number(1), rx)
}

/// Connect a gateway to `server`. The accept runs alongside `connect()`,
/// which does not return until the server completes the WebSocket upgrade.
async fn start(server: &Server) -> (Gateway, broadcast::Receiver<DispatchEvent>, Conn) {
    let (mut gw, rx) = gateway(&server.url);
    let (connected, conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    (gw, rx, conn)
}

async fn next_event(rx: &mut broadcast::Receiver<DispatchEvent>) -> DispatchEvent {
    timeout(WAIT, rx.recv()).await.expect("no dispatch within the wait").unwrap()
}

async fn wait_for_stage(gw: &Gateway, stage: ConnectionStage) {
    let deadline = Instant::now() + WAIT;
    while gw.stage().await != stage {
        assert!(Instant::now() < deadline, "stage stuck at {:?}, wanted {stage:?}", gw.stage().await);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn hello_is_answered_with_identify() {
    let server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;

    conn.hello(QUIET_INTERVAL_MS).await;
    let sent_hello = Instant::now();
    let identify = conn.recv_op(2).await;
    let waited = sent_hello.elapsed();

    assert_eq!(identify["d"]["token"], TOKEN);
    assert_eq!(identify["d"]["capabilities"], 16381);
    assert_eq!(identify["d"]["properties"]["client_build_number"], 1);
    assert_eq!(gw.stage().await, ConnectionStage::Identifying);
    // G-3: HELLO is answered at once, not after a fixed pause.
    assert!(waited < Duration::from_millis(250), "IDENTIFY after {waited:?}");

    gw.disconnect().await;
}

#[tokio::test]
async fn ready_and_dispatches_reach_subscribers() {
    let server = Server::bind().await;
    let (mut gw, mut rx, mut conn) = start(&server).await;

    conn.identify_and_ready("sess-1", "ws://127.0.0.1:9").await;
    conn.dispatch("MESSAGE_CREATE", 2, json!({ "id": "10", "content": "hi" })).await;

    let order: Vec<String> = [next_event(&mut rx).await, next_event(&mut rx).await, next_event(&mut rx).await].into_iter().map(|e| e.event_type).collect();
    assert_eq!(order, ["READY", "CACHE_READY", "MESSAGE_CREATE"]);
    wait_for_stage(&gw, ConnectionStage::Connected).await;
    assert_eq!(gw.reconnection_type().await, ReconnectType::Resume);

    gw.disconnect().await;
}

#[tokio::test]
async fn op_7_reconnects_and_resumes() {
    let server = Server::bind().await;
    let resume_server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;
    conn.identify_and_ready("sess-1", &resume_server.bare_url()).await;
    conn.dispatch("MESSAGE_CREATE", 5, json!({})).await;

    conn.send(json!({ "op": 7, "d": null })).await;

    // G-1: the RESUME goes to READY's resume URL, with the base URL's query.
    let mut conn = resume_server.accept().await;
    assert_eq!(conn.uri, "/?encoding=json&v=9");
    conn.hello(QUIET_INTERVAL_MS).await;
    let resume = conn.recv_op(6).await;
    assert_eq!(resume["d"]["session_id"], "sess-1");
    assert_eq!(resume["d"]["seq"], 5);
    assert_eq!(resume["d"]["token"], TOKEN);

    gw.disconnect().await;
}

#[tokio::test]
async fn session_ending_close_reconnects_with_identify() {
    let server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;
    conn.identify_and_ready("sess-1", "ws://127.0.0.1:9").await;
    wait_for_stage(&gw, ConnectionStage::Connected).await;

    conn.close(4009).await;

    // The session is gone, so the reconnect returns to the base URL, after
    // the 1-5 s IDENTIFY window.
    let mut conn = server.accept_within(Duration::from_secs(8)).await.expect("no re-IDENTIFY");
    conn.hello(QUIET_INTERVAL_MS).await;
    let identify = conn.recv_op(2).await;
    assert_eq!(identify["d"]["token"], TOKEN);

    gw.disconnect().await;
}

#[tokio::test]
async fn fatal_close_stops_the_supervisor() {
    let server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;
    conn.hello(QUIET_INTERVAL_MS).await;
    conn.recv_op(2).await;

    conn.close(4004).await;

    assert!(server.accept_within(Duration::from_millis(1_500)).await.is_none(), "reconnected after 4004");
    assert_eq!(gw.stage().await, ConnectionStage::Disconnected);
    // G-6: a stopped supervisor no longer reports a connection.
    assert!(!gw.is_connected().await);

    gw.disconnect().await;
}

#[tokio::test]
async fn unacknowledged_heartbeat_cuts_the_connection() {
    let server = Server::bind().await;
    let resume_server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;
    conn.hello(200).await;
    conn.recv_op(2).await;
    conn.ready("sess-1", &resume_server.bare_url()).await;

    // G-2: heartbeats arrive, none is acknowledged, and the client gives up on
    // the connection at the next tick and RESUMEs elsewhere.
    conn.recv_op(1).await;
    let gave_up = Instant::now();
    let mut conn = resume_server.accept().await;
    // Next heartbeat tick (<= 200 ms), then the ladder floor (1 s).
    assert!(gave_up.elapsed() < Duration::from_millis(2_000), "reconnected after {:?}", gave_up.elapsed());
    conn.hello(QUIET_INTERVAL_MS).await;
    assert_eq!(conn.recv_op(6).await["d"]["session_id"], "sess-1");

    gw.disconnect().await;
}

#[tokio::test]
async fn resumable_op_9_reconnects_on_the_fast_floor() {
    let server = Server::bind().await;
    let resume_server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;
    conn.identify_and_ready("sess-1", &resume_server.bare_url()).await;
    wait_for_stage(&gw, ConnectionStage::Connected).await;

    conn.send(json!({ "op": 9, "d": true })).await;
    let sent = Instant::now();

    // G-4: no 1-5 s wait on the reader; the RESUME follows the 250 ms floor.
    let mut conn = resume_server.accept().await;
    assert!(sent.elapsed() < Duration::from_millis(900), "reconnected after {:?}", sent.elapsed());
    conn.hello(QUIET_INTERVAL_MS).await;
    assert_eq!(conn.recv_op(6).await["d"]["session_id"], "sess-1");

    gw.disconnect().await;
}

#[tokio::test]
async fn non_resumable_op_9_identifies_after_the_jittered_wait() {
    let server = Server::bind().await;
    let (mut gw, _rx, mut conn) = start(&server).await;
    conn.identify_and_ready("sess-1", "ws://127.0.0.1:9").await;
    wait_for_stage(&gw, ConnectionStage::Connected).await;

    conn.send(json!({ "op": 9, "d": false })).await;
    let sent = Instant::now();

    // The jittered window runs to 5 s, as long as the default accept wait,
    // so this accept allows for the whole window.
    let mut conn = server.accept_within(Duration::from_secs(8)).await.expect("no re-IDENTIFY");
    let waited = sent.elapsed();
    assert!(waited >= Duration::from_millis(950) && waited < Duration::from_millis(5_500), "re-identified after {waited:?}");
    conn.hello(QUIET_INTERVAL_MS).await;
    conn.recv_op(2).await;

    gw.disconnect().await;
}

// ── Phase 3: caller hooks and the ordered event stream ──

async fn next_gateway_event(events: &mut GatewayEvents) -> GatewayEvent {
    timeout(WAIT, events.recv()).await.expect("no gateway event within the wait").expect("event stream closed")
}

/// The next status event, skipping frames and lag reports.
async fn next_status(events: &mut GatewayEvents) -> (GatewayStatus, Option<String>) {
    loop {
        if let GatewayEvent::Status { status, detail } = next_gateway_event(events).await {
            return (status, detail);
        }
    }
}

/// Fast, fixed reconnect pacing so hook tests do not wait out the ladder.
fn quick_delay(backoff: Duration) -> (Duration, Duration) {
    (Duration::from_millis(50), backoff)
}

#[tokio::test]
async fn frames_stream_reports_every_frame_and_each_status_change() {
    let server = Server::bind().await;
    let (mut gw, _rx) = gateway(&server.url);
    let mut events = gw.frames(FrameBuffer::Unbounded).unwrap();
    assert!(gw.frames(FrameBuffer::Unbounded).is_none(), "the receiver is handed out once");
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();

    conn.identify_and_ready("sess-1", "ws://127.0.0.1:9").await;
    conn.dispatch("MESSAGE_CREATE", 2, json!({ "content": "hi" })).await;

    assert_eq!(next_gateway_event(&mut events).await, GatewayEvent::Status { status: GatewayStatus::Connecting, detail: None });
    let GatewayEvent::Frame { op: 10, s: None, t: None, raw } = next_gateway_event(&mut events).await else { panic!("expected HELLO") };
    assert_eq!(serde_json::from_str::<Value>(&raw).unwrap()["d"]["heartbeat_interval"], QUIET_INTERVAL_MS);
    let GatewayEvent::Frame { op: 0, s: Some(1), t: Some(t), .. } = next_gateway_event(&mut events).await else { panic!("expected READY") };
    assert_eq!(t, "READY");
    assert_eq!(next_gateway_event(&mut events).await, GatewayEvent::Status { status: GatewayStatus::Live, detail: None });
    let GatewayEvent::Frame { s: Some(2), raw, .. } = next_gateway_event(&mut events).await else { panic!("expected MESSAGE_CREATE") };
    assert!(raw.contains("\"content\":\"hi\""), "raw frame is passed through: {raw}");

    gw.disconnect().await;
    assert_eq!(next_gateway_event(&mut events).await, GatewayEvent::Status { status: GatewayStatus::Stopped, detail: Some("shutdown".to_string()) });
    assert!(timeout(WAIT, events.recv()).await.unwrap().is_none(), "the stream closes after Stopped");
}

#[tokio::test]
async fn rejected_token_reports_auth_failed_then_stopped() {
    let server = Server::bind().await;
    let (mut gw, _rx) = gateway(&server.url);
    let mut events = gw.frames(FrameBuffer::Unbounded).unwrap();
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    conn.hello(QUIET_INTERVAL_MS).await;
    conn.recv_op(2).await;

    conn.close(4004).await;

    assert_eq!(next_status(&mut events).await.0, GatewayStatus::Connecting);
    assert_eq!(next_status(&mut events).await, (GatewayStatus::AuthFailed, Some("close 4004: test".to_string())));
    assert_eq!(next_status(&mut events).await, (GatewayStatus::Stopped, Some("Authentication failed".to_string())));
    assert!(timeout(WAIT, events.recv()).await.unwrap().is_none());
    assert!(!gw.is_connected().await, "is_connected agrees with Stopped");
}

/// Hands out tokens from a list and re-harvests after a 4004.
struct Reharvest {
    tokens: Mutex<Vec<&'static str>>,
    invalidated: AtomicUsize,
}

impl GatewayHooks for Reharvest {
    fn token<'a>(&'a self, _configured: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move { Some(self.tokens.lock().unwrap().remove(0).to_string()) })
    }

    fn invalidate(&self) {
        self.invalidated.fetch_add(1, Ordering::SeqCst);
    }

    fn on_close(&self, code: Option<u16>, _reason: &str) -> CloseVerdict {
        match code {
            Some(4004) => CloseVerdict::Identify,
            other => default_close_verdict(other),
        }
    }

    fn delay(&self, _cause: ReconnectCause, backoff: Duration, _held: bool, _rand: f64) -> (Duration, Duration) {
        quick_delay(backoff)
    }
}

#[tokio::test]
async fn token_hook_is_asked_before_every_connect_and_4004_can_reharvest() {
    let server = Server::bind().await;
    let hooks = Arc::new(Reharvest { tokens: Mutex::new(vec!["tok-1", "tok-2"]), invalidated: AtomicUsize::new(0) });
    let (gw, _rx) = gateway(&server.url);
    let mut gw = gw.with_hooks(hooks.clone());
    let mut events = gw.frames(FrameBuffer::Unbounded).unwrap();
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    conn.hello(QUIET_INTERVAL_MS).await;
    assert_eq!(conn.recv_op(2).await["d"]["token"], "tok-1");

    conn.close(4004).await;

    let mut conn = server.accept().await;
    conn.hello(QUIET_INTERVAL_MS).await;
    assert_eq!(conn.recv_op(2).await["d"]["token"], "tok-2");
    assert_eq!(hooks.invalidated.load(Ordering::SeqCst), 1);
    assert_eq!(next_status(&mut events).await.0, GatewayStatus::Connecting);
    assert_eq!(next_status(&mut events).await.0, GatewayStatus::AuthFailed);
    assert_eq!(next_status(&mut events).await.0, GatewayStatus::Connecting, "AuthFailed is not terminal");

    gw.disconnect().await;
}

#[tokio::test]
async fn a_changed_token_identifies_instead_of_resuming() {
    let server = Server::bind().await;
    let resume_server = Server::bind().await;
    let hooks = Arc::new(Reharvest { tokens: Mutex::new(vec!["tok-1", "tok-2"]), invalidated: AtomicUsize::new(0) });
    let (gw, _rx) = gateway(&server.url);
    let mut gw = gw.with_hooks(hooks);
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    conn.identify_and_ready("sess-1", &resume_server.bare_url()).await;
    wait_for_stage(&gw, ConnectionStage::Connected).await;

    conn.send(json!({ "op": 7, "d": null })).await;

    // The session belonged to tok-1, so the reconnect skips the resume URL.
    let mut conn = server.accept().await;
    conn.hello(QUIET_INTERVAL_MS).await;
    assert_eq!(conn.recv_op(2).await["d"]["token"], "tok-2");

    gw.disconnect().await;
}

struct NoCredential;

impl GatewayHooks for NoCredential {
    fn token<'a>(&'a self, _configured: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(async { None })
    }
}

#[tokio::test]
async fn connect_without_a_credential_fails() {
    let server = Server::bind().await;
    let (gw, _rx) = gateway(&server.url);
    let mut gw = gw.with_hooks(Arc::new(NoCredential));
    assert!(matches!(gw.connect().await, Err(DiscordError::GatewayConnection(_))));
    assert!(server.accept_within(Duration::from_millis(300)).await.is_none());
}

/// Sends fms-dash's IDENTIFY shape and stops on any 4000.
struct LinuxClient;

impl GatewayHooks for LinuxClient {
    fn identify<'a>(&'a self, token: &'a str, _defaults: &'a IdentifyDefaults) -> BoxFuture<'a, Value> {
        Box::pin(async move { json!({ "token": token, "capabilities": 0, "properties": { "os": "Linux", "browser": "Chrome" } }) })
    }

    fn on_close(&self, code: Option<u16>, reason: &str) -> CloseVerdict {
        match code {
            Some(4000) => CloseVerdict::Stop(format!("caller stopped on 4000 ({reason})")),
            other => default_close_verdict(other),
        }
    }
}

#[tokio::test]
async fn identify_and_close_hooks_replace_the_defaults() {
    let server = Server::bind().await;
    let (gw, _rx) = gateway(&server.url);
    let mut gw = gw.with_hooks(Arc::new(LinuxClient));
    let mut events = gw.frames(FrameBuffer::Unbounded).unwrap();
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    conn.hello(QUIET_INTERVAL_MS).await;
    let identify = conn.recv_op(2).await;
    assert_eq!(identify["d"], json!({ "token": TOKEN, "capabilities": 0, "properties": { "os": "Linux", "browser": "Chrome" } }));

    conn.close(4000).await;

    assert_eq!(next_status(&mut events).await.0, GatewayStatus::Connecting);
    assert_eq!(next_status(&mut events).await, (GatewayStatus::Stopped, Some("caller stopped on 4000 (test)".to_string())));
    assert!(server.accept_within(Duration::from_millis(500)).await.is_none());
}

struct QuickReconnect;

impl GatewayHooks for QuickReconnect {
    fn delay(&self, _cause: ReconnectCause, backoff: Duration, _held: bool, _rand: f64) -> (Duration, Duration) {
        quick_delay(backoff)
    }
}

#[tokio::test]
async fn delay_hook_paces_reconnects() {
    let server = Server::bind().await;
    let (gw, _rx) = gateway(&server.url);
    let mut gw = gw.with_hooks(Arc::new(QuickReconnect));
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    conn.identify_and_ready("sess-1", "ws://127.0.0.1:9").await;
    wait_for_stage(&gw, ConnectionStage::Connected).await;

    // 4009 would wait the 1-5 s IDENTIFY window by default.
    conn.close(4009).await;
    let closed = Instant::now();
    let _conn = server.accept().await;
    assert!(closed.elapsed() < Duration::from_millis(500), "reconnected after {:?}", closed.elapsed());

    gw.disconnect().await;
}

#[tokio::test]
async fn bounded_stream_counts_drops_and_never_stalls_the_reader() {
    let server = Server::bind().await;
    let (gw, _rx) = gateway(&server.url);
    let mut gw = gw.with_hooks(Arc::new(QuickReconnect));
    let mut events = gw.frames(FrameBuffer::Bounded(4)).unwrap();
    let (connected, mut conn) = tokio::join!(gw.connect(), server.accept());
    connected.unwrap();
    conn.hello(600).await;
    conn.recv_op(2).await;
    conn.ready("sess-1", "ws://127.0.0.1:9").await;

    // Nobody reads the stream while 200 dispatches arrive and heartbeats keep
    // being acknowledged across several intervals.
    for s in 2..202 {
        conn.dispatch("TYPING_START", s, json!({})).await;
    }
    for _ in 0..4 {
        conn.recv_op(1).await;
        conn.send(json!({ "op": 11 })).await;
    }
    assert!(server.accept_within(Duration::from_millis(200)).await.is_none(), "the connection survived");
    assert_eq!(gw.stage().await, ConnectionStage::Connected);

    let buffered: Vec<GatewayEvent> = std::iter::from_fn(|| events.try_recv().ok()).collect();
    assert_eq!(buffered.len(), 4);
    conn.dispatch("MESSAGE_CREATE", 202, json!({})).await;
    let GatewayEvent::Lagged { dropped } = next_gateway_event(&mut events).await else { panic!("expected Lagged") };
    // The four kept are Connecting, HELLO, READY and Live; the 200
    // dispatches and the 4 ACKs were dropped.
    assert_eq!(dropped, 204);
    let GatewayEvent::Frame { s: Some(202), .. } = next_gateway_event(&mut events).await else { panic!("expected the next frame") };

    gw.disconnect().await;
}
