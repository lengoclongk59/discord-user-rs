---
name: discord-cli
description: Use when working in the discord-user-rs repo (or any clone of it) and you need to build, run, or interpret the bundled `discord` CLI client — a full-featured Discord client (send/edit/react/threads/roles/bans/invites/webhooks/…) plus a local SQLite archive for offline querying. Covers token setup, syncing, querying, and every read & write command.
---

# discord-user-rs CLI (`discord` binary)

## Overview

`discord` is the full-featured Discord **client CLI** bundled with the `discord-user-rs` library. It logs into Discord with a **user token** (self-bot — violates Discord ToS, use at your own risk) or a **bot/bearer token**, and does two things:

1. **Acts as a client** — send/edit/delete/react to messages (with file/sticker/voice attachments), DMs, group DMs, threads, channels, roles, bans, invites, emojis, stickers, soundboard, webhooks, scheduled events, stage instances, AutoMod rules, application commands, relationships, presence/status, your own profile, guild templates/welcome-screen/onboarding/widget/integrations, polls, embeds, etc. (`dc` write commands).
2. **Archives + queries offline** — syncs guilds/channels into a local SQLite DB (`~/.discord/discord.db` by default) and runs search/stats/export/timeline over it with no network.

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
| `--db <PATH>` | SQLite archive path (default `$DISCORD_DB`, else `~/.discord/discord.db`) |
| `--json` | Output as JSON (machine-readable) |
| `--no-color` | Disable ANSI (also honors `$NO_COLOR`) |

Logging uses `tracing`; set `RUST_LOG=info` (or `debug`) for gateway/HTTP detail (default `warn`). Logs go to **stderr**, data to **stdout** — pipe-safe.

## Command map

- **`dc <...>`** — talks to Discord's REST/Gateway API (needs token + network). Split into **read** ops and **write/client** ops (flat verbs + nested groups).
- **Everything else** (top-level) — operates on the local SQLite archive, offline.

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
| `dc search <guild> [keyword] [-c CHAN] [--author ID] [--mentions ID] [--has link\|embed\|file\|image\|video\|sound] [--pinned] [--before ID] [--after ID] [-n N=25]` | Remote search via Discord's API; keyword optional when another filter is given |
| `dc members <guild> [-n N=50]` | List members |
| `dc info <guild>` | Guild details |
| `dc pins <channel>` | Pinned messages |
| `dc threads <guild>` | Active threads |
| `dc relationships` | Friends / blocked / pending |
| `dc roles <guild>` | List roles |
| `dc emojis <guild>` | List custom emojis |
| `dc stickers <guild>` | List custom stickers |
| `dc profile [USER_ID]` | Profile (self if omitted) |
| `dc audit <guild> [-n N=50] [--user ID] [--action-type N] [--before ID] [--after ID]` | Audit log (`-n` is `u8`, max 255); filter by moderator, action type, and ID cursors |
| `dc events <guild>` | Scheduled events |
| `dc download <channel> [-o DIR=downloads] [--hours N]` | Save attachments from stored messages |
| `dc snapshot <guild> [-o FILE]` | Dump full guild as JSON (stdout if no `-o`) |
| `dc dms` | List your open DM and group-DM channels (`GET /users/@me/channels`) |

### `dc` — write / client (flat verbs)

These mutate Discord state. For **user** tokens, `dc send`/`dc dm` auto-include `mobile_network_type: "unknown"`; it is omitted for app tokens (Discord 403s otherwise).

| Command | What it does |
|---|---|
| `dc send <channel> <content> [--reply MSG] [--file PATH …] [--sticker ID …] [--tts] [--silent] [--suppress-embeds] [--no-mentions] [--nonce N]` | Send a message; `--reply` makes it a reply; `--file` attaches files (repeatable, multipart); `--sticker` attaches stickers (repeatable, max 3); `--tts` text-to-speech; `--silent` no notification (flag 1<<12); `--suppress-embeds` hide unfurls (flag 1<<2); `--no-mentions` ping no one; `--nonce` idempotency key |
| `dc edit <channel> <message> <content>` | Edit a message you authored |
| `dc delete <channel> <message> [-y]` | Delete a message |
| `dc react <channel> <message> <emoji> [--burst]` | Add a reaction (unicode `👍` or custom `name:id`); `--burst` = super/burst reaction |
| `dc unreact <channel> <message> <emoji>` | Remove your own reaction |
| `dc pin <channel> <message>` | Pin a message |
| `dc unpin <channel> <message>` | Unpin a message |
| `dc typing <channel>` | Fire the typing indicator once (~10s server-side) |
| `dc dm <user_id> <content> [--file PATH …]` | Open/reuse a DM channel and send a message (with optional attachments) |
| `dc voice-message <channel> <file> [--duration SECS] [--waveform B64]` | Send an audio file as a voice message (`flags=8192`); duration is sent as-is (default 1.0), waveform auto-generated as a placeholder if omitted |

### `dc` — write / client (nested groups)

Run `discord dc <group> --help` for each. All take raw snowflake IDs (validated locally before any request). Destructive actions prompt for confirmation unless `-y/--yes` is given; moderation actions accept `--reason` (recorded in the audit log).

| Group | Subcommands |
|---|---|
| `dc thread` | `create <chan> <name> [--auto-archive 60\|1440\|4320\|10080] [--type public\|private\|announcement]`, `forum-post <chan> <name> <message> [--auto-archive …] [--tag TAG_ID …]` (forum/media post with applied tags), `create-from <chan> <msg> <name>`, `join <thread>`, `leave <thread>`, `add-member <thread> <user>`, `remove-member <thread> <user>`, `archive <thread> [--locked]`, `unarchive <thread>` |
| `dc channel` | `create <guild> <name> [--type N] [--topic] [--parent] [--nsfw] [--slowmode]`, `edit <chan> [--name/--topic/--nsfw/--slowmode/--bitrate/--position]`, `delete <chan> [-y] [--reason]`, `slowmode <chan> <seconds>` |
| `dc perms` | `set <chan> <overwrite_id> [--type role\|member] [--allow BITS] [--deny BITS]`, `clear <chan> <overwrite_id>`, `list <chan>` |
| `dc member` | `set-nick <guild> <user> <nick>`, `add-role <guild> <user> <role>`, `remove-role <guild> <user> <role>`, `timeout <guild> <user> <until_iso8601> [--reason]`, `untimeout <guild> <user> [--reason]`, `kick <guild> <user> [-y] [--reason]`, `search <guild> <query> [-n N=25]`, `prune-count <guild> [--days 7] [--include-roles ids]`, `prune <guild> [--days 7] [--include-roles ids] [--count] [--reason] [-y]` |
| `dc ban` | `add <guild> <user> [--delete-seconds N] [--reason] [-y]`, `remove <guild> <user> [--reason] [-y]`, `list <guild>`, `get <guild> <user>`, `bulk <guild> <id,id,…> [--delete-seconds N] [--reason] [-y]` (≤200 users) |
| `dc role` | `create <guild> [--name/--color/--hoist/--mentionable/--permissions]`, `edit <guild> <role> [...]`, `delete <guild> <role> [-y]`, `list <guild>` |
| `dc invite` | `create <chan> [--max-age/--max-uses/--temporary/--unique]`, `delete <code>`, `list-channel <chan>`, `list-guild <guild>`, `get <code>` |
| `dc friend` | `add <user>`, `request <username>`, `accept <user>`, `remove <user>`, `block <user>`, `unblock <user>`, `nickname <user> <nick>` (PATCH relationship; `""` clears) |
| `dc emoji` | `create <guild> <name> <image_path>`, `rename <guild> <emoji_id> <name>`, `delete <guild> <emoji_id> [-y]` |
| `dc sticker` | `create <guild> <name> <tags> <file_path> [--description]`, `edit <guild> <sticker_id> [...]`, `delete <guild> <sticker_id> [-y]` |
| `dc message` | `bulk-delete <chan> <id,id,…> [-y]` (2–100), `crosspost <chan> <msg>`, `forward <src_chan> <msg> <dest_chan> [--content TEXT]` (message_reference type 1), `clear-reactions <chan> <msg> [-y]`, `clear-emoji <chan> <msg> <emoji> [-y]`, `remove-user-reaction <chan> <msg> <user> <emoji>`, `reactors <chan> <msg> <emoji> [-n N]` |
| `dc guild` | `edit <guild> [--name/--description/--afk-channel/--afk-timeout]`, `set-nick <guild> <nick>`, `leave <guild> [-y]`, `vanity <guild>`, `widget-get <guild>`, `widget-edit <guild> [--enabled true\|false] [--channel ID]`, `welcome-get <guild>`, `welcome-edit <guild> [--enabled true\|false] [--description TEXT]`, `onboarding-get <guild>`, `integrations <guild>`, `integration-delete <guild> <integration_id> [-y]`, `templates <guild>`, `template-create <guild> <name> [--description TEXT]`, `template-sync <guild> <code>`, `template-delete <guild> <code> [-y]` |
| `dc event` | `create <guild> <name> <start_iso> [--type voice\|stage\|external] [--channel/--location/--end-iso/--description]`, `edit <guild> <event_id> [...]`, `delete <guild> <event_id> [-y]` |
| `dc webhook` | `create <chan> <name>`, `edit <webhook_id> [--name/--channel]`, `delete <webhook_id> [-y]`, `execute <webhook_id> <token> <content>`, `list-channel <chan>` |
| `dc compose` | `embed <chan> [--content/--title/--description/--color/--url/--footer/--footer-icon/--author/--author-url/--author-icon/--image/--thumbnail/--timestamp] [--field "name\|value\|inline" …] [--file PATH …]`, `tts <chan> <content>`, `poll <chan> <question> <ans,ans,…> [--duration H] [--multiselect]`, `mark-read <chan> <msg>` |
| `dc dm-group` | `create <user,user,…>` (2+ recipients), `add <chan> <user>`, `remove <chan> <user>`, `set-name <chan> <name>` |
| `dc presence` | `set <online\|idle\|dnd\|invisible> [--activity TEXT] [--type playing\|streaming\|listening\|watching\|competing\|custom] [--url URL] [--emoji 🎮\|name:id]` — connects the Gateway and **holds the connection open until Ctrl+C** (presence reverts on disconnect); `--emoji` applies to `--type custom` (live only, not persisted) |
| `dc account` | `edit [--username/--global-name/--bio/--pronouns/--accent-color/--avatar PATH/--banner PATH]` — edits your own profile (PATCH `/users/@me` + `/users/@me/profile`; at least one field required); `connections` (list linked accounts), `connection-edit <type> <id> [--visible BOOL] [--show-activity BOOL]`, `connection-remove <type> <id>`, `note-get <user>`, `note-set <user> <note>` (`""` clears) |
| `dc notify` | `guild <guild> [--muted BOOL] [--level all\|mentions\|nothing] [--suppress-everyone BOOL] [--suppress-roles BOOL]`, `channel <guild\|@me> <chan> [--muted BOOL] [--level …]`, `read-channel <chan> <msg>` (mark read up to message), `read-guild <guild>` (mark whole guild read) |
| `dc automod` | `list <guild>`, `get <guild> <rule>`, `create <guild> <name> --event-type N --trigger-type N [--keyword w,w] [--preset N,N] [--mention-limit N] [--enabled true\|false] [--exempt-roles ids] [--exempt-channels ids]`, `edit <guild> <rule> [...]`, `delete <guild> <rule> [-y]` (trigger: 1=KEYWORD 3=SPAM 4=KEYWORD_PRESET 5=MENTION_SPAM; event: 1=MESSAGE_SEND; default action BLOCK_MESSAGE) |
| `dc soundboard` | `list-default`, `list <guild>`, `get <guild> <sound>`, `create <guild> <name> <file> [--volume 0.0-1.0] [--emoji-id ID] [--emoji-name 😀]` (mp3/ogg ≤512 KB), `edit <guild> <sound> [...]`, `delete <guild> <sound> [-y]`, `send <channel> <sound> [--source-guild ID]` (plays into a voice channel you're connected to) |
| `dc stage` | `start <channel> <topic> [--privacy 1\|2] [--send-start-notification]` (topic 1-120, privacy default 2=GUILD_ONLY), `get <channel>`, `edit <channel> [--topic/--privacy]`, `stop <channel> [-y]` |
| `dc command` | `list [--guild G]`, `get [--guild G] <command_id>`, `create <name> <description> [--guild G] [--type 1\|2\|3]` (registers an app command; works for bot/application tokens), `delete [--guild G] <command_id> [-y]`, `run <channel> <command_name>` (**not supported over REST for user tokens — prints an explanation; use `list`/`create` instead**) |

### Local archive commands (offline, operate on the SQLite archive)

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
discord dc send 123 "see attached" --file report.pdf --file pic.png
discord dc react 123 456 👍 --burst
discord dc voice-message 123 clip.ogg --duration 3.5
discord dc thread create 123 "planning" --auto-archive 1440
discord dc member kick 111 222 --reason "spam" -y
discord dc ban add 111 222 --delete-seconds 86400 --reason "raid" -y
discord dc compose poll 123 "Lunch?" "Pizza,Sushi,Tacos" --multiselect
discord dc webhook create 123 "ci-bot"
discord dc presence set dnd --activity "shipping features" --type playing
discord dc dm-group create 111,222,333
discord dc automod create 111 "no-invites" --event-type 1 --trigger-type 1 --keyword "discord.gg,join my"
discord dc stage start 999 "Town Hall" --send-start-notification
discord dc ban bulk 111 222,333,444 --delete-seconds 3600 --reason "raid" -y
discord dc member prune-count 111 --days 30
discord dc compose embed 123 --title "Release" --color 65280 --field "Version|0.6.0|inline" --footer "ci"
discord dc notify guild 111 --muted true --level mentions
discord dc account note-set 222 "met at the meetup"
discord dc presence set online --type custom --activity "focusing" --emoji 🎯
```

**Account / social housekeeping:**
```bash
discord dc dms                         # list open DMs / group DMs
discord dc account connections         # list linked accounts (Steam, Spotify, …)
discord dc friend nickname 222 "Bob"   # set a friend nickname
discord dc notify read-guild 111       # mark a whole guild read
```

**Live-monitor a channel:** `discord dc tail <channel_id> -n 50`

## Gotchas

- **Self-botting violates Discord ToS.** Account bans are a real risk; throttle write activity.
- **Confirmation prompts:** destructive commands (`delete`, `kick`, `ban`, channel/role/emoji/sticker/event/webhook `delete`, `guild leave`, `message bulk-delete`/`clear-reactions`/`clear-emoji`, `automod delete`, `soundboard delete`, `stage stop`, `command delete`, `guild integration-delete`/`template-delete`) prompt for `yes` on stdin unless `-y/--yes` is passed.
- **`dc presence set` holds the Gateway connection open** until Ctrl+C — Discord only honors a gateway presence update while the originating socket stays connected, so a one-shot send would revert immediately. Leave it running to keep the status live.
- **`dc command run` is intentionally unsupported:** invoking a slash command as a user needs the undocumented `/interactions` flow (live `session_id` + nonce) and isn't reachable over REST. The subcommand prints an explanation and exits; use `dc command list`/`create` (which work for bot/application tokens). Voice messages and soundboard `send` upload/trigger over REST but **do not stream real-time audio** — the CLI has no UDP/Opus voice media transport.
- **Local validation:** snowflake IDs, `bulk-delete` count (2–100), `reactors --limit` (≤100), and `thread --auto-archive` (60/1440/4320/10080) are checked locally and fail fast before any request.
- **Audit-log `--reason`** is supported on `ban add/remove`, `member kick/timeout/untimeout`, and `channel delete`.
- **`mobile_network_type`** fingerprint is auto-added on message sends for user tokens and omitted for `Bot `/`Bearer ` tokens (Discord 403 code 40333 otherwise).
- **Hardcoded gateway client build number** in the IDENTIFY handshake — must be refreshed periodically when Discord pushes new client versions (symptom: gateway closes immediately / rejects identify). Gateway host is hardcoded to `wss://gateway-us-east1-c.discord.gg`.
- **`.env` is read from CWD only**, not walked up (by design).
- **The archive is per-user, not CWD-relative:** `--db` → `$DISCORD_DB` → `~/.discord/discord.db` (`%USERPROFILE%\.discord\discord.db` on Windows), with the parent directory created on first open. A leading `~` is expanded in either override (`~user` is left literal, since resolving it needs a passwd lookup). A directory or archive the CLI creates itself is chmod'd `0700`/`0600` — it holds message bodies — while a pre-existing one is left untouched. Only when no home directory can be resolved does it fall back to `./discord.db`. Archives written by ≤ 0.6.1, which defaulted to `./discord.db`, are not picked up automatically — the CLI prints a warning whenever a stranded `./discord.db` is present; move the file or point `$DISCORD_DB` at it.
- **`DISCORD_DB` works only as a real environment variable, not from `.env`.** The archive path is resolved in `Ctx::from_cli` before any command runs, whereas `.env` is loaded lazily by `resolve_token`, so a `DISCORD_DB=` line in `.env` is read too late and is silently ignored — unlike `DISCORD_TOKEN=`, which works there. Export it in the shell or pass `--db`.
- **Rate limiting:** the HTTP client parses `X-RateLimit-*` and retries up to 5× with backoff; per-route buckets normalize all-digit path segments to `{id}`. Sustained 429s mean you're going too fast (Cloudflare 1015 IP-ban risk).

## Source map (extending the CLI)

| Concern | File |
|---|---|
| Argument definitions (clap) | `src/cli/args.rs` |
| Subcommand dispatch | `src/cli/commands/mod.rs` |
| Read command impls | `src/cli/commands/dc_*.rs`, `*.rs` |
| Flat write command impls | `src/cli/commands/dc_send.rs`, `dc_react.rs`, `dc_dm.rs`, `dc_voice_message.rs`, … |
| Write command groups | `src/cli/commands/dcw_<domain>.rs` (e.g. `dcw_presence`, `dcw_dmgroup`, `dcw_account`, `dcw_automod`, `dcw_soundboard`, `dcw_stage`, `dcw_commands`, `dcw_notify`) |
| Read REST wrapper (`Api`/`ReadOnlyHttp`) | `src/cli/api.rs` |
| Write REST wrapper (`WriteApi`/`WriteHttp`) | `src/cli/api.rs` + `src/cli/api_<domain>.rs` (incl. `api_account`, `api_automod`, `api_soundboard`, `api_stage`, `api_commands`, `api_usersettings`) |
| Multipart upload helpers (files / stickers / voice / soundboard / images) | `post_raw_multipart`, `post_message_with_files`, `send_message_with_files`, `send_voice_message`, `build_file_part`, `guess_mime` in `src/cli/api.rs` |
| Generic endpoint escape hatch | `Route::Custom` in `src/route.rs` |
| Token resolution, default archive path, `.env` upsert | `src/cli/config.rs` |
| Token auto-discovery (per-platform) | `src/cli/auth/{windows,macos,linux}.rs` |
| SQLite schema + queries | `src/cli/db.rs` |
| Output (tables, JSON, colors, `confirm()`) | `src/cli/output.rs` |
| Binary entrypoint (worker thread) | `src/bin/discord.rs` |
| Write-verb containment test | `tests/readonly.rs` |

**To add a write command:** add an `impl WriteApi` method in `src/cli/api_<domain>.rs` (reach the endpoint via `Route::Custom`, validate snowflakes with `crate::cli::api::parse_snowflake`), add a `Subcommand` variant + `dispatch` in `src/cli/commands/dcw_<domain>.rs`, then register the module in `src/cli.rs` (`api_*`) / `src/cli/commands/mod.rs` (`dcw_*`) and wire the `DcCmd` variant + arm in `args.rs` / `mod.rs`.
