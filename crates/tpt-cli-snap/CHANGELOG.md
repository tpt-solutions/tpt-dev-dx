# Changelog

## [0.1.0] — 2026-07-17

### Added

- Initial release.
- `CliTest::cargo_bin(name)` / `CliTest::command(cmd)` constructors.
- Builder methods: `.arg`, `.args`, `.env`, `.env_remove`, `.stdin`, `.with_snap_dir`.
- `assert_snapshot(name)` (stdout), `assert_snapshot_stderr(name)`, `assert_snapshot_both(name)`.
- `CliOutcome` with exit-code chaining (`assert_success`, `assert_code`, `assert_failure`).
- `cli_snap_dir!()` macro pointing at the calling crate's `tests/snapshots`.
- `UPDATE_SNAPSHOTS=1` passthrough to `tpt-snapshot-lite`.

### Fixed

- Crate-level doc example called `.arg()` directly on `CliTest::cargo_bin(...)`'s `Result`; added the missing `.unwrap()` so the doctest compiles.
