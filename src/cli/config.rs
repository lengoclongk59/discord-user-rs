//! Token resolution chain, archive location, and `.env` upsert.

use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use tempfile::NamedTempFile;
use zeroize::{Zeroize, Zeroizing};

/// Resolve the user token using this priority:
/// 1. `cli_flag` (e.g. `--token <T>`)
/// 2. `DISCORD_TOKEN` env var
/// 3. `./.env` -> `DISCORD_TOKEN=...`
///
/// The returned token is wrapped in `Zeroizing<String>` so its heap buffer
/// is scrubbed on drop. `Zeroizing` derefs to `String` so callers can pass
/// `&token` to anything expecting `&str`.
pub fn resolve_token(cli_flag: Option<String>) -> Result<Zeroizing<String>> {
    if let Some(t) = cli_flag.filter(|s| !s.is_empty()) {
        return Ok(Zeroizing::new(t));
    }
    if let Ok(t) = std::env::var("DISCORD_TOKEN") {
        if !t.is_empty() {
            return Ok(Zeroizing::new(t));
        }
    }
    // CWD-only — `dotenvy::dotenv()` walks parent directories, which lets
    // an attacker who can plant `../.env` (or further up) substitute their
    // own `DISCORD_TOKEN`.
    let _ = dotenvy::from_path(".env");
    if let Ok(t) = std::env::var("DISCORD_TOKEN") {
        if !t.is_empty() {
            return Ok(Zeroizing::new(t));
        }
    }
    Err(anyhow!(
        "No Discord token found. Run `discord auth --save`, set $DISCORD_TOKEN, or pass --token <T>."
    ))
}

/// Upsert `DISCORD_TOKEN=<token>` into `./.env`. Creates the file if
/// missing. Preserves all other lines.
///
/// Write semantics: write to a tempfile in the same directory, fsync, then
/// rename. Pre-rename crashes leave the original .env intact. After write,
/// best-effort 0600 on POSIX so the token isn't world-readable.
pub fn write_token_to_env(token: &str) -> Result<PathBuf> {
    let path = Path::new(".env").to_path_buf();
    let existing = fs::read_to_string(&path).unwrap_or_default();

    let mut found = false;
    let mut new_lines: Vec<String> = Vec::new();
    for line in existing.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("DISCORD_TOKEN") {
            // Match either `DISCORD_TOKEN=...` or `DISCORD_TOKEN =...`
            let rest = rest.trim_start();
            if rest.starts_with('=') {
                new_lines.push(format!("DISCORD_TOKEN={}", token));
                found = true;
                continue;
            }
        }
        new_lines.push(line.to_string());
    }
    if !found {
        new_lines.push(format!("DISCORD_TOKEN={}", token));
    }
    let mut out = new_lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }

    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut tmp = if let Some(p) = parent {
        NamedTempFile::new_in(p)
    } else {
        NamedTempFile::new_in(".")
    }
    .context("creating .env tempfile")?;
    tmp.write_all(out.as_bytes())?;
    tmp.as_file().sync_all().ok();

    // Tighten perms before the rename so the file is never visible to
    // other users (relevant on POSIX; on Windows ACLs inherit from parent).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600));
    }

    tmp.persist(&path)
        .map_err(|e| anyhow!("persisting .env: {}", e))?;

    // Best-effort scrub of the in-memory copy.
    out.zeroize();
    Ok(path)
}

/// Resolve the SQLite archive path when `--db` was not given:
/// 1. `DISCORD_DB` env var
/// 2. `<home>/.discord/discord.db`
/// 3. `./discord.db` (only when the home directory cannot be determined)
///
/// Defaulting under `$HOME` keeps one archive for the user regardless of the
/// working directory; the old CWD-relative default silently switched archives
/// on every `cd`.
pub fn default_db_path() -> PathBuf {
    db_path_from(std::env::var_os("DISCORD_DB"), home_dir())
}

/// Pure core of [`default_db_path`], split out so it is testable without
/// mutating process-wide environment state.
fn db_path_from(env_db: Option<OsString>, home: Option<PathBuf>) -> PathBuf {
    if let Some(p) = env_db.filter(|v| !v.is_empty()) {
        return expand_tilde_with(PathBuf::from(p), home.as_deref());
    }
    match home {
        Some(h) => h.join(".discord").join("discord.db"),
        None => PathBuf::from("discord.db"),
    }
}

/// Expand a leading `~` / `~/...` to the home directory.
///
/// A shell expands `~` before the value ever reaches us, but a quoted
/// `--db '~/a.db'`, a `.env` line, or a systemd unit does not — and since
/// [`crate::cli::db::Db::open`] creates missing parents, an unexpanded `~`
/// would silently produce a directory literally named `~` in the CWD.
///
/// `~user` is deliberately left untouched: resolving another account's home
/// needs a passwd lookup, and guessing it would point at the wrong archive.
pub fn expand_tilde(path: PathBuf) -> PathBuf {
    expand_tilde_with(path, home_dir().as_deref())
}

fn expand_tilde_with(path: PathBuf, home: Option<&Path>) -> PathBuf {
    let Some(home) = home else { return path };
    let Some(s) = path.to_str() else { return path };
    if s == "~" {
        return home.to_path_buf();
    }
    match s.strip_prefix("~/") {
        // `~user/...` has no leading `~/`, so it falls through untouched.
        Some(rest) => home.join(rest),
        None => path,
    }
}

/// The user's home directory, without pulling in a dependency for it.
fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let raw = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    #[cfg(not(windows))]
    let raw = std::env::var_os("HOME");
    raw.filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Lightweight shape check shared across platform-specific token finders.
/// Discord tokens look like `<base64-ish>.<base64-ish>.<base64-ish>` with
/// total length 40–200 chars.
pub fn looks_like_discord_token(s: &str) -> bool {
    let len = s.len();
    if !(40..=200).contains(&len) {
        return false;
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

/// Mask the token to `<first4>…<last4>` (or `<masked>` if shorter than 8 chars).
pub fn mask_token(t: &str) -> String {
    if t.len() < 8 {
        return "<masked>".to_string();
    }
    let head: String = t.chars().take(4).collect();
    let tail_skip = t.chars().count().saturating_sub(4);
    let tail: String = t.chars().skip(tail_skip).collect();
    format!("{}…{}", head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_short_token() {
        assert_eq!(mask_token(""), "<masked>");
        assert_eq!(mask_token("abc"), "<masked>");
        assert_eq!(mask_token("1234567"), "<masked>");
    }

    #[test]
    fn mask_normal_token() {
        let t = "abcdefghijklmnopqrstuvwxyz";
        assert_eq!(mask_token(t), "abcd…wxyz");
    }

    #[test]
    fn mask_typical_discord_token_only_reveals_first4_last4() {
        let t = "FAKE_TOKEN_aaaa.bbbbbb.cccccccccccccccccccccccccccc";
        let masked = mask_token(t);
        assert!(masked.starts_with("FAKE"));
        assert!(masked.ends_with("cccc"));
        // Total chars revealed should be exactly 8 (4 head + 4 tail).
        let revealed = masked.replace('…', "");
        assert_eq!(revealed.chars().count(), 8);
    }

    #[test]
    fn db_path_prefers_env_override() {
        let got = db_path_from(
            Some(OsString::from("/tmp/other.db")),
            Some(PathBuf::from("/home/u")),
        );
        assert_eq!(got, PathBuf::from("/tmp/other.db"));
    }

    #[test]
    fn db_path_defaults_under_home() {
        let got = db_path_from(None, Some(PathBuf::from("/home/u")));
        assert_eq!(got, PathBuf::from("/home/u/.discord/discord.db"));
    }

    #[test]
    fn db_path_ignores_empty_env_override() {
        let got = db_path_from(Some(OsString::new()), Some(PathBuf::from("/home/u")));
        assert_eq!(got, PathBuf::from("/home/u/.discord/discord.db"));
    }

    #[test]
    fn db_path_falls_back_to_cwd_without_home() {
        let got = db_path_from(None, None);
        assert_eq!(got, PathBuf::from("discord.db"));
    }

    #[test]
    fn db_path_expands_tilde_in_env_override() {
        let got = db_path_from(
            Some(OsString::from("~/archives/a.db")),
            Some(PathBuf::from("/home/u")),
        );
        assert_eq!(got, PathBuf::from("/home/u/archives/a.db"));
    }

    #[test]
    fn expand_tilde_leaves_other_paths_alone() {
        let home = Path::new("/home/u");
        // Bare `~` is the home directory itself.
        assert_eq!(
            expand_tilde_with(PathBuf::from("~"), Some(home)),
            PathBuf::from("/home/u")
        );
        // `~user` needs a passwd lookup, so it stays literal.
        assert_eq!(
            expand_tilde_with(PathBuf::from("~root/a.db"), Some(home)),
            PathBuf::from("~root/a.db")
        );
        // A `~` that is not leading is an ordinary character.
        assert_eq!(
            expand_tilde_with(PathBuf::from("/tmp/~/a.db"), Some(home)),
            PathBuf::from("/tmp/~/a.db")
        );
        // Absolute and relative paths pass through.
        assert_eq!(
            expand_tilde_with(PathBuf::from("/tmp/a.db"), Some(home)),
            PathBuf::from("/tmp/a.db")
        );
        // Without a home directory there is nothing to expand to.
        assert_eq!(
            expand_tilde_with(PathBuf::from("~/a.db"), None),
            PathBuf::from("~/a.db")
        );
    }

    #[test]
    fn resolve_token_from_flag() {
        let got = resolve_token(Some("from-flag".to_string())).unwrap();
        assert_eq!(*got, "from-flag");
    }
}
