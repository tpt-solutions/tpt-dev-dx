# Changelog

## [0.1.0] — 2026-07-17

### Added

- Initial release.
- `#[tpt_fixture(scope = "suite" | "module" | "test", name = "...")]` attribute macro on init functions.
- Async init support (`block_on`; optional `tokio` feature).
- `(resource, teardown)` tuple return registers a synchronous teardown.
- `Arc<T>` injection into test functions (macro resolves each fixture parameter by name).
- `test`-scope teardown via `TestScopeGuard` (runs even on panic).
- `module`/`suite` singletons cached in a global `OnceLock`, tidied by `shutdown()`.
- `tpt-fixture-macros` proc-macro companion crate.

### Fixed

- `block_on` (with the `tokio` feature) no longer panics with "Cannot start a runtime from within a runtime" when a fixture is initialised from inside an already-running tokio runtime (e.g. a `#[tokio::test]` async test). It now detects that case via `Handle::try_current()` and drives the init future to completion on a dedicated thread with its own runtime.
