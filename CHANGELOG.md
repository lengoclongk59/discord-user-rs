# Changelog

## Unreleased

The REST client stops sending a request twice when the first attempt may
already have been carried out, and its credential can be replaced while it is
in use. Both were found auditing the crate as the transport of a dashboard
that posts as a user account: a POST that timed out could become several
messages, and a rotated token meant a new client and lost rate-limit state.

### Changed — library: when a REST request is sent again

- **A POST or a PATCH is never sent again after a 5xx or a transport failure.**
  It used to be re-sent up to five times on a 5xx, a timeout or a reset
  connection. Discord may already have carried out the first attempt, and
  `send_message` carries no nonce, so one call could post up to five
  messages. GET, HEAD, OPTIONS, PUT and DELETE are still re-sent there.
  A 429 is still waited out and re-sent for every method: Discord carried
  nothing out.
- **`post_multipart` is never re-sent after a transport failure**, for the
  same reason. It is still re-sent after a 429.
- The policy is `client::ResendPolicy`: `IdempotentOnly` (the default,
  described above) or `Never`, which re-sends nothing and returns a 429 as
  `DiscordError::RateLimited`, the bucket still recorded so the next request
  waits for its reset. Set it with `DiscordHttpClient::with_resend_policy` or
  `DiscordUserBuilder::resend_policy`.

### Changed — library: breaking

- `DiscordHttpClient::token()` returns `String` instead of `&str`, because the
  token can now be replaced while the client is shared.

### Fixed — library

- A PUT, POST or PATCH answered 204 with an empty body no longer returns
  `DiscordError::Json` through `put::<()>` and its siblings. Pinning a
  message, adding a reaction and joining or adding to a thread all failed
  this way after Discord had carried them out. An empty body now reads as
  JSON `null`, which `()` and `Option<_>` accept.
- A configured `content-type` header is no longer sent beside a multipart
  request's own `multipart/form-data; boundary=…`, which made two.
- A response body that fails to parse is logged by its length, not its text.

### Added — library

- `DiscordHttpClient::set_token` and `replace_headers` replace the credential
  for every later request on the client and all its clones, keeping the
  rate-limit buckets. `replace_headers` takes the token from the
  `authorization` entry, as `with_headers` does, and refuses a map without
  one.
- `DiscordHttpClient::with_api_base` sends relative routes somewhere other
  than `client::API_BASE` (now public): a stand-in server in a test, or a
  recording proxy.
- `tests/rest_loopback.rs` drives the client against a scripted HTTP server
  on 127.0.0.1 and counts how many times each call was sent.

## 0.7.0

The gateway can now run as the socket layer of another program: a sans-IO
core, caller hooks for policy, and an ordered stream of raw frames. Several
reconnect defaults change with it; they are listed first because they alter
observable timing for every current user. Plan and findings:
`docs/plans/2026-10-06-gateway-transport.md`.

### Changed — library: reconnect timing

- **HELLO is answered at once.** IDENTIFY or RESUME used to follow a fixed
  500 ms pause, which also blocked the read loop.
- **An unacknowledged heartbeat ends the connection at the next heartbeat
  tick.** The zombie check never fired before, so a dead socket lasted until
  the read timeout of two heartbeat intervals (about 82 s).
- **Op 9 reconnects instead of waiting on the reader.** A resumable op 9 now
  reconnects and RESUMEs (250 ms when the backoff is at its floor). A
  non-resumable one drops the session and reconnects with IDENTIFY after a
  jittered 1–5 s, or the backoff if longer. Previously both waited 1–5 s
  inside the read loop and answered on the same socket.
- **Every reconnect goes through one backoff ladder (1 s doubling to 60 s).**
  The fixed 500 ms pause between connections is gone. A close that keeps the
  session RESUMEs after 250 ms while the ladder is at its floor; a socket
  error, a missed heartbeat ACK, a read timeout, a refused connect or close
  4008 waits the ladder; a
  session-ending close waits the IDENTIFY window above. The ladder advances
  on every reconnect and resets only at the first heartbeat ACK after READY
  or RESUMED — not at READY itself — so a connection that reaches READY and
  is then closed keeps slowing down instead of re-IDENTIFYing every 500 ms.
  The pure planner is public as `gateway::plan_reconnect`.

### Fixed — library

- RESUME now targets READY's `resume_gateway_url` (with the base URL's query
  appended) instead of the base gateway URL, which invited an op 9 and turned
  a free RESUME into an IDENTIFY. A fresh IDENTIFY still uses the base URL.
- After a fatal close, `Gateway::is_connected()` returned `true` because the
  writer outlived the supervisor. The writer is now cleared.
- RESUME strips a `Bot ` / `Bearer ` prefix from the token, as IDENTIFY
  already did.
- A heartbeat request from Discord (op 1) is answered at once. It used to be
  ignored until the next scheduled heartbeat. The schedule restarts from the
  requested beat.
- The synthetic `CACHE_READY` dispatch now follows the event that completes
  the cache (READY with no guilds, or the last `GUILD_CREATE`). It used to be
  sent just before that event, so a subscriber heard "ready" before the last
  guild it covered.
- Crates that depend on this one no longer fail to build with E0433 in
  `events.rs`. Named listener tasks (`tokio::task::Builder`) are now used
  only under `--cfg tokio_unstable`; without it, listeners are spawned with
  `tokio::spawn`. CI checks a dependent crate built outside the repository.

### Added — library

- `GatewayHooks`, installed with `Gateway::with_hooks`: `token` (asked before
  every connect; a changed token drops the session), `invalidate` (on 4004),
  `identify` (the whole IDENTIFY body), `on_close` (returns a `CloseVerdict`:
  `Resume`, `Identify`, `Retry` or `Stop`) and `delay` (reconnect pacing).
  Every method defaults to the library's behaviour (`DefaultHooks`,
  `default_close_verdict`, `plan_reconnect`, `IdentifyDefaults`).
- `Gateway::frames(FrameBuffer)`: one ordered stream of every inbound frame
  (raw text, opcode, Discord's `s` and `t`) and every status change
  (`Connecting`, `Live`, `AuthFailed`, `Stopped`). The reader never waits on
  it; a bounded stream reports what it dropped as `Lagged`. The existing
  `DispatchEvent` broadcast is unchanged.
- `Gateway::with_url` (base URL override, for tests and proxies),
  `Gateway::with_client_build_number` (pin the IDENTIFY build number instead
  of fetching it), `Gateway::identify_defaults`.

### Known limitations

- Compressed transports do not work. `CompressionKind::Zlib` negotiates
  nothing, and a `zlib-stream` requested through `with_url` is not decoded
  reliably: depending on the `flate2` version, a frame is rejected or, once it
  refers back to earlier frames, decodes to corrupted text.
  `CompressionKind::Zstd` cannot decode any frame. Both are now documented on
  `CompressionKind`, and a test pins the `zlib-stream` behaviour.

### Fixed — CLI (`discord` binary)

- `discord download` now treats `\` as a path separator on every platform
  when it derives a local file name. On Linux and macOS an attachment named
  `..\evil.exe` was saved under that literal name, backslash included: inside
  the download directory, but a traversal for anything that later reads the
  directory from Windows. It is now saved as `evil.exe`.

### Changed — CLI (`discord` binary)

- **The local archive is now per-user, not per-directory.** With no `--db`, the
  SQLite path resolves to `$DISCORD_DB`, else `~/.discord/discord.db`
  (`%USERPROFILE%\.discord\discord.db` on Windows; `./discord.db` only when no
  home directory can be determined). The parent directory is created on first
  open. `DISCORD_DB` is read from the process environment only — it is resolved
  before `.env` is loaded, so a `DISCORD_DB=` line there has no effect.

  Previously the default was `./discord.db`, relative to the working directory,
  so a `cd` silently switched to a different — usually empty — archive, and
  `sync`/`search` appeared to lose data. An archive written by an earlier
  version is not migrated: move the file to `~/.discord/discord.db`, or point
  `$DISCORD_DB` at it. To make that transition visible rather than silent, the
  CLI now warns on stderr when it starts in a directory holding a `./discord.db`
  it is not using.

- **A leading `~` is expanded in `--db` and `$DISCORD_DB`.** A shell expands it
  before the value arrives, but a quoted flag, a `.env` line, or a systemd unit
  does not — and since missing parents are created, an unexpanded `~/a.db`
  produced a directory literally named `~` in the working directory. `~user` is
  left untouched, as resolving it needs a passwd lookup.

- **A directory or archive the CLI creates is owner-only** (`0700` / `0600` on
  Unix), matching the treatment of `.env`: the archive stores message bodies.
  The file is pre-created before SQLite opens it, so the derived `-wal` and
  `-shm` siblings inherit the same mode. Pre-existing paths are left alone —
  their permissions may have been widened deliberately.

## 0.6.1

### Fixed — library

- `send_files` and the `MessageBuilder` send path added the user-account
  `mobile_network_type` fingerprint unconditionally, leaking it under
  application (bot) tokens; Discord rejects those requests with `403` code
  `40333`. Both now gate on the token type, matching `send_message` /
  `reply_to_message`.

### Changed — CLI

- The write layer unified on the canonical `is_application_token()` helper
  (was a divergent `is_app_token()` with inconsistent call sites).

## 0.6.0

CLI client-parity pass — closes many gaps against the official Discord client.
Library API is backward compatible except for one additive trait method.

### Added — library

- `StatusOps::set_custom_status_with_emoji` — set a live custom status with a
  unicode or custom (`name:id`) emoji over the gateway.

### Added — CLI (`discord` binary)

- **Bulk ban**: `dc ban bulk <guild> <id,id,…>` (up to 200 users).
- **Member prune**: `dc member prune-count` and `dc member prune`.
- **Member search**: `dc member search <guild> <query>`.
- **Audit-log filters**: `dc audit … --user --action-type --before --after`.
- **Send options**: `dc send … --tts --silent --suppress-embeds --no-mentions --nonce`.
- **Rich embeds**: `dc compose embed` now supports `--footer(-icon)`,
  `--author(-url/-icon)`, `--image`, `--thumbnail`, `--timestamp`, and
  repeatable `--field "name|value|inline"`.
- **Thread types**: `dc thread create --type public|private|announcement`.
- **Forum posts**: `dc thread forum-post <chan> <name> <message> --tag <id>`.
- **Remote search filters**: `dc search … --author --mentions --has --pinned
  --before --after` (keyword now optional).
- **Presence**: `competing` activity type, and `--emoji` for custom status.
- **Notification settings**: `dc notify guild` / `dc notify channel`
  (mute, level, suppress @everyone/roles, per-channel overrides).
- **Read-state**: `dc notify read-channel` / `dc notify read-guild`.
- **User notes**: `dc account note-get` / `note-set`.
- **DM listing**: `dc dms`.
- **Friend nickname**: `dc friend nickname <user> <nick>`.
- **Account**: `dc account edit --global-name`; connections management
  (`dc account connections` / `connection-edit` / `connection-remove`).

## 0.5.0

Full Discord client CLI: attachments, presence, group DMs, automod, soundboard,
stage, application commands, and 15 nested command groups.
