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

## Phase 8 — tpt-port-scout
RAII TCP/UDP port reservation — holds sockets open until the server binds, eliminating TOCTOU races in parallel integration tests.
- [ ] `crates/tpt-port-scout/Cargo.toml` (zero deps, pure `std::net`)
- [ ] `PortSet::reserve(n)` — bind N listeners on `127.0.0.1:0`, return guard
- [ ] `PortSet::reserve_one()` convenience constructor
- [ ] `PortSet::addr(i)` / `PortSet::addrs()` — expose `SocketAddr` per port
- [ ] Hand-off: convert `TcpListener` to `std::os::unix::io::FromRawFd` for server reuse (or document the rebind-before-drop pattern)
- [ ] `Drop` impl releases all reserved listeners
- [ ] Unit tests (parallel port allocation, no collisions)
- [ ] `crates/tpt-port-scout/README.md`
- [ ] `crates/tpt-port-scout/CHANGELOG.md`

## Phase 9 — tpt-log-tap
Per-test structured tracing event capture — assert on field values, not text. RAII install/uninstall of a per-test subscriber layer.
- [ ] `crates/tpt-log-tap/Cargo.toml` (dep: `tracing`, `tracing-subscriber`)
- [ ] `LogTap::new()` builder (filter by level, target)
- [ ] `LogTap::install()` → `TapGuard` (installs per-thread subscriber layer)
- [ ] Internal event buffer storing structured fields per event
- [ ] `TapGuard::assert_contains(level, target, fields)` — field-level match
- [ ] `TapGuard::assert_not_contains(...)` counterpart
- [ ] `TapGuard::events()` — raw access for custom assertions
- [ ] `Drop` impl uninstalls the layer and checks any pending expectations
- [ ] Tests confirming isolation across parallel async tests
- [ ] `crates/tpt-log-tap/README.md`
- [ ] `crates/tpt-log-tap/CHANGELOG.md`

## Phase 10 — tpt-cli-snap
CLI process testing with integrated snapshot assertions — bridges `assert_cmd` and `tpt-snapshot-lite` for readable, maintainable binary output tests.
- [ ] `crates/tpt-cli-snap/Cargo.toml` (deps: `assert_cmd`, `tpt-snapshot-lite`)
- [ ] `CliTest::cargo_bin(name)` / `CliTest::command(cmd)` constructors
- [ ] `.arg()`, `.args()`, `.env()`, `.stdin()` builder methods
- [ ] `.assert_snapshot(name)` — runs process, snapshots stdout via `tpt-snapshot-lite`
- [ ] `.assert_snapshot_stderr(name)` — stderr variant
- [ ] `.assert_snapshot_both(name)` — combined stdout+stderr snapshot
- [ ] Exit code assertion chaining
- [ ] `UPDATE_SNAPSHOTS=1` passthrough from `tpt-snapshot-lite`
- [ ] Integration tests against a fixture binary in the workspace
- [ ] `crates/tpt-cli-snap/README.md`
- [ ] `crates/tpt-cli-snap/CHANGELOG.md`

## Phase 11 — tpt-http-stub
Lightweight in-process HTTP stub server — minimal deps, no async runtime required for simple request/response stubs.
- [ ] `crates/tpt-http-stub/Cargo.toml`
- [ ] `HttpStub::new()` — binds to a random free port (uses tpt-port-scout internally)
- [ ] `.on(method, path).respond(status, body)` stub registration
- [ ] `.on(...).respond_json(status, value)` convenience for JSON bodies
- [ ] Request capture: `.last_request()`, `.requests()` for assertion
- [ ] `.assert_called_once()` / `.assert_called_n(n)` call-count assertions
- [ ] `Drop` impl verifies all expectations and shuts down the server
- [ ] `base_url()` method returning a `String` for client configuration
- [ ] Tests: parallel stubs don't interfere, missing stub returns 500
- [ ] `crates/tpt-http-stub/README.md`
- [ ] `crates/tpt-http-stub/CHANGELOG.md`

## Phase 12 — tpt-fixture
Session- and module-scoped test fixtures with async init and async teardown — fills the `beforeAll`/`afterAll` gap (rstest #119).
- [ ] `crates/tpt-fixture/Cargo.toml` (proc-macro companion crate + library)
- [ ] `crates/tpt-fixture-macros/Cargo.toml` (proc-macro crate)
- [ ] `#[tpt_fixture(scope = "suite" | "module" | "test")]` attribute macro
- [ ] Async init function support (returns `(Resource, impl AsyncDrop)`)
- [ ] Suite-scope: shared across all tests in a binary, cleaned up at process exit
- [ ] Module-scope: shared across tests in one module
- [ ] Thread-safe sharing via `Arc<T>` injection into test functions
- [ ] nextest compatibility — document process-per-test implications
- [ ] Async teardown workaround (spawn a teardown runtime in `Drop` if needed)
- [ ] Tests: fixture initialised once, shared reference correct, teardown fires
- [ ] `crates/tpt-fixture/README.md`
- [ ] `crates/tpt-fixture/CHANGELOG.md`
