use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "tpt-cargo-scrub",
    about = "Find, analyse, and clean Rust target/ directories across your machine",
    version
)]
pub struct Args {
    /// Root path to search from
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Show what would be deleted without actually deleting
    #[arg(long)]
    pub dry_run: bool,

    /// Only show target/ dirs not accessed in this many days
    #[arg(long, default_value_t = 0)]
    pub older_than: u64,

    /// Output results as JSON (implies --no-tui)
    #[arg(long)]
    pub json: bool,

    /// Disable the interactive TUI; print a plain summary instead
    #[arg(long)]
    pub no_tui: bool,

    /// Include hidden directories in traversal
    #[arg(long)]
    pub include_hidden: bool,
}
