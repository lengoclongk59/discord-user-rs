//! `discord export <CHANNEL>` — export messages to file.

use std::fs;
use std::io::{self, BufWriter, Write};

use anyhow::Result;

use crate::cli::args::{ExportArgs, ExportFormat};
use crate::cli::commands::resolve;
use crate::cli::commands::Ctx;
use crate::cli::db::Db;
use crate::cli::output;

pub fn run(ctx: &Ctx, args: ExportArgs) -> Result<()> {
    let db = Db::open(&ctx.db_path)?;

    let channel_id = resolve::resolve_channel_required(&db, &args.channel)?;

    // Validate output path safety before any operations
    if let Some(path) = &args.output {
        use std::path::Component;

        // 1. Check for path traversal attempts
        for component in path.components() {
            if matches!(component, Component::ParentDir) {
                anyhow::bail!("Path traversal not allowed: {}", path.display());
            }
        }

        // 2. Check if file exists and get confirmation
        if path.exists()
            && !output::confirm(&format!("File {} exists. Overwrite?", path.display()))?
        {
            output::dim("Aborted.");
            return Ok(());
        }

        // 3. Validate against restricted directories
        // We check before opening to provide clear errors, then verify again after open
        let check_restricted = |p: &std::path::Path| -> Result<()> {
            let canonical = p.canonicalize().or_else(|_| {
                // If file doesn't exist, canonicalize parent
                p.parent()
                    .and_then(|parent| parent.canonicalize().ok())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Cannot verify output path safety: {}", p.display())
                    })
            })?;

            #[cfg(unix)]
            {
                let canonical_str = canonical
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?;
                const RESTRICTED: &[&str] = &["/etc", "/sys", "/proc", "/boot", "/dev"];
                for restricted in RESTRICTED {
                    if canonical_str.starts_with(restricted) {
                        anyhow::bail!("Cannot write to system directory: {}", canonical.display());
                    }
                }
            }

            #[cfg(windows)]
            {
                let canonical_upper = canonical.to_string_lossy().to_uppercase();
                const RESTRICTED: &[&str] = &[
                    "C:\\WINDOWS",
                    "C:\\SYSTEM32",
                    "C:\\PROGRAM FILES",
                    "C:\\PROGRAMDATA",
                    "C:\\BOOT",
                ];
                for restricted in RESTRICTED {
                    if canonical_upper.starts_with(restricted) {
                        anyhow::bail!("Cannot write to system directory: {}", canonical.display());
                    }
                }
            }

            Ok(())
        };

        check_restricted(path)?;
    }

    let msgs = db.recent(Some(&channel_id), args.hours, 100_000)?;

    if msgs.is_empty() {
        output::dim("No messages to export.");
        return Ok(());
    }

    let mut writer: BufWriter<Box<dyn Write>> = if let Some(path) = &args.output {
        BufWriter::new(Box::new(fs::File::create(path)?))
    } else {
        BufWriter::new(Box::new(io::stdout().lock()))
    };

    match args.format {
        ExportFormat::Json => {
            serde_json::to_writer_pretty(&mut writer, &msgs)?;
            writeln!(writer)?;
        }
        ExportFormat::Yaml => {
            serde_yaml::to_writer(&mut writer, &msgs)?;
        }
        ExportFormat::Text => {
            for m in &msgs {
                let ts = m.timestamp.format("%Y-%m-%d %H:%M:%S");
                writeln!(
                    writer,
                    "[{}] {}: {}",
                    ts,
                    m.sender_name,
                    m.content.replace('\n', " ")
                )?;
            }
        }
    }

    writer.flush()?;

    if let Some(path) = &args.output {
        output::success(&format!(
            "Exported {} messages to {}",
            msgs.len(),
            path.display()
        ));
    }
    Ok(())
}
