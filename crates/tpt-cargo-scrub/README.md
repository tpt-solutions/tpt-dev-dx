# tpt-cargo-scrub

[![crates.io](https://img.shields.io/crates/v/tpt-cargo-scrub.svg)](https://crates.io/crates/tpt-cargo-scrub)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

Blazing-fast CLI/TUI tool to find, analyse, and clean Rust `target/` directories across your entire machine.

Rust developers constantly run out of SSD space due to fragmented `target/` folders scattered across projects. `tpt-cargo-scrub` uses `.gitignore`-aware traversal to find them all and shows you exactly how much space you can reclaim.

## Install

```sh
cargo install tpt-cargo-scrub
```

## Usage

```sh
# Interactive TUI (default when stdout is a terminal)
tpt-cargo-scrub ~/projects

# Dry run — see what would be deleted without deleting
tpt-cargo-scrub ~/projects --dry-run

# Only show target/ dirs not accessed in 30 days
tpt-cargo-scrub ~/projects --older-than 30

# JSON output for scripting
tpt-cargo-scrub ~/projects --json

# Non-interactive summary
tpt-cargo-scrub ~/projects --no-tui
```

## TUI Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Space` | Toggle selection |
| `a` | Select / deselect all |
| `d` | Delete selected |
| `q` / `Esc` | Quit |

Size colour coding: 🔴 > 1 GB · 🟡 > 100 MB · 🟢 smaller

## License

MIT OR Apache-2.0
