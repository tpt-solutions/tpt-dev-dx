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

## Phase 5 — out-faker-rs
Renamed from `tpt-faker-rs` (see commit `38142c2`) and marked `publish = false` — deemed too redundant with the established `fake` crate to publish. The `out-` prefix flags any crate held back from crates.io.
- [x] `crates/out-faker-rs-derive/Cargo.toml` (proc-macro crate)
- [x] `#[derive(Fake)]` macro with `#[fake(...)]` field attrs
- [x] `crates/out-faker-rs/Cargo.toml`
- [x] `Fake` trait definition
- [x] Generators: `name`, `first_name`, `last_name`, `email`, `username`
- [x] Generators: `url`, `ipv4`, `ipv6`, `uuid`
- [x] Generators: `luhn_card` (Luhn algorithm)
- [x] Generators: `iso_date`, `iso_datetime`
- [x] Generators: `word`, `sentence`, `paragraph`
- [x] Generators: `range = "lo..=hi"` for numerics
- [x] `serde` optional feature
- [x] Unit tests for each generator
- [x] `crates/out-faker-rs/README.md`
- [x] `crates/out-faker-rs/CHANGELOG.md`

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
- [ ] `cargo publish --dry-run` — tpt-cargo-scrub
- [ ] `cargo publish --dry-run` — tpt-port-scout
- [ ] `cargo publish --dry-run` — tpt-log-tap
- [ ] `cargo publish --dry-run` — tpt-cli-snap
- [ ] `cargo publish --dry-run` — tpt-http-stub
- [ ] `cargo publish --dry-run` — tpt-fixture-macros
- [ ] `cargo publish --dry-run` — tpt-fixture
- [ ] `cargo doc --workspace --no-deps` — zero warnings
- [ ] Final `cargo test --workspace` green

## Phase 8 — tpt-port-scout
RAII TCP/UDP port reservation — holds sockets open until the server binds, eliminating TOCTOU races in parallel integration tests.
- [x] `crates/tpt-port-scout/Cargo.toml` (zero deps, pure `std::net`)
- [x] `PortSet::reserve(n)` — bind N listeners on `127.0.0.1:0`, return guard
- [x] `PortSet::reserve_one()` convenience constructor
- [x] `PortSet::addr(i)` / `PortSet::addrs()` — expose `SocketAddr` per port
- [x] Hand-off: `PortSet::take_listener()` / `into_std_listener()` for server reuse
- [x] `Drop` impl releases all reserved listeners
- [x] Unit tests (parallel port allocation, no collisions) — 12 passing
- [x] `crates/tpt-port-scout/README.md`
- [x] `crates/tpt-port-scout/CHANGELOG.md`

## Phase 9 — tpt-log-tap
Per-test structured tracing event capture — assert on field values, not text. RAII install/uninstall of a per-test subscriber layer.
- [x] `crates/tpt-log-tap/Cargo.toml` (dep: `tracing`, `tracing-subscriber`)
- [x] `LogTap::new()` builder (filter by level, target)
- [x] `LogTap::install()` → `TapGuard` (installs per-thread subscriber layer)
- [x] Internal event buffer storing structured fields per event
- [x] `TapGuard::assert_contains(level, target, fields)` — field-level match
- [x] `TapGuard::assert_not_contains(...)` counterpart
- [x] `TapGuard::events()` — raw access for custom assertions
- [x] `Drop` impl uninstalls the layer and checks any pending expectations
- [x] Tests confirming isolation across parallel async tests — 9 passing
- [x] `crates/tpt-log-tap/README.md`
- [x] `crates/tpt-log-tap/CHANGELOG.md`

## Phase 10 — tpt-cli-snap
CLI process testing with integrated snapshot assertions — bridges `assert_cmd` and `tpt-snapshot-lite` for readable, maintainable binary output tests.
- [x] `crates/tpt-cli-snap/Cargo.toml` (deps: `assert_cmd`, `tpt-snapshot-lite`)
- [x] `CliTest::cargo_bin(name)` / `CliTest::command(cmd)` constructors
- [x] `.arg()`, `.args()`, `.env()`, `.stdin()` builder methods
- [x] `.assert_snapshot(name)` — runs process, snapshots stdout via `tpt-snapshot-lite`
- [x] `.assert_snapshot_stderr(name)` — stderr variant
- [x] `.assert_snapshot_both(name)` — combined stdout+stderr snapshot
- [x] Exit code assertion chaining (`assert_success` / `assert_code` / `assert_failure`)
- [x] `UPDATE_SNAPSHOTS=1` passthrough from `tpt-snapshot-lite`
- [x] Integration tests against `crates/cli-fixture` binary in the workspace — 6 passing
- [x] `crates/tpt-cli-snap/README.md`
- [x] `crates/tpt-cli-snap/CHANGELOG.md`

## Phase 11 — tpt-http-stub
Lightweight in-process HTTP stub server — minimal deps, no async runtime required for simple request/response stubs.
- [x] `crates/tpt-http-stub/Cargo.toml`
- [x] `HttpStub::new()` — binds to a random free port (uses tpt-port-scout internally)
- [x] `.on(method, path).respond(status, body)` stub registration
- [x] `.on(...).respond_json(status, value)` convenience for JSON bodies
- [x] Request capture: `.last_request()`, `.requests()` for assertion
- [x] `.assert_called_once()` / `.assert_called_n(n)` call-count assertions
- [x] `Drop` impl verifies all expectations and shuts down the server
- [x] `base_url()` method returning a `String` for client configuration
- [x] Tests: parallel stubs don't interfere, missing stub returns 500 — 9 passing
- [x] `crates/tpt-http-stub/README.md`
- [x] `crates/tpt-http-stub/CHANGELOG.md`

## Phase 12 — tpt-fixture
Session- and module-scoped test fixtures with async init and async teardown — fills the `beforeAll`/`afterAll` gap (rstest #119).
- [x] `crates/tpt-fixture/Cargo.toml` (proc-macro companion crate + library)
- [x] `crates/tpt-fixture-macros/Cargo.toml` (proc-macro crate)
- [x] `#[tpt_fixture(scope = "suite" | "module" | "test")]` attribute macro
- [x] Async init function support (returns `(Resource, impl AsyncDrop)`)
- [x] Suite-scope: shared across all tests in a binary, cleaned up at process exit
- [x] Module-scope: shared across tests in one module
- [x] Thread-safe sharing via `Arc<T>` injection into test functions
- [ ] nextest compatibility — document process-per-test implications
- [x] Async teardown workaround — `block_on` (tokio feature) now detects an already-running runtime via `Handle::try_current()` and drives the future on a dedicated thread/runtime instead of nesting, fixing the "Cannot start a runtime from within a runtime" panic when a fixture is used from `#[tokio::test]`
- [x] Tests: fixture initialised once, shared reference correct, teardown fires — all 7 integration tests pass (including `async_fixture_test` and `verify_teardown_fires_on_shutdown`)
- [x] `crates/tpt-fixture/README.md`
- [x] `crates/tpt-fixture/CHANGELOG.md`
