use crate::cli::Args;
use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize)]
pub struct TargetEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    /// RFC 3339 timestamp of the most-recently-modified file inside target/
    pub last_modified: String,
}

pub fn find_targets(args: &Args) -> anyhow::Result<Vec<TargetEntry>> {
    let cutoff: Option<SystemTime> = if args.older_than > 0 {
        Some(SystemTime::now() - Duration::from_secs(args.older_than * 86400))
    } else {
        None
    };

    let mut targets: Vec<TargetEntry> = Vec::new();
    let mut skip_prefixes: Vec<PathBuf> = Vec::new();

    let walker = WalkBuilder::new(&args.path)
        .hidden(!args.include_hidden)
        .ignore(true)
        .git_ignore(true)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path().to_path_buf();

        // Skip subtrees we've already identified as target/ dirs (avoid nested target/).
        if skip_prefixes.iter().any(|p| path.starts_with(p)) {
            continue;
        }

        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
            && entry.file_name() == "target"
        {
            // Check it's a Cargo target dir by looking for a `.rustc_info.json`
            // or any `.d` files — a heuristic to avoid false positives.
            let is_cargo_target = path.join(".rustc_info.json").exists()
                || path.join("CACHEDIR.TAG").exists()
                || fs::read_dir(&path)
                    .map(|mut d| d.any(|e| {
                        e.ok()
                            .and_then(|e| e.path().extension().map(|x| x == "d"))
                            .unwrap_or(false)
                    }))
                    .unwrap_or(false);

            if !is_cargo_target {
                continue;
            }

            let (size, last_mod) = dir_stats(&path);

            // Apply --older-than filter.
            if let Some(cutoff_time) = cutoff {
                if last_mod > cutoff_time {
                    continue;
                }
            }

            let last_modified = format_time(last_mod);
            targets.push(TargetEntry { path: path.clone(), size_bytes: size, last_modified });
            skip_prefixes.push(path);
        }
    }

    targets.sort_by_key(|e| core::cmp::Reverse(e.size_bytes));
    Ok(targets)
}

/// Recursively sum file sizes and find the newest mtime in a directory.
fn dir_stats(dir: &std::path::Path) -> (u64, SystemTime) {
    let mut total = 0u64;
    let mut newest = SystemTime::UNIX_EPOCH;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = fs::metadata(&path) {
                if meta.is_file() {
                    total += meta.len();
                    if let Ok(mtime) = meta.modified() {
                        if mtime > newest {
                            newest = mtime;
                        }
                    }
                } else if meta.is_dir() {
                    let (sub_size, sub_newest) = dir_stats(&path);
                    total += sub_size;
                    if sub_newest > newest {
                        newest = sub_newest;
                    }
                }
            }
        }
    }

    (total, newest)
}

fn format_time(t: SystemTime) -> String {
    // Format as a simple ISO 8601 timestamp without external deps.
    match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            // Quick & dirty UTC breakdown (no chrono dep).
            let s = secs % 60;
            let m = (secs / 60) % 60;
            let h = (secs / 3600) % 24;
            let days = secs / 86400;
            // Approximate date from days since epoch.
            let (year, month, day) = days_to_ymd(days);
            format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
        }
        Err(_) => "1970-01-01T00:00:00Z".to_string(),
    }
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Gregorian calendar calculation (no external deps).
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub fn delete_target(entry: &TargetEntry) -> anyhow::Result<()> {
    fs::remove_dir_all(&entry.path)?;
    Ok(())
}
