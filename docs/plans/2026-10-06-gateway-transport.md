# Plan: make `Gateway` usable as an embedded transport

Status: proposed, 2026-10-06. Nothing in this plan has been implemented.
Revised the same day after a review against the source.

## Goal

Make `discord_user::gateway::Gateway` correct enough, and open enough, to run as
the socket layer of another program. The first such program is `fms-dash`, which
would load it into Node through a napi-rs addon and use it in place of its own
`backend/src/gatewaysocket.ts` (522 lines).

The guiding split is **mechanism in the library, policy in the caller**. The
library owns the socket, heartbeat, framing, the RESUME url, and decompression.
The caller decides what a close code means, how long to wait before
reconnecting, which credential to present, and what goes in IDENTIFY. Every new
hook ships with a default that reproduces today's behaviour, so `DiscordUser`,
the `discord` CLI (`dc tail`, `dc members`, `dcw presence`) and the examples
behave the same unless they opt in.

## Non-goals

- Moving any domain logic (buffers, read state, member lists, the 4002 blame
  rule) into Rust. Those stay with the caller.
- zstd decoding and a correct `zlib-stream` decoder (see G-13).
- Any change to REST, the cache, the collector or the framework modules.

## How this was verified

Each finding below was read from `src/gateway.rs` at `0ad9f03` (0.6.1). Findings
G-8, G-9 and G-10 were also exercised: a throwaway napi-rs crate
(`~/.cache/fms-dash-napi-spike`, outside both repositories) built `Gateway` into a
Node addon and pushed synthetic dispatches through the library's own broadcast
channel. That spike never opened a socket to Discord.

| Probe | Result |
|---|---|
| Build | 37 s cold release build, 3 s incremental, 1.0 MB `.node` |
| Load | 0.8 ms through `createRequire` from an ESM module |
| Paced delivery (bursts of 100 into a 1024 buffer) | 20000/20000 delivered; 3.3 µs per frame on an idle box, 13.6–23 µs at load average 24 |
| Unpaced burst of 5000 into a 256 buffer | 300–2544 delivered across five runs; the rest dropped with no signal to the consumer |
| Plain dependent crate in `/tmp`, path dependency, `RUSTFLAGS` unset | `cargo check` fails with three E0433 at `events.rs:272`, `:282`, `:292` (G-10) |

## Findings

### Correctness: these affect every user of the library today

**G-1. RESUME goes to the wrong host.** READY carries `resume_gateway_url`, and
the type already models it (`typed_events.rs:28`), but the supervisor always
reconnects to `gateway_url` (`gateway.rs:417`, `:450`). Discord documents that a
RESUME must target `resume_gateway_url`. Sending it elsewhere invites an op 9,
which turns a free RESUME into a budgeted IDENTIFY. READY's value is a bare host,
so the fix must append the same query string the base URL carries
(`?encoding=json&v=9`, plus `compress` when negotiated); `fms-dash` does the same
in `ready()`. A fresh IDENTIFY goes back to the base URL.

**G-2. Zombie detection is dead code.** The heartbeat task has a `break` for "no
ACK for more than two intervals" (`:664`, whose comment says "Break out to
trigger reconnection"), but that branch is never reached. When an ACK is missing
at a tick, the task only warns, sends another heartbeat and refreshes
`last_sent`. The elapsed time it checks is therefore always about one interval,
never more than two. Even if the branch did run, it would end only the heartbeat
task; nothing tells the supervisor. The only thing that cuts a zombie today is
the read timeout of 2× the interval (~82 s). Discord's rule is simpler: if no ACK
has arrived by the next heartbeat tick, close the socket and RESUME.

**G-3. Every connect waits 500 ms inside the read loop.** On HELLO, the handler
sleeps 500 ms before IDENTIFY or RESUME (`:688`). The sleep blocks the read loop
and adds half a second to every connect and every resume. `fms-dash` measured
about 12 op-7 reconnects a day and lowered its own floor to 250 ms for exactly
this cost.

**G-4. Op 9 sleeps 1–5 s inside the read loop, even when resumable**
(`:790`). After the sleep it re-sends on the same socket. The jitter is right for
a non-resumable op 9, because IDENTIFY is budgeted. For a resumable op 9 it is
pure delay, and holding the reader that long means the frame stream stalls with
it.

**G-5. No brake on a READY-then-close loop.** Backoff applies only when the
WebSocket connect itself fails (`:450`), and it resets as soon as a connect
succeeds (`:455`). A connection that opens, identifies and is then closed (4003,
4007, 4009, or any unknown close) comes back after the fixed `RECONNECT_PAUSE`
of 500 ms (`:435`) forever. Each loop without a session is a fresh IDENTIFY. On a
user token that is a daily-budgeted login and a visible session event on the
account. The comment on 4008 (`:183`) says the outer loop already applies
backoff; it does not.

**G-6. A fatal close is silent.** On 4004 and other fatal codes the supervisor
logs and exits (`classify_close`, `:168`). The only trace is `stage()` reading
`Disconnected`, which looks identical to an ordinary drop. A consumer cannot tell
"the token was rejected" from "the network blinked", and nothing announces that
the supervisor has stopped. Worse, the fatal path never clears the writer, so
`is_connected()` (`:1059`, which only checks `writer.is_some()`) keeps returning
`true` after the supervisor has exited.

### Openness: what blocks an embedding caller

**G-7. The token is fixed at construction.** `Gateway::new` takes a `String` and
reuses it for every reconnect. `fms-dash` harvests its token from a live browser
session, re-reads it on every connect, and re-harvests on 4004. There is no hook
for that.

**G-8. Raw frames are not exposed.** Consumers receive only
`DispatchEvent { event_type, data, seq }`, and `seq` is a process-wide counter
(`events.rs:14-37`), not Discord's `s`. HELLO, HEARTBEAT_ACK, RECONNECT and
INVALID_SESSION never reach the caller. A frame recorder (`fms-dash`'s Gateway
monitor tab) needs every frame, and a consumer that tracks resume state needs
`s`.

**G-9. Delivery drops silently under load.** The event channel is a
`tokio::sync::broadcast` (`:252`). A receiver that falls behind gets `Lagged(n)`,
and the events in between are gone. The spike showed this, and showed that load
on the box makes it worse.

**G-10. Downstream crates must set `--cfg tokio_unstable`.** `events.rs:272`,
`:282` and `:292` call `tokio::task::Builder`, which tokio gates on
`all(tokio_unstable, feature = "tracing")`. The `tracing` feature is enabled in
`Cargo.toml`, but the flag lives in this repo's `.cargo/config.toml`, which a
dependent crate does not inherit, so the first build of any dependent fails with
E0433. This was reproduced (see the probe table).

**G-11. IDENTIFY is hardcoded.** `send_identify` (`:856`) always sends Windows /
Chrome 120 (`:870-873`), capabilities 16381, a presence, and a `client_state`
block. `fms-dash` sends `capabilities: 0` with Linux properties. Capabilities
change the shape of READY and of several dispatches, so `fms-dash`'s frame parser
depends on that value. `new_with_capabilities` covers only one field of the
payload.

### Supporting gaps

**G-12. The gateway has no tests.** `gateway.rs` contains no `#[test]`, and
nothing under `tests/` exercises it. Most state transitions sit inside one
18-argument `async fn handle_message`, which cannot be driven without a live
socket. The endpoint is a `const &'static str` (`GATEWAY_URL`, `:31`), so there
is also no way to point the gateway at a test server.

**G-13. Compressed transports do not work.** `zlib-stream` is decoded per frame
(`decompress_zlib`). That stream shares one inflate context across the
connection, so per-frame decoding fails after the first frame. Selecting
`CompressionKind::Zstd` is worse: it negotiates `zstd-stream`, and every binary
frame, HELLO included, is then fed to the zlib decoder and dropped with a
warning, so the connection never identifies. This is out of scope, because
nothing outside `gateway.rs` selects compression. Both variants should be either
fixed or documented.

## Design

### A sans-IO core

Extract the per-connection state machine from `handle_message` and the
heartbeat task into a pure `GatewayCore`. It holds the session id, Discord's `s`,
the resume URL, the heartbeat interval and schedule, ack state and the stage. It
does no I/O, reads no clock and draws no random number of its own: time and
randomness arrive as inputs.

- **Inputs:** `Opened`, `Frame(text)`, `Tick { now, rand }`,
  `Closed { code, reason }`, `ReadTimeout`, `TransportError(detail)`.
- **Outputs:** a list of actions (`Send(text)`, `Reconnect { cause, url }`,
  `Emit(GatewayEvent)`, `Stop(reason)`), plus `next_deadline: Option<Instant>`
  with every result.

The core owns the heartbeat schedule rather than handing it out as a
`StartHeartbeat(ms)` action. A `Tick` at or past the deadline yields the
heartbeat `Send`, or, when the previous heartbeat is still unacknowledged, a
`Reconnect` with a resume cause (G-2). The first heartbeat's jitter comes from
the `rand` in the first `Tick` after HELLO. The supervisor is a thin loop that
reads the socket, sleeps until `next_deadline`, and runs the actions.

This is what makes G-1 through G-6 testable without a socket. It is also the
shape `fms-dash` already uses on its side (`gatewaycore.ts` is pure, and
`gatewaysocket.ts` does the I/O).

### Gateway URLs

`GatewayConfig` takes a base URL, which defaults to today's `GATEWAY_URL` or
`GATEWAY_URL_ZSTD`. The loopback tests set it to `ws://127.0.0.1:<port>`. The
resume URL is a `String` held by the core, built from READY's
`resume_gateway_url` plus the base URL's query string, and cleared together with
the session. `Reconnect { url }` carries the resume URL for a RESUME and the
base URL for an IDENTIFY (G-1).

### Caller hooks on a `GatewayConfig`

All of these are optional, and each default matches 0.6.1 behaviour except
where a finding above says that behaviour is a bug. The hooks live on one trait
held as `Arc<dyn GatewayHooks>`. The crate is on edition 2021, and an `async fn`
in a trait cannot be called through `dyn`, so the asynchronous hook returns a
`BoxFuture` (or uses `async_trait`, whichever the crate settles on).

| Hook | Shape | Default |
|---|---|---|
| Credentials (G-7) | `fn token(&self) -> BoxFuture<'_, Option<String>>` called before every IDENTIFY and RESUME; `fn invalidate(&self)` called on 4004, before `on_close` | The fixed token passed today |
| IDENTIFY body (G-11) | `fn identify(&self, token: &str) -> serde_json::Value` returning the whole `d` object | Today's payload, with capabilities/intents/shard as now |
| Close policy (G-6) | `fn on_close(&self, code: Option<u16>, reason: &str) -> CloseVerdict` where the verdict is `Resume`, `Identify`, `Retry`, or `Stop(reason)` | Today's `classify_close`; 4004 maps to `Stop` with an `AuthFailed` status |
| Reconnect delay (G-3, G-4, G-5) | `fn delay(&self, cause: ReconnectCause, backoff: Duration, session_held: bool, rand: f64) -> (Duration, Duration)`, returning the wait and the next backoff | See below |

Credential rules:

- `token()` returning `None` means "no credential right now". The supervisor
  treats it as a `failure` cause, waits the ladder, and asks again. It does not
  stop.
- If `token()` returns a different value from the one the held session was
  opened with, the supervisor drops the session and IDENTIFYs. A RESUME with a
  different token would only earn an op 9.
- A caller that re-harvests on 4004 (as `fms-dash` does) overrides `on_close`
  to return `Identify` for 4004. `invalidate()` has already run by then, so the
  next `token()` call fetches the fresh credential.

The default reconnect planner starts from `fms-dash`'s `planReconnect`, which is
already pure and table-tested. A `resume` cause with a held session waits 250 ms
when backoff is at the floor. An `identify` cause waits a jittered 1–5 s, or the
current backoff if that is larger. A `failure` cause waits the ladder. The ladder
advances on every cause.

The reset rule deliberately differs from `fms-dash`. `fms-dash` resets the
ladder in `markLive()`, that is on READY or RESUMED (`gatewaysocket.ts:366-368`).
That reset would leave G-5 open: a connection that reaches READY and is then
closed would reset the ladder on every loop. Each `identify` would then wait only
1–5 s, which is roughly 700–3600 IDENTIFYs an hour, far beyond the daily budget.
The library therefore resets the ladder only when a connection has proven
stable: the first HEARTBEAT_ACK received after READY or RESUMED. A socket that
is closed before its first acknowledged heartbeat keeps climbing the ladder. The
`fms-dash` side should adopt the same rule when it switches over.

### One ordered event stream

Add `Gateway::frames()`, which returns an `mpsc::Receiver<GatewayEvent>`. The
receiver can be taken once; a second call returns `None`.

```rust
pub enum GatewayEvent {
    Frame { op: u8, s: Option<u64>, t: Option<String>, raw: String }, // every inbound frame, as received
    Status { status: GatewayStatus, detail: Option<String> },         // only on a change
    Lagged { dropped: u64 },                                          // only if the caller chose a bounded channel
}
pub enum GatewayStatus { Connecting, Live, AuthFailed, Stopped }
```

`Frame` carries the original text, so a caller that parses its own frames (as
`fms-dash` does) never pays for a second serde round trip. `Status` is emitted
only when the value changes, the same rule `fms-dash` uses for `onStatus`.

`AuthFailed` is not terminal by itself. It reports that the token was rejected.
If the close policy returns `Identify`, the next status is `Connecting`. If it
returns `Stop`, `Stopped` follows. `Stopped` is the only terminal status. It is
always the last event before the channel closes, and the supervisor clears the
writer before emitting it, so `is_connected()` agrees with it (G-6).
`ConnectionStage` stays as the finer-grained view behind `stage()`, and
`GatewayStatus` is derived from it.

The caller picks the channel. An unbounded one gives no loss. A bounded one gives
an explicit `Lagged` count, so loss is never silent (G-9). The reader must never
wait on the consumer: it delivers with `try_send`, counts what does not fit, and
emits `Lagged { dropped }` as soon as the channel has room again. Waiting
instead would stall the read loop behind a slow consumer, and the socket would
then die by read timeout, which is worse than a counted drop. The existing
`broadcast<DispatchEvent>` API stays as it is for current users.

### No unstable flag for dependents

Gate the three `tokio::task::Builder` calls on `#[cfg(tokio_unstable)]` and fall
back to `tokio::spawn` otherwise (G-10). `tokio_unstable` is not a cfg rustc
knows, so the gate raises an `unexpected_cfgs` warning, which fails the
`-D warnings` gate. Declare it in `Cargo.toml`:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(tokio_unstable)'] }
```

The repo's own `.cargo/config.toml` can keep the flag for tokio-console.

## Phases

Each phase lands as its own commit with `cargo fmt --check`,
`cargo clippy --all-targets --all-features -- -D warnings` and `cargo test` green.

1. **Test harness (G-12).** Make the base URL configurable (see Gateway URLs).
   Extract `GatewayCore` with no behaviour change. Add unit tests that pin
   today's behaviour, and a loopback integration test that runs a
   `tokio-tungstenite` server on `127.0.0.1` scripted to send HELLO, READY,
   dispatches, op 7, op 9 and closes. Timing tests run under
   `tokio::time::pause()`, so the 0.5 s and 1–5 s sleeps do not slow or flake the
   suite. That needs `test-util` added to the dev-dependency on `tokio`, because
   `full` does not include it. This phase is the safety net for every phase
   after it.
2. **Correctness fixes (G-1 to G-6).** One test per finding, each first written
   to fail against phase 1's pinned behaviour:
   - RESUME targets `resume_gateway_url` with the base query string appended,
     and a fresh IDENTIFY targets the base URL.
   - A heartbeat tick that finds the previous heartbeat unacknowledged
     reconnects with RESUME at once.
   - No sleep after HELLO.
   - A resumable op 9 reconnects on the 250 ms floor without blocking the reader.
   - The backoff ladder applies to every reconnect cause, and a connection
     closed after READY but before its first HEARTBEAT_ACK does not reset it.
   - A fatal close emits `AuthFailed` (for 4004) then `Stopped`, and
     `is_connected()` returns `false` afterwards.
3. **Caller hooks (G-7, G-11) and the event stream (G-8, G-9).** Add the
   `GatewayConfig` hooks and `frames()`. Test each hook through the loopback
   server, and test that every default reproduces phase 2's behaviour. Test that
   a bounded channel reports `Lagged` while the reader keeps answering
   heartbeats.
4. **Dependent builds (G-10).** Prove that a dependent compiles without the
   flag. The check must not inherit this repo's `.cargo/config.toml`. Cargo
   discovers that file from the current directory upwards, and an unset
   `RUSTFLAGS` lets it apply, so a test crate inside the repo tree, or a CI step
   with `RUSTFLAGS` unset, proves nothing. Either build a dependent crate
   generated outside the repository (for example under a temp dir in CI), or
   build with `RUSTFLAGS` explicitly set to the empty string, which overrides
   `build.rustflags`.
5. **Release 0.7.0.** Write the CHANGELOG entry. It must name the default timing
   changes, because they alter observable reconnect timing for current users:
   no sleep after HELLO (G-3), op 9 handling (G-4), the ladder and its reset
   rule (G-5), and the heartbeat-tick zombie cut (G-2). It must also name the
   RESUME-host fix. Document G-13 on both `CompressionKind::Zlib` and
   `CompressionKind::Zstd`, and correct the 4008 comment.

The `fms-dash` side is a separate plan, written once 0.7.0 exists. It would
cover the napi crate, replacing `gatewaysocket.ts`, and keeping the 4002 blame
rule and the subscription flags in TypeScript as a `CloseVerdict` callback.

## Verification and the live-account boundary

- Everything in phases 1–4 runs against the loopback server and fixtures. No test
  reaches `gateway.discord.gg`.
- Fixtures are synthetic by default. `fms-dash`'s recorded gateway frames
  (`discord_gateway_events`) may serve as a reference for frame shape, but they
  must not be committed as they are. They come from the work account in a work
  guild: READY carries the account's email, phone, `session_id` and
  `analytics_token`, and dispatches carry colleagues' messages and member data.
  This repository's remote is a personal GitHub account. Any frame copied from
  that collection is reduced to its structure, with every id, name, token,
  session value and message body replaced by placeholders before it is
  committed. Reading the collection is free on Discord but costs Atlas egress,
  so take one bounded sample.
- A live connect would IDENTIFY as the real work account. It sends no message,
  but it opens a second gateway session and spends from the daily IDENTIFY
  budget. It is not part of this plan's gate and needs an explicit go-ahead if it
  is ever wanted.

## Open questions

1. ~~Should the close-policy hook also receive the close **reason** string?~~
   Resolved: yes. `on_close` and `Closed` both carry it. `fms-dash` does not use
   it today, but it costs nothing to pass.
2. Should `Frame.raw` hold decompressed text when compression is negotiated?
   This plan says yes, which ties into G-13.
3. Should the default IDENTIFY body stay Windows / Chrome 120? Changing it would
   alter the fingerprint of every current CLI session, so this plan leaves it
   alone and only makes it overridable.
4. Is the first HEARTBEAT_ACK after going live the right stability signal for
   resetting the ladder, or should it be a fixed time live (for example 60 s)?
   The ACK is cheaper to test and comes in under one interval (~41 s). A
   server that closes right after acknowledging would still defeat it.
5. Should `Frame` events also cover outbound frames (IDENTIFY, RESUME,
   heartbeats, op 8) for the recorder? If so, IDENTIFY and RESUME carry the
   token and must be redacted before they are emitted.
