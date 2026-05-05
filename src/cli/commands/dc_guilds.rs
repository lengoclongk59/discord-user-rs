//! `discord dc guilds` — list joined guilds.

use anyhow::Result;

use crate::cli::api::Api;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

pub async fn run(ctx: &Ctx) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = Api::new(&token);
    let guilds = api.list_guilds().await?;
    if ctx.json {
        output::print_json(&guilds);
        return Ok(());
    }
    let rows: Vec<Vec<String>> = guilds
        .iter()
        .map(|g| {
            vec![
                g.id.clone(),
                g.name.clone(),
                if g.owner { "✓" } else { "" }.to_string(),
            ]
        })
        .collect();
    output::print_table(&["id", "name", "owner"], &rows);
    output::dim(&format!("\nTotal: {} guilds", guilds.len()));
    Ok(())
}
