//! Discord WebSocket Gateway connection

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc, RwLock},
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream};
use tracing::{error, info, warn};

pub use self::hooks::{default_close_verdict, CloseVerdict, DefaultHooks, GatewayHooks, IdentifyDefaults};
pub use self::machine::{plan_reconnect, ReconnectCause, BACKOFF_MAX, BACKOFF_MIN, FAST_RECONNECT, INVALID_SESSION_MAX, INVALID_SESSION_MIN};
use self::machine::{Action, GatewayCore};
use self::stream::EventSink;
pub use self::stream::{FrameBuffer, GatewayEvent, GatewayEvents, GatewayStatus};
use crate::{
    error::Result,
    events::DispatchEvent,
    types::{ConnectionStage, ReconnectType, UserStatus},
};

mod hooks;
mod machine;
mod stream;

/// Hardcoded gateway endpoint (mimics Chrome browser session)
const GATEWAY_URL: &str = "wss://gateway-us-east1-c.discord.gg/?encoding=json&v=9";

/// Gateway endpoint when zstd-stream compression is requested.
const GATEWAY_URL_ZSTD: &str = "wss://gateway-us-east1-c.discord.gg/?encoding=json&v=9&compress=zstd-stream";

/// Capabilities bitmask for the IDENTIFY payload (user-account equivalent of
/// gateway intents). 16381 is the value the official Discord web client has
/// shipped at time of writing; it enables every capability bit the user
/// surface depends on. Bump it if Discord rolls a new client default.
const DEFAULT_CAPABILITIES: u32 = 16381;

/// Default `large_threshold` per Discord spec; valid range is 50–250.
/// Members past this count are treated as "large" guilds and require
/// explicit RequestGuildMembers (opcode 8) calls to fetch.
const DEFAULT_LARGE_THRESHOLD: u16 = 50;

/// Compression flavour negotiated for the gateway connection.
///
/// Discord supports raw JSON, `zlib-stream`, and `zstd-stream` transports.
/// Zlib is handled inline via [`Gateway::decompress_zlib`]; zstd decoding
/// is not yet wired up — selecting [`CompressionKind::Zstd`] only flips
/// the URL query parameter so the negotiation is visible to Discord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionKind {
    /// No compression negotiated at the URL level (zlib payloads may still
    /// arrive as binary frames and are decoded best-effort).
    #[default]
    None,
    /// Negotiate `?compress=zlib-stream` (handled in-process today).
    Zlib,
    /// Negotiate `?compress=zstd-stream` (URL-only; decoder TODO).
    Zstd,
}

/// Fallback build number used when the live fetch fails.
const CLIENT_BUILD_NUMBER_FALLBACK: u32 = 534982;

/// Fetch the Discord client build number from the login page once, then cache
/// it.
///
/// Discord embeds `"BUILD_NUMBER":"<digits>"` in the login page's inline
/// script. Falls back to `CLIENT_BUILD_NUMBER_FALLBACK` on any network or parse
/// error.
async fn client_build_number() -> u32 {
    static CACHED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    if let Some(&n) = CACHED.get() {
        return n;
    }
    let n = fetch_build_number_from_discord().await.unwrap_or_else(|| {
        warn!("Could not fetch Discord build number, using fallback {CLIENT_BUILD_NUMBER_FALLBACK}");
        CLIENT_BUILD_NUMBER_FALLBACK
    });
    info!("Discord client build number: {n}");
    let _ = CACHED.set(n);
    n
}

async fn fetch_build_number_from_discord() -> Option<u32> {
    let html = reqwest::get("https://discord.com/login").await.ok()?.text().await.ok()?;
    let marker = "\"BUILD_NUMBER\":\"";
    let start = html.find(marker)? + marker.len();
    let end = html[start..].find('"')? + start;
    html[start..end].parse().ok()
}

type WsWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;
type WsReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Gateway opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    PresenceUpdate = 3,
    VoiceStateUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    RequestGuildMembers = 8,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
    RequestChannelInfo = 43,
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        match value {
            0 => Opcode::Dispatch,
            1 => Opcode::Heartbeat,
            2 => Opcode::Identify,
            3 => Opcode::PresenceUpdate,
            4 => Opcode::VoiceStateUpdate,
            6 => Opcode::Resume,
            7 => Opcode::Reconnect,
            8 => Opcode::RequestGuildMembers,
            9 => Opcode::InvalidSession,
            10 => Opcode::Hello,
            11 => Opcode::HeartbeatAck,
            43 => Opcode::RequestChannelInfo,
            _ => Opcode::Dispatch,
        }
    }
}

/// Gateway payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPayload {
    pub op: u8,
    #[serde(default)]
    pub d: Option<Value>,
    #[serde(default)]
    pub s: Option<u64>,
    #[serde(default)]
    pub t: Option<String>,
}

/// Outcome of a single gateway read-loop iteration. Drives the supervisor's
/// reconnect / give-up decision.
#[derive(Debug)]
enum GatewayLoopOutcome {
    /// Caller requested shutdown — exit the supervisor.
    Shutdown,
    /// The peer sent a close frame; the close policy decides what follows.
    Closed { code: Option<u16>, reason: String },
    /// The connection ended; reconnect. A [`ReconnectCause::Identify`] cause
    /// means Discord told us our session is gone, so the next connection
    /// performs a full IDENTIFY rather than RESUME.
    Reconnect(ReconnectCause),
}

/// Snapshot of the supervisor's state, read by the public getters.
#[derive(Debug, Default)]
struct Shared {
    stage: ConnectionStage,
    latency: Option<Duration>,
    has_session: bool,
}

/// Where the supervisor finds the [`Gateway::frames`] sink, if one was asked
/// for.
#[derive(Debug, Default)]
struct EventSlot {
    sink: Option<EventSink>,
    /// `frames()` already handed out its receiver.
    taken: bool,
}

/// Run `f` on the event sink, if [`Gateway::frames`] installed one.
fn with_sink(slot: &Mutex<EventSlot>, f: impl FnOnce(&mut EventSink)) {
    if let Some(sink) = slot.lock().unwrap_or_else(|e| e.into_inner()).sink.as_mut() {
        f(sink);
    }
}

async fn send_text(writer: &Arc<RwLock<Option<WsWriter>>>, text: String) -> Result<()> {
    if let Some(ref mut w) = *writer.write().await {
        w.send(WsMessage::Text(text.into())).await?;
    }
    Ok(())
}

/// Gateway connection manager
pub struct Gateway {
    token: String,
    custom_status: UserStatus,
    /// Capabilities bitmask sent in the IDENTIFY payload (user-account
    /// equivalent of intents)
    capabilities: u32,
    /// Optional gateway intents bitmask for bot-token connections. When
    /// `Some`, the IDENTIFY payload includes an `intents` field alongside
    /// `capabilities` so the same struct can drive both user and bot tokens.
    intents: Option<u64>,
    /// `large_threshold` value sent in IDENTIFY (50–250). Discord uses this
    /// to decide whether a guild's full member list is delivered in
    /// GUILD_CREATE or must be requested via opcode 8.
    large_threshold: u16,
    /// Negotiated transport compression flavour. Currently only changes the
    /// gateway URL; zstd payload decoding is a future task.
    compression_kind: CompressionKind,
    /// Optional `[shard_id, shard_count]` array sent in IDENTIFY. Discord requires
    /// sharding once a bot reaches 2 500 guilds and may force-close with code 4011.
    shard: Option<[u32; 2]>,
    /// Base URL override. `None` uses the Discord URL for `compression_kind`.
    url: Option<String>,
    /// Client build number pinned for IDENTIFY. `None` fetches it from
    /// discord.com once per process.
    client_build_number: Option<u32>,
    hooks: Arc<dyn GatewayHooks>,
    writer: Arc<RwLock<Option<WsWriter>>>,
    event_sender: broadcast::Sender<DispatchEvent>,
    shared: Arc<Mutex<Shared>>,
    events: Arc<Mutex<EventSlot>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl Gateway {
    /// Create a new gateway connection
    pub fn new(token: String, custom_status: UserStatus, event_buffer_size: usize) -> (Self, broadcast::Receiver<DispatchEvent>) {
        Self::new_with_capabilities(token, custom_status, event_buffer_size, DEFAULT_CAPABILITIES)
    }

    /// Create a gateway connection with a custom capabilities bitmask.
    ///
    /// The `capabilities` value is sent as the `capabilities` field in the
    /// IDENTIFY payload — the user-account equivalent of gateway intents.
    /// Use [`GatewayIntents`](crate::types::GatewayIntents) bits or a raw
    /// `u32`.
    pub fn new_with_capabilities(token: String, custom_status: UserStatus, event_buffer_size: usize, capabilities: u32) -> (Self, broadcast::Receiver<DispatchEvent>) {
        let (event_sender, event_receiver) = broadcast::channel(event_buffer_size);

        (
            Self {
                token,
                custom_status,
                capabilities,
                intents: None,
                large_threshold: DEFAULT_LARGE_THRESHOLD,
                compression_kind: CompressionKind::None,
                shard: None,
                url: None,
                client_build_number: None,
                hooks: Arc::new(DefaultHooks),
                writer: Arc::new(RwLock::new(None)),
                event_sender,
                shared: Arc::new(Mutex::new(Shared::default())),
                events: Arc::new(Mutex::new(EventSlot::default())),
                shutdown_tx: None,
            },
            event_receiver,
        )
    }

    /// Set gateway intents for bot-token connections.
    ///
    /// When set, the IDENTIFY payload will include an `intents` field
    /// alongside the existing `capabilities` field, allowing the same
    /// gateway to drive bot tokens. User-token semantics are unaffected
    /// when this is left as `None` (the default).
    pub fn with_intents(mut self, intents: u64) -> Self {
        self.intents = Some(intents);
        self
    }

    /// Override the IDENTIFY `large_threshold` (50–250 per Discord docs).
    ///
    /// Values outside the valid range are clamped.
    pub fn with_large_threshold(mut self, large_threshold: u16) -> Self {
        self.large_threshold = large_threshold.clamp(50, 250);
        self
    }

    /// Select the transport compression flavour used when (re)connecting.
    ///
    /// Today only changes the gateway URL query parameter; zstd payload
    /// decoding is implemented in a follow-up task.
    pub fn with_compression(mut self, kind: CompressionKind) -> Self {
        self.compression_kind = kind;
        self
    }

    /// Configure the IDENTIFY `shard` array as `[shard_id, shard_count]`.
    ///
    /// Discord requires sharding once a bot reaches 2 500 guilds and may
    /// force-close with code 4011 ("sharding required") otherwise.
    pub fn with_shard(mut self, shard_id: u32, shard_count: u32) -> Self {
        self.shard = Some([shard_id, shard_count]);
        self
    }

    /// Connect to `url` instead of Discord's gateway.
    ///
    /// The URL replaces the one [`Self::with_compression`] would pick, query
    /// string included, so it should carry the same `encoding` and `v`
    /// parameters. Intended for tests against a local server and for proxies.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Pin the `client_build_number` sent in IDENTIFY.
    ///
    /// Without it, the first IDENTIFY in the process fetches the number from
    /// `https://discord.com/login` and caches it.
    pub fn with_client_build_number(mut self, build_number: u32) -> Self {
        self.client_build_number = Some(build_number);
        self
    }

    /// Install caller policy: credentials, the IDENTIFY body, the close
    /// policy and reconnect pacing. See [`GatewayHooks`]; every method left
    /// unimplemented keeps the library's behaviour.
    pub fn with_hooks(mut self, hooks: Arc<dyn GatewayHooks>) -> Self {
        self.hooks = hooks;
        self
    }

    /// Open the ordered event stream: every inbound frame, as received, and
    /// every status change.
    ///
    /// The receiver can be taken once; a later call returns `None`. Call it
    /// before [`Self::connect`] to see the first frames. The stream ends with
    /// [`GatewayStatus::Stopped`] when the supervisor exits, and does not
    /// carry over to a later `connect()`.
    ///
    /// The supervisor never waits on this stream. With
    /// [`FrameBuffer::Bounded`], events that do not fit are dropped and
    /// reported as [`GatewayEvent::Lagged`]. The [`DispatchEvent`] broadcast
    /// returned by [`Self::new`] is unaffected.
    pub fn frames(&self, buffer: FrameBuffer) -> Option<GatewayEvents> {
        let mut slot = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if slot.taken {
            return None;
        }
        let (sink, events) = EventSink::channel(buffer);
        slot.sink = Some(sink);
        slot.taken = true;
        Some(events)
    }

    /// The library's IDENTIFY settings, as configured on this gateway.
    pub fn identify_defaults(&self) -> IdentifyDefaults {
        IdentifyDefaults { custom_status: self.custom_status, capabilities: self.capabilities, intents: self.intents, large_threshold: self.large_threshold, shard: self.shard, client_build_number: self.client_build_number }
    }

    /// Return the gateway URL for the configured override or compression.
    fn gateway_url(&self) -> &str {
        if let Some(url) = &self.url {
            return url;
        }
        match self.compression_kind {
            CompressionKind::Zstd => GATEWAY_URL_ZSTD,
            // TODO: add a dedicated zlib-stream URL once we negotiate it explicitly.
            CompressionKind::Zlib | CompressionKind::None => GATEWAY_URL,
        }
    }

    /// Get event sender for subscribing
    pub fn event_sender(&self) -> broadcast::Sender<DispatchEvent> {
        self.event_sender.clone()
    }

    fn shared(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Connect to the gateway and spawn the supervisor that keeps the
    /// connection alive.
    ///
    /// Returns once the WebSocket is open; IDENTIFY happens in the background
    /// after Discord's HELLO. The supervisor reconnects on its own and exits
    /// only on [`Self::disconnect`] or when the close policy says stop (by
    /// default 4004 auth failure, 4014 disallowed intents, etc.).
    ///
    /// Fails with [`DiscordError::GatewayConnection`](crate::DiscordError::GatewayConnection)
    /// when [`GatewayHooks::token`] has no credential.
    pub async fn connect(&mut self) -> Result<()> {
        with_sink(&self.events, |sink| sink.status(GatewayStatus::Connecting, None));
        let token = self.hooks.token(&self.token).await.ok_or_else(|| crate::error::DiscordError::GatewayConnection("no gateway credential available".to_string()))?;
        let mut core = GatewayCore::new(self.gateway_url().to_string(), token);
        core.connecting();
        self.shared().stage = core.stage();

        let (ws_stream, _) = connect_async(core.connect_url()).await?;
        let (writer, reader) = ws_stream.split();
        *self.writer.write().await = Some(writer);
        core.opened(Instant::now());

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let supervisor = Supervisor {
            core,
            configured_token: self.token.clone(),
            hooks: Arc::clone(&self.hooks),
            identify: self.identify_defaults(),
            writer: Arc::clone(&self.writer),
            event_sender: self.event_sender.clone(),
            shared: Arc::clone(&self.shared),
            events: Arc::clone(&self.events),
        };
        tokio::spawn(supervisor.run(reader, shutdown_rx));

        Ok(())
    }

    /// Connect to the gateway with automatic reconnection and exponential
    /// backoff
    ///
    /// # Arguments
    /// * `max_retries` - Maximum number of connection attempts before giving up
    ///
    /// # Returns
    /// * `Ok(())` - Successfully connected
    /// * `Err(...)` - Failed to connect after max_retries attempts
    ///
    /// # Backoff Strategy
    /// - Base delay: 1 second
    /// - Maximum delay: 60 seconds
    /// - Exponential multiplier: 2x after each failure
    /// - Added jitter: random 0-500ms to prevent thundering herd
    pub async fn connect_with_auto_reconnect(&mut self, max_retries: u32) -> Result<()> {
        let base_delay_secs = 1u64;
        let max_delay_secs = 60u64;

        for attempt in 0..max_retries {
            match self.connect().await {
                Ok(_) => {
                    info!("Gateway connected successfully on attempt {}", attempt + 1);
                    return Ok(());
                }
                Err(e) => {
                    if attempt + 1 >= max_retries {
                        error!(error = %e, max_retries = max_retries, "Failed to connect, giving up");
                        return Err(e);
                    }

                    // Calculate exponential backoff delay
                    let delay_secs = std::cmp::min(base_delay_secs * 2u64.pow(attempt), max_delay_secs);

                    // Add jitter (0-500ms) to prevent thundering herd
                    let jitter_ms = rand::random::<u64>() % 500;
                    let total_delay = Duration::from_secs(delay_secs) + Duration::from_millis(jitter_ms);

                    warn!("Connection attempt {} failed: {}. Retrying in {:?}", attempt + 1, e, total_delay);

                    tokio::time::sleep(total_delay).await;
                }
            }
        }

        Err(crate::error::DiscordError::MaxRetriesExceeded)
    }

    /// Send a Voice State Update (opcode 4) to join, move, or leave a voice
    /// channel.
    ///
    /// Pass `channel_id = None` to disconnect from any voice channel in the
    /// guild. Pass `self_mute = true` / `self_deaf = true` to mute/deafen
    /// yourself locally.
    ///
    /// Mirrors serenity's `VoiceGatewayManager::join()`; Discord responds with
    /// a `VOICE_STATE_UPDATE` dispatch event containing the assigned session.
    pub async fn send_voice_state_update(&self, guild_id: u64, channel_id: Option<u64>, self_mute: bool, self_deaf: bool) -> Result<()> {
        let payload = json!({
            "op": Opcode::VoiceStateUpdate as u8,
            "d": {
                "guild_id": guild_id.to_string(),
                "channel_id": channel_id.map(|id| id.to_string()),
                "self_mute": self_mute,
                "self_deaf": self_deaf,
            }
        });
        if let Some(ref mut w) = *self.writer.write().await {
            w.send(WsMessage::Text(payload.to_string().into())).await?;
        }
        Ok(())
    }

    /// Send a presence update (status change)
    pub async fn send_presence(&self, status: UserStatus) -> Result<()> {
        let presence = json!({
            "op": 3,
            "d": {
                "status": status.as_str(),
                "since": 0,
                "activities": [],
                "afk": false
            }
        });

        if let Some(ref mut w) = *self.writer.write().await {
            w.send(WsMessage::Text(presence.to_string().into())).await?;
        }

        Ok(())
    }

    /// Send a Request Guild Members (opcode 8) payload.
    ///
    /// Mirrors Discord's documented op-8 shape:
    /// ```json
    /// { "guild_id": "...", "query": "ab", "limit": 0,
    ///   "presences": false, "user_ids": ["..."], "nonce": "..." }
    /// ```
    /// Pass `query = None` to use `user_ids` instead, or `query = Some("")`
    /// with `limit = 0` to request the full member list (subject to Discord's
    /// large-guild rules).
    ///
    /// All optional fields are omitted from the JSON when `None` so we don't
    /// trip Discord's strict payload validator with stray nulls.
    pub async fn request_guild_members(&self, guild_id: u64, query: Option<&str>, limit: u32, presences: Option<bool>, user_ids: Option<&[u64]>, nonce: Option<&str>) -> Result<()> {
        let mut d = serde_json::Map::new();
        d.insert("guild_id".to_string(), Value::from(guild_id.to_string()));
        if let Some(q) = query {
            d.insert("query".to_string(), Value::from(q));
        }
        d.insert("limit".to_string(), Value::from(limit));
        if let Some(p) = presences {
            d.insert("presences".to_string(), Value::from(p));
        }
        if let Some(ids) = user_ids {
            let arr: Vec<Value> = ids.iter().map(|id| Value::from(id.to_string())).collect();
            d.insert("user_ids".to_string(), Value::from(arr));
        }
        if let Some(n) = nonce {
            d.insert("nonce".to_string(), Value::from(n));
        }

        let payload = json!({
            "op": Opcode::RequestGuildMembers as u8,
            "d": Value::Object(d),
        });
        if let Some(ref mut w) = *self.writer.write().await {
            w.send(WsMessage::Text(payload.to_string().into())).await?;
        }
        Ok(())
    }

    /// Send a Request Soundboard Sounds (opcode 31) payload.
    ///
    /// Discord's docs define the payload as `{ "guild_ids": ["..."] }`
    /// (snowflakes serialised as strings). Discord replies with one
    /// `SOUNDBOARD_SOUNDS` dispatch per guild.
    pub async fn request_soundboard_sounds(&self, guild_ids: &[u64]) -> Result<()> {
        let ids: Vec<Value> = guild_ids.iter().map(|id| Value::from(id.to_string())).collect();
        let payload = json!({
            "op": 31,
            "d": { "guild_ids": ids }
        });
        if let Some(ref mut w) = *self.writer.write().await {
            w.send(WsMessage::Text(payload.to_string().into())).await?;
        }
        Ok(())
    }

    /// Send a Request Channel Info (opcode 43) payload.
    ///
    /// Discord uses this to request channel-info push events. Payload shape:
    /// ```json
    /// { "guild_id": "...", "channel_ids": ["..."] }
    /// ```
    pub async fn request_channel_info(&self, guild_id: u64, channel_ids: &[u64]) -> Result<()> {
        let ids: Vec<Value> = channel_ids.iter().map(|id| Value::from(id.to_string())).collect();
        let payload = json!({
            "op": 43,
            "d": {
                "guild_id": guild_id.to_string(),
                "channel_ids": ids,
            }
        });
        if let Some(ref mut w) = *self.writer.write().await {
            w.send(WsMessage::Text(payload.to_string().into())).await?;
        }
        Ok(())
    }

    /// Send raw JSON payload
    pub async fn send_raw(&self, payload: Value) -> Result<()> {
        if let Some(ref mut w) = *self.writer.write().await {
            w.send(WsMessage::Text(payload.to_string().into())).await?;
        }
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.writer.read().await.is_some()
    }

    /// Disconnect from gateway
    pub async fn disconnect(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(ref mut w) = *self.writer.write().await {
            let _ = w.close().await;
        }
        *self.writer.write().await = None;
        self.shared().stage = ConnectionStage::Disconnected;
    }

    /// Get current connection stage
    pub async fn stage(&self) -> ConnectionStage {
        self.shared().stage
    }

    /// Return the round-trip gateway latency measured from the most recent
    /// heartbeat/ACK pair.  Returns `None` until the first ACK is received.
    ///
    /// Mirrors serenity's `Shard::latency()`.
    pub async fn latency(&self) -> Option<Duration> {
        self.shared().latency
    }

    /// Return whether the next reconnection should attempt a RESUME or a full
    /// REIDENTIFY.  Mirrors serenity's `reconnection_type()`.
    ///
    /// Returns `Resume` when a `session_id` is cached from the last READY
    /// event, `Reidentify` otherwise.
    pub async fn reconnection_type(&self) -> ReconnectType {
        if self.shared().has_session {
            ReconnectType::Resume
        } else {
            ReconnectType::Reidentify
        }
    }
}

/// The task that owns the socket's read half and the [`GatewayCore`].
///
/// It reads frames, wakes at the core's deadlines, carries out the core's
/// actions, and reconnects when a connection ends.
struct Supervisor {
    core: GatewayCore,
    /// The token the gateway was built with, handed to [`GatewayHooks::token`].
    configured_token: String,
    hooks: Arc<dyn GatewayHooks>,
    identify: IdentifyDefaults,
    writer: Arc<RwLock<Option<WsWriter>>>,
    event_sender: broadcast::Sender<DispatchEvent>,
    shared: Arc<Mutex<Shared>>,
    events: Arc<Mutex<EventSlot>>,
}

impl Supervisor {
    /// Run until shutdown or until the close policy says stop.
    async fn run(mut self, initial_reader: WsReader, mut shutdown_rx: mpsc::Receiver<()>) {
        // The first iteration uses the reader from the initial connect();
        // subsequent iterations re-establish the WebSocket here.
        let mut current_reader: Option<WsReader> = Some(initial_reader);
        let stop_detail: String;

        'supervisor: loop {
            // ── 1. Acquire a reader: initial, or a reconnect ──
            let mut reader = match current_reader.take() {
                Some(r) => r,
                None => {
                    info!("Reconnecting to Discord gateway…");
                    self.core.connecting();
                    self.publish();
                    self.status(GatewayStatus::Connecting, None);
                    // Drop any stale writer before opening a new socket.
                    *self.writer.write().await = None;

                    let connected = match self.hooks.token(&self.configured_token).await {
                        Some(token) => {
                            self.core.set_token(token);
                            connect_async(self.core.connect_url()).await.map_err(|e| e.to_string())
                        }
                        None => Err("no gateway credential available".to_string()),
                    };
                    match connected {
                        Ok((ws_stream, _)) => {
                            let (w, r) = ws_stream.split();
                            *self.writer.write().await = Some(w);
                            info!("Gateway reconnected successfully");
                            self.core.opened(Instant::now());
                            r
                        }
                        Err(e) => {
                            let wait = self.reconnect_delay(ReconnectCause::Failure);
                            error!(error = %e, ?wait, "Gateway reconnect failed; waiting before retry");
                            tokio::select! {
                                _ = shutdown_rx.recv() => {
                                    stop_detail = "shutdown".to_string();
                                    break 'supervisor;
                                }
                                _ = tokio::time::sleep(wait) => {}
                            }
                            continue 'supervisor;
                        }
                    }
                }
            };

            // ── 2. Read until the connection ends ──
            let outcome = self.read_connection(&mut reader, &mut shutdown_rx).await;

            // ── 3. Decide what to do next ──
            let cause = match outcome {
                GatewayLoopOutcome::Shutdown => {
                    self.core.connection_lost(false);
                    stop_detail = "shutdown".to_string();
                    break 'supervisor;
                }
                GatewayLoopOutcome::Closed { code, reason } => {
                    if code == Some(4004) {
                        self.hooks.invalidate();
                        self.status(GatewayStatus::AuthFailed, Some(format!("close 4004: {reason}")));
                    }
                    match self.hooks.on_close(code, &reason) {
                        CloseVerdict::Resume => ReconnectCause::Resume,
                        CloseVerdict::Identify => ReconnectCause::Identify,
                        CloseVerdict::Retry => ReconnectCause::Failure,
                        CloseVerdict::Stop(why) => {
                            error!(close_code = ?code, reason = %why, "Gateway closed; not reconnecting");
                            self.core.connection_lost(false);
                            stop_detail = why;
                            break 'supervisor;
                        }
                    }
                }
                GatewayLoopOutcome::Reconnect(cause) => cause,
            };
            self.core.connection_lost(cause == ReconnectCause::Identify);
            self.publish();

            // Every cause advances the backoff ladder; only a connection that
            // has proven stable resets it (see `plan_reconnect`).
            let wait = self.reconnect_delay(cause);
            info!(?cause, ?wait, "Gateway connection ended; reconnecting");
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    stop_detail = "shutdown".to_string();
                    break 'supervisor;
                }
                _ = tokio::time::sleep(wait) => {}
            }
            // current_reader stays None — the loop reconnects.
        }

        // Nothing will write to this socket again, and `is_connected()` reads
        // the writer, so clear it before reporting the stop.
        if let Some(mut w) = self.writer.write().await.take() {
            let _ = w.close().await;
        }
        self.publish();
        let sink = self.events.lock().unwrap_or_else(|e| e.into_inner()).sink.take();
        if let Some(sink) = sink {
            sink.finish(Some(stop_detail)).await;
        }
        info!("Gateway supervisor task exiting");
    }

    /// Read one connection until it ends, running the core's actions.
    async fn read_connection(&mut self, reader: &mut WsReader, shutdown_rx: &mut mpsc::Receiver<()>) -> GatewayLoopOutcome {
        loop {
            let deadline = self.core.next_deadline();
            let wake = async {
                match deadline {
                    Some(at) => tokio::time::sleep_until(at.into()).await,
                    None => std::future::pending().await,
                }
            };

            let ended = tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Gateway shutdown requested");
                    Some(GatewayLoopOutcome::Shutdown)
                }
                msg = reader.next() => match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        let actions = self.core.frame(text.as_str(), Instant::now(), rand::random());
                        self.run_actions(actions, Some(text.as_str())).await
                    }
                    Some(Ok(WsMessage::Binary(bytes))) => {
                        // Discord may send zlib-compressed JSON payloads as binary frames.
                        match Gateway::decompress_zlib(&bytes) {
                            Ok(text) => {
                                let actions = self.core.frame(&text, Instant::now(), rand::random());
                                self.run_actions(actions, Some(&text)).await
                            }
                            Err(e) => {
                                warn!("Failed to decompress binary gateway frame ({} bytes): {}", bytes.len(), e);
                                None
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(frame))) => {
                        match &frame {
                            Some(f) => info!("Gateway connection closed: code={} reason={}", f.code, f.reason),
                            None => info!("Gateway connection closed"),
                        }
                        let code = frame.as_ref().map(|f| u16::from(f.code));
                        let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                        Some(GatewayLoopOutcome::Closed { code, reason })
                    }
                    Some(Err(e)) => {
                        error!(error = %e, "WebSocket error");
                        Some(GatewayLoopOutcome::Reconnect(ReconnectCause::Failure))
                    }
                    None => {
                        info!("WebSocket stream ended");
                        Some(GatewayLoopOutcome::Reconnect(ReconnectCause::Failure))
                    }
                    _ => None,
                },
                _ = wake => {
                    let actions = self.core.tick(Instant::now(), rand::random());
                    self.run_actions(actions, None).await
                }
            };

            if let Some(outcome) = ended {
                return outcome;
            }
        }
    }

    /// Carry out the core's actions. `raw` is the frame that produced them,
    /// if any. Returns an outcome when one of the actions ends the
    /// connection.
    async fn run_actions(&mut self, actions: Vec<Action>, raw: Option<&str>) -> Option<GatewayLoopOutcome> {
        for action in actions {
            match action {
                Action::Received { op, s, t } => {
                    if let Some(raw) = raw {
                        with_sink(&self.events, |sink| sink.emit(GatewayEvent::Frame { op, s, t, raw: raw.to_string() }));
                    }
                }
                Action::Live => self.status(GatewayStatus::Live, None),
                Action::Send(text) => {
                    if let Err(e) = send_text(&self.writer, text).await {
                        error!(error = %e, "Failed to send gateway frame");
                    }
                }
                Action::Identify => {
                    let d = self.hooks.identify(self.core.token(), &self.identify).await;
                    match send_text(&self.writer, json!({ "op": 2, "d": d }).to_string()).await {
                        Ok(()) => info!("Sent identify payload"),
                        Err(e) => error!(error = %e, "Failed to send identify payload"),
                    }
                }
                Action::Dispatch { event_type, data } => {
                    if let Err(e) = self.event_sender.send(DispatchEvent::new(event_type.clone(), data)) {
                        warn!("Failed to dispatch event '{}': no active receivers ({})", event_type, e);
                    }
                }
                Action::Reconnect { cause, reason } => {
                    warn!(?cause, reason, "Ending gateway connection");
                    self.publish();
                    return Some(GatewayLoopOutcome::Reconnect(cause));
                }
            }
        }
        self.publish();
        None
    }

    /// The wait before the next connect, from the caller's pacing hook.
    fn reconnect_delay(&mut self, cause: ReconnectCause) -> Duration {
        let hooks = &self.hooks;
        self.core.reconnect_delay(cause, rand::random(), |cause, backoff, held, rand| hooks.delay(cause, backoff, held, rand))
    }

    fn status(&self, status: GatewayStatus, detail: Option<String>) {
        with_sink(&self.events, |sink| sink.status(status, detail));
    }

    /// Copy the core's state into the snapshot the public getters read.
    fn publish(&self) {
        let mut shared = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        shared.stage = self.core.stage();
        shared.latency = self.core.latency();
        shared.has_session = self.core.session_id().is_some();
    }
}

impl Gateway {
    /// Decompress a zlib-compressed binary WebSocket frame into a UTF-8 string.
    ///
    /// Discord sends zlib-compressed gateway payloads as raw deflate or zlib
    /// streams depending on whether the `?compress=zlib-stream` query parameter
    /// was negotiated.  We handle both deflate and zlib wrappers here.
    fn decompress_zlib(bytes: &[u8]) -> std::result::Result<String, String> {
        use std::io::Read;

        // Try zlib (deflate with zlib header — the most common Discord format)
        let mut decoder = flate2::read::ZlibDecoder::new(bytes);
        let mut out = String::new();
        if decoder.read_to_string(&mut out).is_ok() {
            return Ok(out);
        }

        // Fall back to raw deflate (no zlib header)
        let mut decoder = flate2::read::DeflateDecoder::new(bytes);
        let mut out = String::new();
        decoder.read_to_string(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());
        }
    }
}
