//! Event system for Discord dispatch events

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock, RwLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::{DashMap, DashSet};
use serde_json::Value;

/// Spawn a listener callback on its own task.
///
/// Under `--cfg tokio_unstable` (this repository's `.cargo/config.toml`) the
/// task is named, so it shows up in tokio-console. A dependent crate does not
/// inherit that flag, and `tokio::task::Builder` does not exist without it, so
/// everywhere else this is a plain `tokio::spawn`.
fn spawn_listener(name: &str, task: impl std::future::Future<Output = ()> + Send + 'static) {
    #[cfg(tokio_unstable)]
    {
        let _ = tokio::task::Builder::new().name(name).spawn(task);
    }
    #[cfg(not(tokio_unstable))]
    {
        let _ = name;
        tokio::spawn(task);
    }
}

/// A dispatch event from the gateway.
///
/// `seq` is a process-wide monotonic counter assigned at gateway-read time
/// (before fan-out to listeners). Because `EventEmitter::dispatch` spawns each
/// listener callback on its own tokio task, listeners that persist state may
/// otherwise apply two events for the same logical entity in the wrong order.
/// Use `seq` as a write-time filter (e.g. only `update` if `stored.seq < seq`)
/// to make stale, out-of-order writes silently no-op.
#[derive(Debug, Clone)]
pub struct DispatchEvent {
    pub event_type: String,
    pub data: Value,
    /// Monotonic sequence number. Strictly increasing within the lifetime of
    /// a single process. Across restarts, monotonic in expectation: the
    /// counter is seeded from `SystemTime::now()` at first use, so a fresh
    /// process emits seqs strictly greater than those of any prior process
    /// (modulo clock travel). Two events with `a.seq < b.seq` were observed
    /// in that order on the gateway socket.
    pub seq: u64,
}

impl DispatchEvent {
    /// Construct a dispatch event with a freshly-allocated sequence number.
    /// Prefer this over the struct literal so `seq` stays monotonic.
    pub fn new(event_type: String, data: Value) -> Self {
        Self { event_type, data, seq: next_event_seq() }
    }
}

/// Process-wide event sequence counter. Assigned at gateway-read time so a
/// single ordering exists even when multiple subsystems construct events.
///
/// **Seeding.** The first `next_event_seq` call lazily initializes the counter
/// to the current wall-clock time in nanoseconds since the Unix epoch.
/// Reasoning: consumers that persist `seq` (e.g. as a Mongo write-filter
/// guard) need cross-restart monotonicity. A counter that starts at zero on
/// every process restart would silently fail every write whose stored `seq`
/// is greater than the new in-memory counter — and since the unique
/// `(guild_id, user_id)` index also blocks the upsert fallback, those
/// writes would silently disappear.
///
/// Seeding from wall-clock means a fresh process always starts above any
/// seq written by a prior process (assuming the clock didn't move backwards
/// by years). Within a process, `fetch_add` keeps the counter strictly
/// monotonic. The seed is captured at first call, not at static init, so
/// tests can manipulate it via `OnceLock::set_for_tests`-style hacks if
/// needed and so we don't pay the syscall cost until the first event.
static EVENT_SEQ: OnceLock<AtomicU64> = OnceLock::new();

fn event_seq_counter() -> &'static AtomicU64 {
    EVENT_SEQ.get_or_init(|| {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
        AtomicU64::new(seed)
    })
}

/// Allocate the next monotonic event sequence number.
///
/// Strictly increasing within a process. Across restarts, monotonic in
/// expectation: the counter is seeded from the wall-clock time at first
/// call, so a fresh process produces seqs strictly greater than those
/// emitted before its launch (modulo clock travel). Exposed so external
/// gateways or tests can produce ordered events without going through
/// `DispatchEvent::new`.
pub fn next_event_seq() -> u64 {
    event_seq_counter().fetch_add(1, Ordering::Relaxed)
}

/// Type alias for event callback
pub type EventCallback = Arc<dyn Fn(DispatchEvent) + Send + Sync>;

/// Event listener with unique ID
#[derive(Clone)]
struct EventListener {
    id: String,
    callback: EventCallback,
}

/// Counter for unique listener IDs
static LISTENER_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_listener_id() -> String {
    let id = LISTENER_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("listener_{}", id)
}

/// Event emitter managing subscriptions to Discord events
pub struct EventEmitter {
    /// Event-specific listeners: event_type -> listeners
    /// Using DashMap for high-concurrency access (sharded locking)
    private_events: Arc<DashMap<String, Vec<EventListener>>>,
    /// Listeners that receive all events
    any_events: Arc<RwLock<Vec<EventListener>>>,
    /// Listeners for unhandled events
    unhandled_events: Arc<RwLock<Vec<EventListener>>>,
    /// Set of handled event types
    handled_types: Arc<DashSet<String>>,
}

/// RAII subscription guard for event listeners
///
/// When this struct is dropped, the listener is automatically removed.
pub struct EventSubscription {
    id: String,
    emitter: EventEmitter,
}

impl EventSubscription {
    /// Get the listener ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Detach the subscription, preventing automatic removal on drop
    /// Returns the listener ID
    pub fn detach(self) -> String {
        let id = self.id.clone();
        std::mem::forget(self);
        id
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        self.emitter.remove_listener(&self.id);
    }
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new() -> Self {
        Self {
            private_events: Arc::new(DashMap::new()),
            any_events: Arc::new(RwLock::new(Vec::new())),
            unhandled_events: Arc::new(RwLock::new(Vec::new())),
            handled_types: Arc::new(DashSet::new()),
        }
    }

    /// Register a listener for a specific event
    ///
    /// Returns a subscription guard that removes the listener when dropped
    pub async fn on_event<F>(&self, event_name: &str, callback: F) -> EventSubscription
    where
        F: Fn(DispatchEvent) + Send + Sync + 'static,
    {
        let id = generate_listener_id();
        let listener = EventListener { id: id.clone(), callback: Arc::new(callback) };

        // DashMap handles locking internally per shard
        self.private_events.entry(event_name.to_string()).or_default().push(listener);

        self.handled_types.insert(event_name.to_string());

        EventSubscription { id, emitter: self.clone() }
    }

    /// Register a listener for multiple events (space-separated)
    pub async fn on_events<F>(&self, event_names: &str, callback: F) -> Vec<EventSubscription>
    where
        F: Fn(DispatchEvent) + Send + Sync + Clone + 'static,
    {
        let mut subscriptions = Vec::new();
        for name in event_names.split_whitespace() {
            let sub = self.on_event(name, callback.clone()).await;
            subscriptions.push(sub);
        }
        subscriptions
    }

    /// Register a listener for all events ("firehose")
    pub async fn on_any_event<F>(&self, callback: F) -> EventSubscription
    where
        F: Fn(DispatchEvent) + Send + Sync + 'static,
    {
        let id = generate_listener_id();
        let listener = EventListener { id: id.clone(), callback: Arc::new(callback) };

        self.any_events.write().unwrap_or_else(|e| e.into_inner()).push(listener);
        EventSubscription { id, emitter: self.clone() }
    }

    /// Register a listener for unhandled events
    pub async fn on_unhandled_event<F>(&self, callback: F) -> EventSubscription
    where
        F: Fn(DispatchEvent) + Send + Sync + 'static,
    {
        let id = generate_listener_id();
        let listener = EventListener { id: id.clone(), callback: Arc::new(callback) };

        self.unhandled_events.write().unwrap_or_else(|e| e.into_inner()).push(listener);
        EventSubscription { id, emitter: self.clone() }
    }

    /// Remove a listener by ID (synchronous — safe to call from Drop)
    pub fn remove_listener(&self, listener_id: &str) -> bool {
        // Check private events (DashMap is already synchronous)
        for mut r in self.private_events.iter_mut() {
            let listeners = r.value_mut();
            if let Some(pos) = listeners.iter().position(|l| l.id == listener_id) {
                listeners.remove(pos);
                return true;
            }
        }

        // Check any events — acquire write lock once to avoid TOCTOU race
        {
            let mut any = self.any_events.write().unwrap_or_else(|e| e.into_inner());
            if let Some(pos) = any.iter().position(|l| l.id == listener_id) {
                any.remove(pos);
                return true;
            }
        }

        // Check unhandled events — same pattern
        {
            let mut unhandled = self.unhandled_events.write().unwrap_or_else(|e| e.into_inner());
            if let Some(pos) = unhandled.iter().position(|l| l.id == listener_id) {
                unhandled.remove(pos);
                return true;
            }
        }

        false
    }

    /// Remove a listener by ID
    pub async fn off_event(&self, listener_id: &str) -> bool {
        self.remove_listener(listener_id)
    }

    /// Remove all listeners for a specific event by name (space-separated names
    /// supported)
    pub async fn off_event_by_name(&self, event_names: &str) {
        for name in event_names.split_whitespace() {
            self.private_events.remove(name);
            self.handled_types.remove(name);
        }
    }

    /// Remove all listeners for a specific event
    pub async fn off_all(&self, event_name: &str) {
        self.private_events.remove(event_name);
        self.handled_types.remove(event_name);
    }

    /// Dispatch an event to all registered listeners.
    ///
    /// Each listener callback is spawned as a named tokio task, making them
    /// visible in `tokio-console` under the label
    /// `dispatch::event_handler::{event_type}`.
    pub async fn dispatch(&self, event: DispatchEvent) {
        let event_type = event.event_type.clone();

        // 1. Dispatch to event-specific listeners.
        let specific_listeners = self.private_events.get(&event_type).map(|l| l.clone());
        if let Some(listeners) = specific_listeners {
            for listener in listeners {
                let ev = event.clone();
                let task_name = format!("dispatch::event_handler::{}", event_type);
                spawn_listener(&task_name, async move {
                    (listener.callback)(ev);
                });
            }
        }

        // 2. Dispatch to any-event listeners.
        let any_listeners = self.any_events.read().unwrap_or_else(|e| e.into_inner()).clone();
        for listener in any_listeners {
            let ev = event.clone();
            spawn_listener("dispatch::event_handler::any", async move {
                (listener.callback)(ev);
            });
        }

        // 3. Dispatch to unhandled if no specific handler was registered.
        if !self.handled_types.contains(&event_type) {
            let unhandled_listeners = self.unhandled_events.read().unwrap_or_else(|e| e.into_inner()).clone();
            for listener in unhandled_listeners {
                let ev = event.clone();
                spawn_listener("dispatch::event_handler::unhandled", async move {
                    (listener.callback)(ev);
                });
            }
        }
    }

    /// Check if there are any listeners for an event
    pub async fn has_listeners(&self, event_name: &str) -> bool {
        self.private_events.get(event_name).map(|l| !l.is_empty()).unwrap_or(false)
    }

    /// Get count of listeners for an event
    pub async fn listener_count(&self, event_name: &str) -> usize {
        self.private_events.get(event_name).map(|l| l.len()).unwrap_or(0)
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventEmitter {
    fn clone(&self) -> Self {
        Self {
            private_events: Arc::clone(&self.private_events),
            any_events: Arc::clone(&self.any_events),
            unhandled_events: Arc::clone(&self.unhandled_events),
            handled_types: Arc::clone(&self.handled_types),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_event_seq_is_strictly_monotonic() {
        let a = next_event_seq();
        let b = next_event_seq();
        let c = next_event_seq();
        assert!(b > a, "seq must be strictly increasing: {a} → {b}");
        assert!(c > b, "seq must be strictly increasing: {b} → {c}");
    }

    #[test]
    fn dispatch_event_new_assigns_increasing_seqs() {
        let e1 = DispatchEvent::new("A".to_string(), Value::Null);
        let e2 = DispatchEvent::new("B".to_string(), Value::Null);
        assert!(e2.seq > e1.seq);
    }

    #[test]
    fn seed_is_close_to_wall_clock() {
        // First call lazily seeds from SystemTime; assert the value is in a
        // sane window around now (greater than year 2000, less than year 2100
        // expressed as nanos). This catches regressions where the seed
        // accidentally falls back to 0.
        let seq = next_event_seq();
        const Y2K_NANOS: u64 = 946_684_800_000_000_000; //  2000-01-01 UTC
        const Y2100_NANOS: u64 = 4_102_444_800_000_000_000; // 2100-01-01 UTC
        assert!(seq > Y2K_NANOS, "seq seemed to seed below year-2000 wall-clock: {seq}");
        assert!(seq < Y2100_NANOS, "seq seemed to seed above year-2100 wall-clock: {seq}");
    }
}
