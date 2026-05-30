//! Write-surface containment test for the bundled CLI.
//!
//! The CLI is a full Discord client (it sends, edits, deletes, reacts, …),
//! but raw HTTP write verbs must stay confined to the API layer
//! (`src/cli/api.rs`, home of `WriteApi`). Every other CLI module — command
//! handlers, db, output, config — must go *through* `WriteApi`/`Api` rather
//! than constructing write requests directly. This grep gate fails if a
//! forbidden write-method call appears outside the sanctioned files.
//!
//! Sanctioned files:
//! - `src/cli/api.rs`        — the `WriteApi` layer (the one place writes live)
//! - `src/cli/auth/windows.rs` — `validate_token` uses raw reqwest pre-token
//!
//! Only meaningful when the `cli` feature is on.

#![cfg(feature = "cli")]

use std::fs;
use walkdir::WalkDir;

// Method-call substrings that, if present, indicate a write path. Covers
// every public write helper currently exposed by `DiscordHttpClient` plus
// raw `Method::*` literals in case anyone constructs requests directly.
const FORBIDDEN: &[&str] = &[
    ".post(",
    ".patch(",
    ".put(",
    ".delete(",
    ".delete_with_response(",
    ".post_empty(",
    ".put_empty(",
    ".post_no_response(",
    ".patch_no_response(",
    ".post_with_referer(",
    ".post_multipart(",
    ".post_raw_multipart(",
    // Lower-level request builders that take a Method argument — these can
    // issue any verb without a `Method::*` literal on the call line, so list
    // them explicitly (e.g. `put_optional`, the `request*` family).
    ".put_optional(",
    ".request(",
    ".request_no_response(",
    ".request_optional(",
    ".request_with_reason(",
    ".request_with_reason_no_response(",
    "Method::POST",
    "Method::PATCH",
    "Method::PUT",
    "Method::DELETE",
];

#[test]
fn write_methods_confined_to_api_layer() {
    let mut violations: Vec<String> = Vec::new();
    for entry in WalkDir::new("src/cli").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        // Sanctioned files where raw write verbs are allowed:
        //  - cli/api.rs          : the core WriteApi layer
        //  - cli/api_*.rs        : per-domain WriteApi extension modules
        //  - cli/auth/windows.rs : validate_token uses raw reqwest pre-token
        let path_str = path.to_string_lossy().replace('\\', "/");
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path_str.ends_with("cli/api.rs")
            || path_str.ends_with("cli/auth/windows.rs")
            || (path_str.contains("/cli/") && file_name.starts_with("api_"))
        {
            continue;
        }
        let body = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for needle in FORBIDDEN {
            for (line_no, line) in body.lines().enumerate() {
                let stripped = line.split("//").next().unwrap_or(line);
                if stripped.contains(needle) {
                    violations.push(format!(
                        "{}:{}: forbidden write call {:?} in `{}`",
                        path_str,
                        line_no + 1,
                        needle,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Write verbs must be confined to src/cli/api.rs (WriteApi). Found raw write calls elsewhere:\n{}",
        violations.join("\n")
    );
}
