//! Subcommand dispatch.

mod auth;
mod dc_audit;
mod dc_channels;
mod dc_delete;
mod dc_dm;
mod dc_download;
mod dc_edit;
mod dc_emojis;
mod dc_events;
mod dc_guilds;
mod dc_history;
mod dc_info;
mod dc_members;
mod dc_pin;
mod dc_pins;
mod dc_profile;
mod dc_react;
mod dc_relationships;
mod dc_roles;
mod dc_search_remote;
mod dc_send;
mod dc_snapshot;
mod dc_stickers;
mod dc_sync;
mod dc_sync_all;
mod dc_tail;
mod dc_threads;
mod dc_typing;
// Write/client command groups produced by the multi-agent buildout.
// `pub(crate)` so `args.rs` can name the nested `*Cmd` enums.
pub(crate) mod dcw_bans;
pub(crate) mod dcw_channels;
pub(crate) mod dcw_compose;
pub(crate) mod dcw_emojis;
pub(crate) mod dcw_events;
pub(crate) mod dcw_friends;
pub(crate) mod dcw_guild;
pub(crate) mod dcw_invites;
pub(crate) mod dcw_members;
pub(crate) mod dcw_msgops;
pub(crate) mod dcw_perms;
pub(crate) mod dcw_roles;
pub(crate) mod dcw_stickers;
pub(crate) mod dcw_threads;
pub(crate) mod dcw_webhooks;
mod export;
mod purge;
mod recent;
pub(crate) mod resolve;
mod search;
mod stats;
mod status;
mod timeline;
mod today;
mod top;
mod whoami;

use std::path::PathBuf;

use anyhow::Result;

use crate::cli::args::{Cli, Cmd, DcCmd, TimelineGrouping};

pub struct Ctx {
    pub token_flag: Option<String>,
    pub db_path: PathBuf,
    pub json: bool,
}

impl Ctx {
    fn from_cli(cli: &Cli) -> Self {
        Self {
            token_flag: cli.token.clone(),
            db_path: cli
                .db
                .clone()
                .unwrap_or_else(|| PathBuf::from("discord.db")),
            json: cli.json,
        }
    }
}

pub async fn dispatch(cli: Cli) -> Result<()> {
    let ctx = Ctx::from_cli(&cli);
    match cli.cmd {
        Cmd::Auth(args) => auth::run(args).await,
        Cmd::Status => status::run(&ctx).await,
        Cmd::Whoami => whoami::run(&ctx).await,
        Cmd::Dc(DcCmd::Guilds) => dc_guilds::run(&ctx).await,
        Cmd::Dc(DcCmd::Channels { guild }) => dc_channels::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Sync { channel, limit }) => dc_sync::run(&ctx, &channel, limit).await,
        Cmd::Dc(DcCmd::SyncAll { limit }) => dc_sync_all::run(&ctx, limit).await,
        Cmd::Dc(DcCmd::History { channel, limit }) => dc_history::run(&ctx, &channel, limit).await,
        Cmd::Dc(DcCmd::Tail {
            channel,
            limit,
            once,
        }) => dc_tail::run(&ctx, &channel, limit, once).await,
        Cmd::Dc(DcCmd::SearchRemote {
            guild,
            keyword,
            channel,
            limit,
        }) => dc_search_remote::run(&ctx, &guild, &keyword, channel.as_deref(), limit).await,
        Cmd::Dc(DcCmd::Members { guild, limit }) => dc_members::run(&ctx, &guild, limit).await,
        Cmd::Dc(DcCmd::Info { guild }) => dc_info::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Pins { channel }) => dc_pins::run(&ctx, &channel).await,
        Cmd::Dc(DcCmd::Threads { guild }) => dc_threads::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Relationships) => dc_relationships::run(&ctx).await,
        Cmd::Dc(DcCmd::Roles { guild }) => dc_roles::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Emojis { guild }) => dc_emojis::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Profile { user_id }) => dc_profile::run(&ctx, user_id.as_deref()).await,
        Cmd::Dc(DcCmd::Stickers { guild }) => dc_stickers::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Audit { guild, limit }) => dc_audit::run(&ctx, &guild, limit).await,
        Cmd::Dc(DcCmd::Events { guild }) => dc_events::run(&ctx, &guild).await,
        Cmd::Dc(DcCmd::Download { channel, output, hours }) => dc_download::run(&ctx, &channel, &output, hours).await,
        Cmd::Dc(DcCmd::Snapshot { guild, output }) => dc_snapshot::run(&ctx, &guild, output).await,
        Cmd::Dc(DcCmd::Send {
            channel,
            content,
            reply,
        }) => dc_send::run(&ctx, &channel, &content, reply.as_deref()).await,
        Cmd::Dc(DcCmd::Edit {
            channel,
            message,
            content,
        }) => dc_edit::run(&ctx, &channel, &message, &content).await,
        Cmd::Dc(DcCmd::Delete {
            channel,
            message,
            yes,
        }) => dc_delete::run(&ctx, &channel, &message, yes).await,
        Cmd::Dc(DcCmd::React {
            channel,
            message,
            emoji,
        }) => dc_react::run(&ctx, &channel, &message, &emoji, false).await,
        Cmd::Dc(DcCmd::Unreact {
            channel,
            message,
            emoji,
        }) => dc_react::run(&ctx, &channel, &message, &emoji, true).await,
        Cmd::Dc(DcCmd::Pin { channel, message }) => {
            dc_pin::run(&ctx, &channel, &message, false).await
        }
        Cmd::Dc(DcCmd::Unpin { channel, message }) => {
            dc_pin::run(&ctx, &channel, &message, true).await
        }
        Cmd::Dc(DcCmd::Typing { channel }) => dc_typing::run(&ctx, &channel).await,
        Cmd::Dc(DcCmd::Dm { user_id, content }) => dc_dm::run(&ctx, &user_id, &content).await,
        Cmd::Dc(DcCmd::Thread(c)) => dcw_threads::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Channel(c)) => dcw_channels::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Perms(c)) => dcw_perms::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Member(c)) => dcw_members::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Ban(c)) => dcw_bans::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Role(c)) => dcw_roles::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Invite(c)) => dcw_invites::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Friend(c)) => dcw_friends::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Emoji(c)) => dcw_emojis::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Sticker(c)) => dcw_stickers::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Message(c)) => dcw_msgops::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Guild(c)) => dcw_guild::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Event(c)) => dcw_events::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Webhook(c)) => dcw_webhooks::dispatch(&ctx, c).await,
        Cmd::Dc(DcCmd::Compose(c)) => dcw_compose::dispatch(&ctx, c).await,
        Cmd::Search(args) => search::run(&ctx, args),
        Cmd::Recent(args) => recent::run(&ctx, args),
        Cmd::Stats => stats::run(&ctx),
        Cmd::Today(args) => today::run(&ctx, args.channel.as_deref()),
        Cmd::Top(args) => top::run(&ctx, args.channel.as_deref(), args.hours, args.limit),
        Cmd::Timeline(args) => {
            let by = match args.by {
                TimelineGrouping::Day => "day",
                TimelineGrouping::Hour => "hour",
            };
            timeline::run(&ctx, args.channel.as_deref(), args.hours, by)
        }
        Cmd::Export(args) => export::run(&ctx, args),
        Cmd::Purge(args) => purge::run(&ctx, args),
    }
}
