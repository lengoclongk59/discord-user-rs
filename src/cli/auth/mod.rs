//! Platform-specific discovery of the local Discord desktop session.

#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug)]
pub struct DiscoveredToken {
    pub token: String,
    pub source: String,
    pub username: String,
}

/// Discover and validate a local desktop token.
#[cfg(windows)]
pub async fn find_and_save_token() -> anyhow::Result<DiscoveredToken> {
    windows::find_and_save_token().await
}

#[cfg(target_os = "macos")]
pub async fn find_and_save_token() -> anyhow::Result<DiscoveredToken> {
    macos::find_and_save_token().await
}

#[cfg(target_os = "linux")]
pub async fn find_and_save_token() -> anyhow::Result<DiscoveredToken> {
    linux::find_and_save_token().await
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub async fn find_and_save_token() -> anyhow::Result<DiscoveredToken> {
    anyhow::bail!(
        "auth --save is currently unsupported on this platform. \
         Please set $DISCORD_TOKEN or pass --token <T>."
    )
}

#[cfg(any(windows, target_os = "linux"))]
use serde::Deserialize;

#[cfg(any(windows, target_os = "linux"))]
#[derive(Debug, Deserialize)]
struct MeMin {
    username: String,
    global_name: Option<String>,
}

#[cfg(any(windows, target_os = "linux"))]
/// Sanctioned raw-reqwest exception: validate a candidate token directly,
/// outside the `Api`/`ReadOnlyHttp` layer (which itself requires a token).
async fn validate_token(token: &str) -> anyhow::Result<MeMin> {
    let platform = if cfg!(windows) {
        "Windows NT 10.0; Win64; x64"
    } else {
        "X11; Linux x86_64"
    };
    let client = reqwest::Client::builder()
        .user_agent(format!("Mozilla/5.0 ({platform}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"))
        .timeout(std::time::Duration::from_secs(15))
        // Never follow redirects: we send the bare user token in
        // `Authorization`, and reqwest's default policy would replay it to
        // any 30x `Location:` host.
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let resp = client
        .get("https://discord.com/api/v10/users/@me")
        .header(reqwest::header::AUTHORIZATION, token)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {}", resp.status().as_u16()));
    }
    let me: MeMin = resp.json().await?;
    Ok(me)
}
