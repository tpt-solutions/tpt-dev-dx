# tpt-snapshot-lite

[![crates.io](https://img.shields.io/crates/v/tpt-snapshot-lite.svg)](https://crates.io/crates/tpt-snapshot-lite)
[![docs.rs](https://docs.rs/tpt-snapshot-lite/badge.svg)](https://docs.rs/tpt-snapshot-lite)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

Lightweight, **zero-dependency** snapshot testing for Rust.

Compares `Display` or `Debug` output against `.snap` files stored in `tests/snapshots/`. On the first run a snap file is created and the test fails asking you to re-run. Set `UPDATE_SNAPSHOTS=1` to accept new output.

## Quick Start

```toml
[dev-dependencies]
tpt-snapshot-lite = "0.1"
```

```rust
use tpt_snapshot_lite::assert_snapshot;

#[test]
fn test_render_output() {
    let output = render_my_thing();
    assert_snapshot!("render_output", &output);
}

#[test]
fn test_debug_repr() {
    use tpt_snapshot_lite::assert_snapshot_debug;
    assert_snapshot_debug!("my_struct_debug", &my_value);
}
```

Snap files live at `tests/snapshots/<name>.snap` relative to your crate root.

### Updating snapshots

```sh
UPDATE_SNAPSHOTS=1 cargo test
```

## Zero dependencies

`tpt-snapshot-lite` has **no runtime dependencies** — pure `std` only.

## License

MIT OR Apache-2.0
