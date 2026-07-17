# tpt-fixture

[![crates.io](https://img.shields.io/crates/v/tpt-fixture.svg)](https://crates.io/crates/tpt-fixture)
[![docs.rs](https://docs.rs/tpt-fixture/badge.svg)](https://docs.rs/tpt-fixture)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

Session- and module-scoped test fixtures with **async init** and **teardown** — the
`beforeAll` / `afterAll` gap that [`rstest` #119] leaves open.

A fixture init function runs **once** per scope and shares its resource — as a
thread-safe `Arc<T>` — across every test in that scope. Teardown runs when the
scope ends.

## Quick Start

```toml
[dev-dependencies]
tpt-fixture = "0.1"
```

```rust,ignore
use std::sync::Arc;
use tpt_fixture::tpt_fixture;

// Initialised once for the whole suite; torn down at process exit.
#[tpt_fixture(scope = "suite")]
async fn db() -> Database {
    Database::connect().await
}

#[tpt_fixture]
#[tokio::test]
async fn reads_rows(db: Arc<Database>) {
    assert!(db.row_count() > 0);
}
```

## Scopes

| Scope    | Initialised | Shared across        | Teardown            |
|----------|-------------|----------------------|---------------------|
| `test`   | every test  | — (fresh per call)   | end of that test    |
| `module` | once        | the test binary      | `shutdown()`        |
| `suite`  | once        | the whole test binary| `shutdown()`        |

`module` and `suite` are both process-lifetime singletons (Rust has no runtime
module identity), initialised exactly once via a global `OnceLock`. They differ
only in *intent*.

## Teardown

Return a `(resource, teardown)` tuple from your init fn to register a teardown
closure:

```rust,ignore
#[tpt_fixture(scope = "suite")]
fn temp_dir() -> (PathBuf, Box<dyn FnOnce() + Send>) {
    let dir = make_temp_dir();
    let path = dir.path().to_path_buf();
    (path, Box::new(move || std::fs::remove_dir_all(path).ok()))
}
```

- `test`-scope teardowns fire automatically at the end of each test (even on
  panic), via a `TestScopeGuard` inserted by the macro.
- `module`/`suite`-scope teardowns fire when you call [`shutdown`], or via the
  per-process model of `cargo-nextest`.

## Async support

Async init is awaited with `block_on`. By default a tiny single-threaded executor
is used (fine for futures that resolve without external wake-ups). Enable the
`tokio` feature to drive real async I/O / timers on a current-thread tokio
runtime:

```toml
tpt-fixture = { version = "0.1", features = ["tokio"] }
```

## nextest compatibility

`cargo-nextest` runs each test in its own process, so per-process singletons
behave like per-test fixtures. Call `tpt_fixture::shutdown()` from a dedicated
teardown test if you need suite teardown to run.

## License

MIT OR Apache-2.0

[`rstest` #119]: https://github.com/la10736/rstest/issues/119
