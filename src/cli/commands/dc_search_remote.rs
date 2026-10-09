//! `discord dc search <GUILD> [KEYWORD]` — remote Discord search.

use anyhow::{bail, Result};

use crate::cli::api::Api;
use crate::cli::commands::Ctx;
use crate::cli::config;
use crate::cli::output;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    ctx: &Ctx,
    guild: &str,
    keyword: &str,
    channel: Option<&str>,
    author: Option<&str>,
    mentions: Option<&str>,
    has: Option<&str>,
    pinned: bool,
    before: Option<&str>,
    after: Option<&str>,
    limit: u32,
) -> Result<()> {
    let token = config::resolve_token(ctx.token_flag.clone())?;
    let api = Api::new(&token);

    // A search needs at least one constraint. An empty keyword is only valid
    // when another filter is present.
    let content = if keyword.is_empty() {
        None
    } else {
        Some(keyword)
    };
    if content.is_none() && author.is_none() && mentions.is_none() && has.is_none() && !pinned {
        bail!("provide a keyword or at least one of --author, --mentions, --has, --pinned");
    }

    let guild_id = api.resolve_guild_id(guild).await?;
    let results = api
        .search_guild_messages_advanced(
            &guild_id,
            content,
            channel,
            author,
            mentions,
            has,
            if pinned { Some(true) } else { None },
            before,
            after,
            limit,
        )
        .await?;

    if ctx.json {
        output::print_json(&results);
    } else if results.is_empty() {
        output::dim("No results found.");
    } else {
        for m in &results {
            println!("{}", output::format_message(m));
        }
        output::dim(&format!("\n{} results", results.len()));
    }
    Ok(())
}
