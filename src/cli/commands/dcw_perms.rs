//! `discord dc perms ...` — channel permission overwrites.

use anyhow::Result;
use clap::Subcommand;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[derive(Subcommand, Debug)]
pub enum PermsCmd {
    /// Set a permission overwrite on a channel (PUT).
    Set {
        /// Channel snowflake ID.
        channel: String,
        /// Role or member snowflake ID to overwrite.
        overwrite_id: String,
        /// Overwrite type: role (0) or member (1).
        #[arg(long, value_name = "role|member", default_value = "role")]
        r#type: String,
        /// Allowed permissions bitfield (stringified integer).
        #[arg(long, default_value = "0")]
        allow: String,
        /// Denied permissions bitfield (stringified integer).
        #[arg(long, default_value = "0")]
        deny: String,
    },
    /// Delete a permission overwrite from a channel (DELETE).
    Clear {
        /// Channel snowflake ID.
        channel: String,
        /// Role or member snowflake ID whose overwrite to remove.
        overwrite_id: String,
    },
    /// List permission overwrites for a channel (GET channel).
    List {
        /// Channel snowflake ID.
        channel: String,
    },
}

pub async fn dispatch(ctx: &Ctx, cmd: PermsCmd) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);

    match cmd {
        PermsCmd::Set {
            channel,
            overwrite_id,
            r#type,
            allow,
            deny,
        } => {
            let kind: u8 = match r#type.as_str() {
                "role" | "0" => 0,
                "member" | "1" => 1,
                other => {
                    return Err(anyhow::anyhow!(
                        "unknown type {:?}: expected role or member",
                        other
                    ))
                }
            };
            api.perms_set(&channel, &overwrite_id, kind, &allow, &deny)
                .await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "action": "set",
                    "channel_id": channel,
                    "overwrite_id": overwrite_id,
                    "type": kind,
                    "allow": allow,
                    "deny": deny,
                }));
            } else {
                output::success(&format!(
                    "Permission overwrite set on channel {} for {}",
                    channel, overwrite_id
                ));
            }
        }

        PermsCmd::Clear {
            channel,
            overwrite_id,
        } => {
            api.perms_clear(&channel, &overwrite_id).await?;
            if ctx.json {
                output::print_json(&serde_json::json!({
                    "action": "clear",
                    "channel_id": channel,
                    "overwrite_id": overwrite_id,
                }));
            } else {
                output::success(&format!(
                    "Permission overwrite cleared on channel {} for {}",
                    channel, overwrite_id
                ));
            }
        }

        PermsCmd::List { channel } => {
            let overwrites = api.perms_list(&channel).await?;
            if ctx.json {
                output::print_json(&overwrites);
            } else {
                let arr = overwrites.as_array().cloned().unwrap_or_default();
                if arr.is_empty() {
                    output::dim("No permission overwrites.");
                } else {
                    let rows: Vec<Vec<String>> = arr
                        .iter()
                        .map(|o| {
                            let id = o
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let kind = match o.get("type").and_then(|v| v.as_u64()) {
                                Some(0) => "role",
                                Some(1) => "member",
                                _ => "?",
                            }
                            .to_string();
                            let allow = o
                                .get("allow")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string();
                            let deny = o
                                .get("deny")
                                .and_then(|v| v.as_str())
                                .unwrap_or("0")
                                .to_string();
                            vec![id, kind, allow, deny]
                        })
                        .collect();
                    output::print_table(&["id", "type", "allow", "deny"], &rows);
                    output::dim(&format!("\n{} overwrite(s)", arr.len()));
                }
            }
        }
    }

    Ok(())
}
