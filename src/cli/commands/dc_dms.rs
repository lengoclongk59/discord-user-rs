//! `discord dc dms` — list open DM and group-DM channels.

use anyhow::Result;

use crate::cli::api::WriteApi;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = WriteApi::new(&token);
    let result = api.list_dm_channels().await?;

    if ctx.json {
        output::print_json(&result);
        return Ok(());
    }

    let Some(arr) = result.as_array() else {
        output::print_json(&result);
        return Ok(());
    };
    if arr.is_empty() {
        output::dim("No open DM channels.");
        return Ok(());
    }

    let rows: Vec<Vec<String>> = arr
        .iter()
        .map(|c| {
            let id = c["id"].as_str().unwrap_or("").to_string();
            // type 1 = DM, type 3 = group DM
            let kind = match c["type"].as_u64() {
                Some(1) => "DM".to_string(),
                Some(3) => "Group".to_string(),
                Some(t) => format!("type {}", t),
                None => "?".to_string(),
            };
            // Group DMs may carry a `name`; DMs name themselves by recipient.
            let label = c["name"].as_str().filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| {
                c["recipients"]
                    .as_array()
                    .map(|rs| {
                        rs.iter()
                            .filter_map(|r| {
                                r["global_name"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                    .or_else(|| r["username"].as_str())
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default()
            });
            vec![id, kind, label]
        })
        .collect();
    output::print_table(&["id", "type", "recipients"], &rows);
    output::dim(&format!("\n{} channel(s)", arr.len()));
    Ok(())
}
