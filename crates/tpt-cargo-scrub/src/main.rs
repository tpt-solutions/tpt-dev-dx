mod cli;
mod scan;
mod tui;

use cli::Args;
use clap::Parser;
use std::io::IsTerminal;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Scan for target/ directories.
    let entries = scan::find_targets(&args)?;

    if args.json {
        print_json(&entries)?;
        return Ok(());
    }

    if args.no_tui || !std::io::stdout().is_terminal() {
        print_summary(&entries, args.dry_run);
        return Ok(());
    }

    tui::run(entries, args.dry_run)
}

fn print_json(entries: &[scan::TargetEntry]) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(entries)?;
    println!("{json}");
    Ok(())
}

fn print_summary(entries: &[scan::TargetEntry], dry_run: bool) {
    use humansize::{format_size, BINARY};

    let total: u64 = entries.iter().map(|e| e.size_bytes).sum();
    println!(
        "Found {} target/ directories ({} total)",
        entries.len(),
        format_size(total, BINARY)
    );
    for e in entries {
        println!("  {} — {}", e.path.display(), format_size(e.size_bytes, BINARY));
    }
    if dry_run {
        println!("\n[dry-run] No files deleted.");
    }
}
