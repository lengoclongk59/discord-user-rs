# Changelog

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
