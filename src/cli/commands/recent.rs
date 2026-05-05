//! `discord recent [-c CH] [--hours H] [-n N]` — newest stored messages.

use anyhow::Result;

use crate::cli::args::RecentArgs;
use crate::cli::commands::resolve;
use crate::cli::commands::Ctx;
use crate::cli::db::Db;
use crate::cli::output;

pub fn run(ctx: &Ctx, args: RecentArgs) -> Result<()> {
    let db = Db::open(&ctx.db_path)?;
    let chan = resolve::resolve_channel(&db, args.channel.as_deref())?;

    let rows = db.recent(chan.as_deref(), args.hours, args.limit)?;
    if ctx.json {
        output::print_json(&rows);
        return Ok(());
    }
    if rows.is_empty() {
        output::dim("No messages.");
        return Ok(());
    }
    for m in &rows {
        println!("{}", output::format_message(m));
    }
    Ok(())
}
