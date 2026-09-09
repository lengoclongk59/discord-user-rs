# Changelog

## Unreleased

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
