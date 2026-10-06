//! Drives `Gateway` against a scripted WebSocket server on 127.0.0.1.
//!
//! Every test here talks to a local listener only. The gateway URL is pointed
//! at it with `with_url`, and the client build number is pinned so IDENTIFY
//! never fetches it from discord.com.

use std::time::{Duration, Instant};

use discord_user::{gateway::Gateway, ConnectionStage, DispatchEvent, ReconnectType, UserStatus};
use futures::{SinkExt, StreamExt};
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
type SeenUri = std::sync::Arc<std::sync::Mutex<String>>;

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
    assert_eq!(order, ["CACHE_READY", "READY", "MESSAGE_CREATE"]);
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

    // The session is gone, so the reconnect returns to the base URL.

    let mut conn = server.accept().await;
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

    let mut conn = server.accept().await;
    let waited = sent.elapsed();
    assert!(waited >= Duration::from_millis(950), "re-identified after {waited:?}");
    conn.hello(QUIET_INTERVAL_MS).await;
    conn.recv_op(2).await;

    gw.disconnect().await;
}
