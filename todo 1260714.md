# tpt-dev-dx — Task Checklist

## Phase 1 — Workspace Scaffolding
- [x] `Cargo.toml` workspace root (resolver = "2", MSRV 1.75)
- [x] `LICENSE-MIT`
- [x] `LICENSE-APACHE`
- [x] `README.md` workspace overview
- [x] `.github/workflows/ci.yml` (fmt + clippy + test, 3 platforms)

## Phase 2 — tpt-temp-fs
- [x] `crates/tpt-temp-fs/Cargo.toml`
- [x] `TempDir::new()` and `TempDir::with_prefix()`
- [x] `Drop` impl for auto-cleanup
- [x] `path()`, `child()`, `write_file()`, `create_dir()` helpers
- [x] `into_persistent()` opt-out
- [x] `scaffold_from_str()` — JSON/YAML directory tree
- [x] Unit tests
- [x] `crates/tpt-temp-fs/README.md`
- [x] `crates/tpt-temp-fs/CHANGELOG.md`

## Phase 3 — tpt-env-mocker
- [x] `crates/tpt-env-mocker-macros/Cargo.toml` (proc-macro crate)
- [x] `#[tpt_env(...)]` attribute macro implementation
- [x] `crates/tpt-env-mocker/Cargo.toml`
- [x] `ENV_MUTEX` global static
- [x] `MockEnv` builder struct
- [x] `EnvGuard` with Drop restore
- [x] `macros` feature flag re-exporting proc-macro
- [x] Unit + integration tests (parallel env access)
- [x] `crates/tpt-env-mocker/README.md`
- [x] `crates/tpt-env-mocker/CHANGELOG.md`

## Phase 4 — tpt-snapshot-lite
- [x] `crates/tpt-snapshot-lite/Cargo.toml` (zero deps)
- [x] `assert_snapshot!` macro (external .snap files)
- [x] `assert_snapshot_debug!` macro
- [x] `Snapshot` struct with `assert_display()` and `assert_debug()`
- [x] Snap file create-on-first-run behaviour
- [x] `UPDATE_SNAPSHOTS=1` overwrite mode
- [x] Unit tests
- [x] `crates/tpt-snapshot-lite/README.md`
- [x] `crates/tpt-snapshot-lite/CHANGELOG.md`

## Phase 5 — tpt-faker-rs
- [x] `crates/tpt-faker-rs-derive/Cargo.toml` (proc-macro crate)
- [x] `#[derive(Fake)]` macro with `#[fake(...)]` field attrs
- [x] `crates/tpt-faker-rs/Cargo.toml`
- [x] `Fake` trait definition
- [x] Generators: `name`, `first_name`, `last_name`, `email`, `username`
- [x] Generators: `url`, `ipv4`, `ipv6`, `uuid`
- [x] Generators: `luhn_card` (Luhn algorithm)
- [x] Generators: `iso_date`, `iso_datetime`
- [x] Generators: `word`, `sentence`, `paragraph`
- [x] Generators: `range = "lo..=hi"` for numerics
- [x] `serde` optional feature
- [x] Unit tests for each generator
- [x] `crates/tpt-faker-rs/README.md`
- [x] `crates/tpt-faker-rs/CHANGELOG.md`

## Phase 6 — tpt-cargo-scrub
- [x] `crates/tpt-cargo-scrub/Cargo.toml` (binary crate)
- [x] CLI arg parsing with clap (`--dry-run`, `--older-than`, `--json`, `--no-tui`)
- [x] `ignore::Walk` traversal to find all `target/` dirs
- [x] Recursive size calculation per `target/`
- [x] `--older-than` filter via `fs::metadata` mtime
- [x] `--json` output mode
- [x] `--no-tui` plain summary output
- [x] ratatui TUI: left panel (tree), right panel (details)
- [x] TUI: colour coding (red/yellow/green by size)
- [x] TUI: keybindings (space=select, d=delete, a=all, q=quit)
- [x] Deletion confirmation flow
- [ ] Integration tests (dry-run on fixture dirs)
- [x] `crates/tpt-cargo-scrub/README.md`
- [x] `crates/tpt-cargo-scrub/CHANGELOG.md`

## Phase 7 — Crates.io Release Prep
- [ ] Set real GitHub repository URL in workspace Cargo.toml
- [ ] Verify all Cargo.toml metadata (description, keywords, categories, repository, readme)
- [ ] `cargo publish --dry-run` — tpt-temp-fs
- [ ] `cargo publish --dry-run` — tpt-snapshot-lite
- [ ] `cargo publish --dry-run` — tpt-env-mocker-macros
- [ ] `cargo publish --dry-run` — tpt-env-mocker
- [ ] `cargo publish --dry-run` — tpt-faker-rs-derive
- [ ] `cargo publish --dry-run` — tpt-faker-rs
- [ ] `cargo publish --dry-run` — tpt-cargo-scrub
- [ ] `cargo doc --workspace --no-deps` — zero warnings
- [ ] Final `cargo test --workspace` green
