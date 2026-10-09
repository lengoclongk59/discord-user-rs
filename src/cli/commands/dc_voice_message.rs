//! `discord dc voice-message <CHANNEL> <FILE> [--duration <SECS>] [--waveform <BASE64>]`
//! — send an audio file as a Discord voice message.
//!
//! A voice message is an ordinary multipart attachment upload with the message
//! `flags` IS_VOICE_MESSAGE bit set and the attachment carrying `duration_secs`
//! and `waveform` metadata. True audio duration probing is out of scope: when
//! `--duration` is omitted we send a sane default (1.0s). When `--waveform` is
//! omitted we generate a short placeholder waveform (a simple ramp) and
//! base64-encode it.

use std::path::Path;

use anyhow::{bail, Result};
use base64::Engine as _;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

/// Default duration sent when the user does not supply `--duration`.
const DEFAULT_DURATION_SECS: f64 = 1.0;

/// Build a placeholder waveform: a short ascending ramp of bytes, base64
/// encoded. Discord only needs a plausible byte array here; it is not derived
/// from the audio.
fn placeholder_waveform() -> String {
    let bytes: Vec<u8> = (0..32u16).map(|i| (i * 8) as u8).collect();
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

pub async fn run(
    ctx: &Ctx,
    channel: &str,
    file: &Path,
    duration: Option<f64>,
    waveform: Option<&str>,
) -> Result<()> {
    if !file.is_file() {
        bail!("audio file not found: {}", file.display());
    }

    let duration_secs = duration.unwrap_or(DEFAULT_DURATION_SECS);
    let generated = waveform.is_none();
    let waveform = match waveform {
        Some(w) => w.to_string(),
        None => placeholder_waveform(),
    };

    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    let result = api
        .send_voice_message(channel, file, duration_secs, &waveform)
        .await?;

    if ctx.json {
        output::print_json(&result);
    } else {
        let id = result["id"].as_str().unwrap_or("?");
        let note = if generated {
            " (placeholder waveform)"
        } else {
            ""
        };
        output::success(&format!(
            "Sent voice message {} ({:.1}s){} in channel {}",
            id, duration_secs, note, channel
        ));
    }
    Ok(())
}
