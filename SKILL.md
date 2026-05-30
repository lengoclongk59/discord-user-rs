---
name: discord-cli
description: Use when working in the discord-user-rs repo (or any clone of it) and you need to build, run, or interpret the bundled `discord` CLI client — a full-featured Discord client (send/edit/react/threads/roles/bans/invites/webhooks/…) plus a local SQLite archive for offline querying. Covers token setup, syncing, querying, and every read & write command.
---

# discord-user-rs CLI (`discord` binary)

## Overview

`discord` is the full-featured Discord **client CLI** bundled with the `discord-user-rs` library. It logs into Discord with a **user token** (self-bot — violates Discord ToS, use at your own risk) or a **bot/bearer token**, and does two things:

1. **Acts as a client** — send/edit/delete/react to messages, DMs, threads, channels, roles, bans, invites, emojis, stickers, webhooks, scheduled events, relationships, polls, embeds, etc. (`dc` write commands).
2. **Archives + queries offline** — syncs guilds/channels into a local SQLite DB (`discord.db`) and runs search/stats/export/timeline over it with no network.

### Read vs write architecture (for maintainers)

- **Reads** go through `Api`, which wraps the HTTP client in a `ReadOnlyHttp` newtype exposing only `.get()`.
- **Writes** go through `WriteApi` (`src/cli/api.rs`) and per-domain `impl WriteApi` blocks in `src/cli/api_*.rs`. Raw write verbs are confined to `api.rs` + `api_*.rs`; the `tests/readonly.rs` grep gate fails the build if any other CLI module issues a raw write call.
- Endpoints without a dedicated `Route` variant are reached via `Route::Custom("relative/path")`.

The CLI lives behind the `cli` Cargo feature; the library compiles without it.

## Build, run, install

```bash
# build the binary (feature-gated)
cargo build --features cli --bin discord
cargo build --release --features cli --bin discord     # optimized

# run via cargo (dev)
cargo run --features cli --bin discord -- <subcommand> [args]

# run the compiled binary
./target/release/discord <subcommand> [args]

# install onto PATH (~/.cargo/bin)
cargo install --path . --features cli --bin discord --force
```

`Cargo.toml` declares `[[bin]] name = "discord"` at `src/bin/discord.rs`, gated by `required-features = ["cli"]`. The binary runs all work on a 32 MiB worker thread because clap's debug-mode command-tree validation overflows Windows' default 1 MiB main-thread stack (the command graph is large).

## Token resolution (priority order)

`DISCORD_TOKEN` is resolved in this order — first hit wins:

1. `--token <T>` global flag
2. `$DISCORD_TOKEN` environment variable
3. `./.env` file in the current working directory — **CWD only**, parent dirs are intentionally not walked (security)

Missing token → `No Discord token found. Run \`discord auth --save\`, set $DISCORD_TOKEN, or pass --token <T>.`

Tokens are wrapped in `Zeroizing<String>` (scrubbed on drop). Masked form is `<first4>…<last4>`. A bare token is treated as a **user token**; a `Bot `/`Bearer ` prefix marks an **application token** (changes header/fingerprint behavior — see Gotchas).

## Global flags

| Flag | Purpose |
|---|---|
| `--token <T>` | Override token resolution |
| `--db <PATH>` | SQLite path (default `./discord.db`) |
| `--json` | Output as JSON (machine-readable) |
| `--no-color` | Disable ANSI (also honors `$NO_COLOR`) |

Logging uses `tracing`; set `RUST_LOG=info` (or `debug`) for gateway/HTTP detail (default `warn`). Logs go to **stderr**, data to **stdout** — pipe-safe.

## Command map

- **`dc <...>`** — talks to Discord's REST/Gateway API (needs token + network). Split into **read** ops and **write/client** ops (flat verbs + nested groups).
- **Everything else** (top-level) — operates on the local `discord.db` archive, offline.

ID arguments are raw snowflakes for write commands. Read commands that take `<guild>` also accept a case-insensitive name substring; many also accept a stored channel name from the archive.

### `dc` — read (network)

| Command | What it does |
|---|---|
| `dc guilds` | List joined guilds |
| `dc channels <guild>` | List text channels |
| `dc sync <channel> [-n N=5000]` | Incremental sync — only new messages since last |
| `dc sync-all [-n N=200]` | Discover every guild+text channel and sync each |
| `dc history <channel> [-n N=1000]` | Backfill older messages |
| `dc tail <channel> [-n N=20] [--once]` | Live-stream via Gateway WebSocket (`--once` = snapshot then exit) |
| `dc search <guild> <keyword> [-c CHAN] [-n N=25]` | Remote search via Discord's API |
| `dc members <guild> [-n N=50]` | List members |
| `dc info <guild>` | Guild details |
| `dc pins <channel>` | Pinned messages |
| `dc threads <guild>` | Active threads |
| `dc relationships` | Friends / blocked / pending |
| `dc roles <guild>` | List roles |
| `dc emojis <guild>` | List custom emojis |
| `dc stickers <guild>` | List custom stickers |
| `dc profile [USER_ID]` | Profile (self if omitted) |
| `dc audit <guild> [-n N=50]` | Audit log (`-n` is `u8`, max 255) |
| `dc events <guild>` | Scheduled events |
| `dc download <channel> [-o DIR=downloads] [--hours N]` | Save attachments from stored messages |
| `dc snapshot <guild> [-o FILE]` | Dump full guild as JSON (stdout if no `-o`) |

### `dc` — write / client (flat verbs)

These mutate Discord state. For **user** tokens, `dc send`/`dc dm` auto-include `mobile_network_type: "unknown"`; it is omitted for app tokens (Discord 403s otherwise).

| Command | What it does |
|---|---|
| `dc send <channel> <content> [--reply MSG]` | Send a message; `--reply` makes it a reply |
| `dc edit <channel> <message> <content>` | Edit a message you authored |
| `dc delete <channel> <message> [-y]` | Delete a message |
| `dc react <channel> <message> <emoji>` | Add a reaction (unicode `👍` or custom `name:id`) |
| `dc unreact <channel> <message> <emoji>` | Remove your own reaction |
| `dc pin <channel> <message>` | Pin a message |
| `dc unpin <channel> <message>` | Unpin a message |
| `dc typing <channel>` | Fire the typing indicator once (~10s server-side) |
| `dc dm <user_id> <content>` | Open/reuse a DM channel and send a message |

### `dc` — write / client (nested groups)

Run `discord dc <group> --help` for each. All take raw snowflake IDs (validated locally before any request). Destructive actions prompt for confirmation unless `-y/--yes` is given; moderation actions accept `--reason` (recorded in the audit log).

| Group | Subcommands |
|---|---|
| `dc thread` | `create <chan> <name> [--auto-archive 60\|1440\|4320\|10080]`, `create-from <chan> <msg> <name>`, `join <thread>`, `leave <thread>`, `add-member <thread> <user>`, `remove-member <thread> <user>`, `archive <thread> [--locked]`, `unarchive <thread>` |
| `dc channel` | `create <guild> <name> [--type N] [--topic] [--parent] [--nsfw] [--slowmode]`, `edit <chan> [--name/--topic/--nsfw/--slowmode/--bitrate/--position]`, `delete <chan> [-y] [--reason]`, `slowmode <chan> <seconds>` |
| `dc perms` | `set <chan> <overwrite_id> [--type role\|member] [--allow BITS] [--deny BITS]`, `clear <chan> <overwrite_id>`, `list <chan>` |
| `dc member` | `set-nick <guild> <user> <nick>`, `add-role <guild> <user> <role>`, `remove-role <guild> <user> <role>`, `timeout <guild> <user> <until_iso8601> [--reason]`, `untimeout <guild> <user> [--reason]`, `kick <guild> <user> [-y] [--reason]` |
| `dc ban` | `add <guild> <user> [--delete-seconds N] [--reason] [-y]`, `remove <guild> <user> [--reason] [-y]`, `list <guild>`, `get <guild> <user>` |
| `dc role` | `create <guild> [--name/--color/--hoist/--mentionable/--permissions]`, `edit <guild> <role> [...]`, `delete <guild> <role> [-y]`, `list <guild>` |
| `dc invite` | `create <chan> [--max-age/--max-uses/--temporary/--unique]`, `delete <code>`, `list-channel <chan>`, `list-guild <guild>`, `get <code>` |
| `dc friend` | `add <user>`, `request <username>`, `accept <user>`, `remove <user>`, `block <user>`, `unblock <user>` |
| `dc emoji` | `create <guild> <name> <image_path>`, `rename <guild> <emoji_id> <name>`, `delete <guild> <emoji_id> [-y]` |
| `dc sticker` | `create <guild> <name> <tags> <file_path> [--description]`, `edit <guild> <sticker_id> [...]`, `delete <guild> <sticker_id> [-y]` |
| `dc message` | `bulk-delete <chan> <id,id,…> [-y]` (2–100), `crosspost <chan> <msg>`, `clear-reactions <chan> <msg> [-y]`, `clear-emoji <chan> <msg> <emoji> [-y]`, `remove-user-reaction <chan> <msg> <user> <emoji>`, `reactors <chan> <msg> <emoji> [-n N]` |
| `dc guild` | `edit <guild> [--name/--description/--afk-channel/--afk-timeout]`, `set-nick <guild> <nick>`, `leave <guild> [-y]` |
| `dc event` | `create <guild> <name> <start_iso> [--type voice\|stage\|external] [--channel/--location/--end-iso/--description]`, `edit <guild> <event_id> [...]`, `delete <guild> <event_id> [-y]` |
| `dc webhook` | `create <chan> <name>`, `edit <webhook_id> [--name/--channel]`, `delete <webhook_id> [-y]`, `execute <webhook_id> <token> <content>`, `list-channel <chan>` |
| `dc compose` | `embed <chan> [--content/--title/--description/--color/--url]`, `tts <chan> <content>`, `poll <chan> <question> <ans,ans,…> [--duration H] [--multiselect]`, `mark-read <chan> <msg>` |

### Local archive commands (offline, operate on `discord.db`)

| Command | What it does |
|---|---|
| `auth [--save]` | Auto-discover token from local Discord install (DPAPI on Windows). `--save` writes `./.env` |
| `status` | Verify the configured token via `GET /users/@me` |
| `whoami` | Show the authenticated user profile |
| `search <keyword> [-c CHAN] [-n N=50]` | Case-insensitive substring search over stored messages |
| `recent [-c CHAN] [--hours N] [-n N=50]` | Newest stored messages |
| `today [-c CHAN]` | Today's messages grouped by channel |
| `top [-c CHAN] [--hours N] [-n N=20]` | Most active senders |
| `stats` | Per-channel message counts |
| `timeline [-c CHAN] [--hours N] [--by day\|hour]` | Activity over time |
| `export <channel> [-f text\|json\|yaml] [-o FILE] [--hours N]` | Export messages |
| `purge <channel> [-y]` | Delete stored messages for a channel (local only) |

## Typical workflows

**First-time setup (Windows, Discord installed):**
```bash
discord auth --save     # discover + save token to ./.env
discord status          # confirm it works
discord whoami
```

**Sync everything, then query offline:**
```bash
discord dc sync-all -n 500
discord stats
discord search "deadline"
discord export 1234567890 -f json -o channel.json
```

**Act as a client:**
```bash
discord dc send 123 "hello" --reply 456
discord dc react 123 456 👍
discord dc thread create 123 "planning" --auto-archive 1440
discord dc member kick 111 222 --reason "spam" -y
discord dc ban add 111 222 --delete-seconds 86400 --reason "raid" -y
discord dc compose poll 123 "Lunch?" "Pizza,Sushi,Tacos" --multiselect
discord dc webhook create 123 "ci-bot"
```

**Live-monitor a channel:** `discord dc tail <channel_id> -n 50`

## Gotchas

- **Self-botting violates Discord ToS.** Account bans are a real risk; throttle write activity.
- **Confirmation prompts:** destructive commands (`delete`, `kick`, `ban`, channel/role/emoji/sticker/event/webhook `delete`, `guild leave`, `message bulk-delete`/`clear-reactions`/`clear-emoji`) prompt for `yes` on stdin unless `-y/--yes` is passed.
- **Local validation:** snowflake IDs, `bulk-delete` count (2–100), `reactors --limit` (≤100), and `thread --auto-archive` (60/1440/4320/10080) are checked locally and fail fast before any request.
- **Audit-log `--reason`** is supported on `ban add/remove`, `member kick/timeout/untimeout`, and `channel delete`.
- **`mobile_network_type`** fingerprint is auto-added on message sends for user tokens and omitted for `Bot `/`Bearer ` tokens (Discord 403 code 40333 otherwise).
- **Hardcoded gateway client build number** in the IDENTIFY handshake — must be refreshed periodically when Discord pushes new client versions (symptom: gateway closes immediately / rejects identify). Gateway host is hardcoded to `wss://gateway-us-east1-c.discord.gg`.
- **`.env` is read from CWD only**, not walked up (by design). **`--db` defaults to `./discord.db`** relative to CWD — switching directories silently switches archives.
- **Rate limiting:** the HTTP client parses `X-RateLimit-*` and retries up to 5× with backoff; per-route buckets normalize all-digit path segments to `{id}`. Sustained 429s mean you're going too fast (Cloudflare 1015 IP-ban risk).

## Source map (extending the CLI)

| Concern | File |
|---|---|
| Argument definitions (clap) | `src/cli/args.rs` |
| Subcommand dispatch | `src/cli/commands/mod.rs` |
| Read command impls | `src/cli/commands/dc_*.rs`, `*.rs` |
| Write command groups | `src/cli/commands/dcw_<domain>.rs` |
| Read REST wrapper (`Api`/`ReadOnlyHttp`) | `src/cli/api.rs` |
| Write REST wrapper (`WriteApi`/`WriteHttp`) | `src/cli/api.rs` + `src/cli/api_<domain>.rs` |
| Generic endpoint escape hatch | `Route::Custom` in `src/route.rs` |
| Token resolution + `.env` upsert | `src/cli/config.rs` |
| Token auto-discovery (per-platform) | `src/cli/auth/{windows,macos,linux}.rs` |
| SQLite schema + queries | `src/cli/db.rs` |
| Output (tables, JSON, colors, `confirm()`) | `src/cli/output.rs` |
| Binary entrypoint (worker thread) | `src/bin/discord.rs` |
| Write-verb containment test | `tests/readonly.rs` |

**To add a write command:** add an `impl WriteApi` method in `src/cli/api_<domain>.rs` (reach the endpoint via `Route::Custom`, validate snowflakes with `crate::cli::api::parse_snowflake`), add a `Subcommand` variant + `dispatch` in `src/cli/commands/dcw_<domain>.rs`, then register the module in `src/cli.rs` (`api_*`) / `src/cli/commands/mod.rs` (`dcw_*`) and wire the `DcCmd` variant + arm in `args.rs` / `mod.rs`.
