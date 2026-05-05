//! Read-only invariant test for the bundled CLI.
//!
//! Greps `src/cli/` for any forbidden write-method calls and fails if any are
//! found outside the single sanctioned exception (`src/cli/auth/windows.rs`).
//! This is the test-time half of the read-only invariant; the type-level
//! half is `ReadOnlyHttp` in `src/cli/api.rs`, which exposes only `.get<T>()`.
//!
//! Only meaningful when the `cli` feature is on — the library half of the
//! crate (under `src/`) intentionally implements write verbs.

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
    ".post_no_response(",
    ".patch_no_response(",
    ".post_with_referer(",
    ".post_multipart(",
    ".post_raw_multipart(",
    "Method::POST",
    "Method::PATCH",
    "Method::PUT",
    "Method::DELETE",
];

#[test]
fn no_write_methods_in_cli_src() {
    let mut violations: Vec<String> = Vec::new();
    for entry in WalkDir::new("src/cli").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        // Sole sanctioned exception: auth/windows.rs::validate_token uses raw reqwest.
        let path_str = path.to_string_lossy().replace('\\', "/");
        if path_str.ends_with("cli/auth/windows.rs") {
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
        "Read-only invariant violated:\n{}",
        violations.join("\n")
    );
}
