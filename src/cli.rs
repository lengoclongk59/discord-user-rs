//! Bundled archival CLI — only compiled when the `cli` feature is enabled.
//!
//! This is the merged-in `discord-cli` binary tree. It re-uses the parent
//! crate's HTTP client (`crate::client::DiscordHttpClient`) and route table
//! (`crate::route::Route`) for read-only access, plus the gateway re-export
//! (`crate::DiscordUser`) for `dc tail`. All other modules here are CLI-only
//! and never compiled into the published library.

pub mod api;
pub mod args;
pub mod auth;
pub mod commands;
pub mod config;
pub mod db;
pub mod output;
pub mod types;
