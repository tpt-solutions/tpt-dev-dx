# tpt-log-tap

[![crates.io](https://img.shields.io/crates/v/tpt-log-tap.svg)](https://crates.io/crates/tpt-log-tap)
[![docs.rs](https://docs.rs/tpt-log-tap/badge.svg)](https://docs.rs/tpt-log-tap)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

Per-test structured [`tracing`](https://docs.rs/tracing) event capture — assert
on **field values, not text**.

Matching rendered log strings is brittle: formatting, ANSI colour, and field
ordering all change the output. `tpt-log-tap` captures each event's level,
target, and structured fields into an in-memory buffer and lets you assert on
them directly. Install and uninstall are RAII: the subscriber lives only as long
as the returned `TapGuard`, so parallel tests stay isolated.

## Quick Start

```toml
[dev-dependencies]
tpt-log-tap = "0.1"
tracing = "0.1"
```

```rust
use tpt_log_tap::LogTap;
use tracing::Level;

#[test]
fn logs_login_event() {
    let tap = LogTap::new().install();

    tracing::info!(user_id = 42, action = "login", "user signed in");

    // Assert on structured fields — target "" means "any target".
    tap.assert_contains(Level::INFO, "", &[("user_id", "42"), ("action", "login")]);
    tap.assert_not_contains(Level::ERROR, "", &[]);
}
```

## Filtering

```rust
use tpt_log_tap::LogTap;
use tracing::Level;

// Only capture WARN and above, from your crate's target.
let tap = LogTap::new()
    .level(Level::WARN)
    .target("my_crate")
    .install();
```

## Custom assertions

```rust
use tpt_log_tap::LogTap;

let tap = LogTap::new().install();
tracing::info!(count = 3, "processed");

for event in tap.events() {
    if let Some(count) = event.field("count") {
        assert_eq!(count, "3");
    }
}
```

## Deferred expectations

Register an expectation up front; it's verified automatically when the guard is
dropped at the end of the test:

```rust
use tpt_log_tap::LogTap;
use tracing::Level;

let mut tap = LogTap::new().install();
tap.expect_contains(Level::INFO, "", &[("done", "true")]);

// ... run code under test ...
tracing::info!(done = true, "finished");

// `tap` dropped here → expectation checked (panics if unmet).
```

## Isolation & async tests

`install()` uses `tracing::subscriber::set_default`, which scopes the subscriber
to the **current thread**. Under `cargo test`'s default thread-per-test
parallelism each test captures only its own events.

For `async` tests, keep the traced work on the test's own thread (e.g. a
current-thread runtime such as `#[tokio::test(flavor = "current_thread")]`).
Events emitted on worker threads spawned elsewhere won't inherit the thread-local
subscriber.

## License

MIT OR Apache-2.0
