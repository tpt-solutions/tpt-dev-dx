# tpt-env-mocker

[![crates.io](https://img.shields.io/crates/v/tpt-env-mocker.svg)](https://crates.io/crates/tpt-env-mocker)
[![docs.rs](https://docs.rs/tpt-env-mocker/badge.svg)](https://docs.rs/tpt-env-mocker)
[![CI](https://github.com/thirtyfiveparts/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/thirtyfiveparts/tpt-dev-dx/actions)

Safe, isolated environment variable mocking for async tests.

`std::env::set_var` is global. If two async tests run in parallel and change the same env var, they race and fail. `tpt-env-mocker` solves this with a **global `Mutex`** that serialises all tests that touch the environment, and a RAII guard that restores original values on drop.

## Quick Start

```toml
[dev-dependencies]
tpt-env-mocker = "0.1"
```

### Builder API

```rust
use tpt_env_mocker::MockEnv;

#[test]
fn test_reads_database_url() {
    let _guard = MockEnv::new()
        .set("DATABASE_URL", "postgres://localhost/test")
        .set("LOG_LEVEL", "debug")
        .lock();

    assert_eq!(std::env::var("DATABASE_URL").unwrap(), "postgres://localhost/test");
    // `_guard` dropped → env vars restored automatically
}
```

### Attribute macro

```rust
use tpt_env_mocker::tpt_env;

#[test]
#[tpt_env(DATABASE_URL = "postgres://localhost/test", LOG_LEVEL = "debug")]
fn test_with_macro() {
    assert_eq!(std::env::var("DATABASE_URL").unwrap(), "postgres://localhost/test");
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `macros` | ✓ | Enable the `#[tpt_env(...)]` attribute macro |

## License

MIT OR Apache-2.0
