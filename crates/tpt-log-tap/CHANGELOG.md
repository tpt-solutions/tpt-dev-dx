# Changelog

## [0.1.0] — 2026-07-17

### Added

- Initial release.
- `LogTap::new()` builder with `.level()` and `.target()` filters.
- `LogTap::install()` → `TapGuard` installing a per-thread capturing subscriber.
- Structured field capture per event (`CapturedEvent` with level, target, message, fields).
- `TapGuard::assert_contains` / `assert_not_contains` — field-level assertions.
- `TapGuard::contains` / `events` / `len` / `is_empty` — programmatic access.
- `TapGuard::expect_contains` — deferred expectations checked on drop.
- `Drop` uninstalls the subscriber and verifies pending expectations.
