# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`tpt-dev-dx` is a Cargo workspace of independent, production-ready Rust crates focused on eliminating boilerplate in Rust backend development and testing. Most crates are published separately to crates.io under the `tpt-` prefix; crates prefixed `out-` instead (e.g. `out-faker-rs`) are held back from publishing — usually because a review found them too redundant with an existing, well-established crate in the ecosystem — and carry `publish = false`. See `spec.txt` for the original design rationale behind each crate (problem solved, design details) — read it before making architectural changes to a crate.

## Commands

```sh
# Format check (CI runs this — must be clean)
cargo fmt --all --check
cargo fmt --all                      # apply formatting

# Lint (CI treats warnings as errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Test entire workspace
cargo test --workspace --all-features

# Test a single crate
cargo test -p out-faker-rs

# Test a single test by name
cargo test -p out-faker-rs some_test_name

# MSRV check (workspace MSRV is 1.75, enforced in CI)
cargo check --workspace --all-features
```

CI (`.github/workflows/ci.yml`) runs fmt-check, clippy (`-D warnings`), and `cargo test --workspace --all-features` on Linux/macOS/Windows, plus a separate MSRV job pinned to Rust 1.75. Match these locally before pushing.

## Architecture

This is a workspace (`members = ["crates/*"]`), not a single crate. Shared package metadata (version, edition, MSRV, license, repo) and shared dependency versions live in the root `Cargo.toml` under `[workspace.package]` / `[workspace.dependencies]` — individual crates reference them with `.workspace = true` rather than pinning their own versions. When bumping a shared dependency or the workspace version, edit the root `Cargo.toml`, not per-crate files.

Crates (each has its own `README.md` and `CHANGELOG.md` — update both when a crate's public behavior changes):

- **`tpt-temp-fs`** — ephemeral, isolated filesystem fixtures for integration tests. Uses `Drop` for guaranteed cleanup even on panic. `scaffold` feature (default) pulls in `serde`/`serde_json`/`serde_yaml` to build directory trees from YAML/JSON definitions.
- **`tpt-env-mocker`** + **`tpt-env-mocker-macros`** — safe, isolated env-var mocking for async tests, using a global `Mutex` to serialize environment mutation across parallel tests. `tpt-env-mocker-macros` is a proc-macro crate providing the `#[tpt_env(...)]` attribute; it's an internal dependency (`optional = true`, gated behind the `macros` feature, default-on) and is not meant to be depended on directly.
- **`out-faker-rs`** + **`out-faker-rs-derive`** — strongly-typed, realistic mock data generation (e.g. Luhn-valid card numbers, locale-aware names, valid ISO dates) via a derivable `Fake` trait. `out-faker-rs-derive` is the internal proc-macro crate implementing the derive; not meant to be depended on directly. Optional `serde` feature. Not published (`out-` prefix) — deemed too redundant with the `fake` crate.
- **`tpt-snapshot-lite`** — zero-dependency snapshot testing (`Debug`/`Display` output vs `.snap` files). Intentionally has no runtime dependencies — keep it that way; that's the crate's core value proposition.
- **`tpt-cargo-scrub`** — CLI/TUI binary (not a library) to find and clean Rust `target/` directories across a machine. `scan.rs` handles `.gitignore`-aware traversal via the `ignore` crate; `tui.rs` renders an interactive `ratatui` tree-map of disk usage; `cli.rs` defines the `clap` argument surface; `main.rs` wires them together.

### Proc-macro pairing convention

Two crates in this workspace (`tpt-env-mocker`, `out-faker-rs`) each pair a public library crate with a private `-macros`/`-derive` proc-macro crate. The proc-macro crate is a path dependency of its parent, versioned in lockstep (`version = "0.1.0"` pinned explicitly, not `.workspace = true`, since these are inter-crate deps rather than workspace-shared deps). When changing the macro's expansion, the parent crate's `src/lib.rs` is where the public-facing trait/attribute is documented and re-exported.

## Licensing

Dual-licensed MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE` at repo root, referenced via `license.workspace = true`). Copyright holder is TPT Solutions.
