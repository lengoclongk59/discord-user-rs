//! Read the active Discord Local Storage token from a private LevelDB copy.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rusty_leveldb::{LdbIterator, Options, DB};
use zeroize::Zeroizing;

use super::{validate_token, DiscoveredToken};
use crate::cli::config::looks_like_discord_token;

pub async fn find_and_save_token() -> Result<DiscoveredToken> {
    let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let mut last_error = None;
    for dir in storage_dirs(&home, &config) {
        if !dir.is_dir() {
            continue;
        }
        let source = dir.display().to_string();
        let tokens = match tokio::task::spawn_blocking(move || read_tokens(&dir)).await? {
            Ok(tokens) => tokens,
            Err(_) => {
                last_error = Some(format!("Could not read {source}; close Discord and retry"));
                continue;
            }
        };
        for token in tokens {
            match validate_token(&token).await {
                Ok(me) => return Ok(DiscoveredToken {
                    token: token.to_string(),
                    source,
                    username: me.global_name.unwrap_or(me.username),
                }),
                Err(_) => last_error = Some("A stored token could not be validated; check your connection and Discord login".to_string()),
            }
        }
    }
    bail!(
        "{}. Log into the Discord desktop app, or set $DISCORD_TOKEN / pass --token <T>.",
        last_error
            .as_deref()
            .unwrap_or("No supported Discord desktop token found")
    )
}

fn storage_dirs(home: &Path, config: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for (name, app_id) in [
        ("discord", "com.discordapp.Discord"),
        ("discordptb", "com.discordapp.DiscordPTB"),
        ("discordcanary", "com.discordapp.DiscordCanary"),
    ] {
        for root in [
            config.to_path_buf(),
            home.join(".var/app").join(app_id).join("config"),
        ] {
            dirs.push(root.join(name).join("Local Storage/leveldb"));
        }
    }
    dirs.push(home.join("snap/discord/current/.config/discord/Local Storage/leveldb"));
    dirs
}

fn read_tokens(dir: &Path) -> Result<Vec<Zeroizing<String>>> {
    // LevelDB recovery writes files. Never open the live profile with this library.
    let copy = tempfile::tempdir()?;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let path = entry.path();
        let needed = name == "CURRENT"
            || name.to_string_lossy().starts_with("MANIFEST-")
            || matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("ldb" | "sst" | "log")
            );
        if needed && entry.file_type()?.is_file() {
            std::fs::copy(path, copy.path().join(name))?;
        }
    }
    let mut db = DB::open(
        copy.path(),
        Options {
            create_if_missing: false,
            ..Options::default()
        },
    )
    .context("Could not open Discord storage copy")?;
    let mut iter = db.new_iter()?;
    let mut tokens = Vec::new();
    while let Some((key, value)) = iter.next() {
        let value = Zeroizing::new(value);
        if let Some(token) = token_record(&key, &value) {
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        }
    }
    Ok(tokens)
}

fn token_record(key: &[u8], value: &[u8]) -> Option<Zeroizing<String>> {
    // Chromium Local Storage: '_' + origin + NUL + encoding byte + script key.
    let key = key.strip_prefix(b"_")?;
    let end = key.iter().position(|&b| b == 0)?;
    if !matches!(
        &key[..end],
        b"https://discord.com"
            | b"https://ptb.discord.com"
            | b"https://canary.discord.com"
            | b"https://discordapp.com"
    ) {
        return None;
    }
    let script_key = decode_string(&key[end + 1..])?;
    if script_key.as_str() != "token" {
        return None;
    }
    let value = decode_string(value)?;
    let token: String = serde_json::from_str(&value).ok()?;
    let token = Zeroizing::new(token);
    looks_like_discord_token(&token).then_some(token)
}

fn decode_string(bytes: &[u8]) -> Option<Zeroizing<String>> {
    let (&encoding, data) = bytes.split_first()?;
    match encoding {
        1 => Some(Zeroizing::new(
            data.iter().map(|&b| char::from(b)).collect(),
        )),
        0 if data.len() % 2 == 0 => {
            let words = Zeroizing::new(
                data.as_chunks::<2>()
                    .0
                    .iter()
                    .copied()
                    .map(u16::from_le_bytes)
                    .collect::<Vec<_>>(),
            );
            String::from_utf16(&words).ok().map(Zeroizing::new)
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/support/linux_auth.rs"]
mod tests;
