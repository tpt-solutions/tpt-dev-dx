# tpt-cli-snap

[![crates.io](https://img.shields.io/crates/v/tpt-cli-snap.svg)](https://crates.io/crates/tpt-cli-snap)
[![docs.rs](https://docs.rs/tpt-cli-snap/badge.svg)](https://docs.rs/tpt-cli-snap)
[![CI](https://github.com/tpt-solutions/tpt-dev-dx/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-dev-dx/actions)

CLI process testing with **integrated snapshot assertions**.

`tpt-cli-snap` bridges [`assert_cmd`] (run a binary, inspect output) and
[`tpt-snapshot-lite`] (compare text against `.snap` files). You get readable,
self-documenting binary-output tests without hand-writing golden-file boilerplate.

## Quick Start

```toml
[dev-dependencies]
tpt-cli-snap = "0.1"
```

```rust,ignore
use tpt_cli_snap::CliTest;

#[test]
fn renders_status() {
    let outcome = CliTest::cargo_bin("my-binary")
        .arg("--format=json")
        .arg("status")
        .assert_snapshot("status_json");
    outcome.assert_success();
}
```

Snapshots live at `<your_crate>/tests/snapshots/<name>.snap`. On the first run a
file is created and the test fails asking you to re-run. Set `UPDATE_SNAPSHOTS=1`
to accept new output — the flag passes straight through to `tpt-snapshot-lite`.

## Builder API

```rust,ignore
use tpt_cli_snap::CliTest;

let outcome = CliTest::cargo_bin("wm")
    .arg("deploy")
    .args(["--env", "prod"])
    .env("TOKEN", "secret")
    .stdin("piped config")
    .assert_snapshot("deploy_prod");

outcome.assert_success();   // exit code 0
outcome.assert_code(0);     // or a specific code
```

## Snapshot variants

| Method | Snapshots |
|--------|-----------|
| `assert_snapshot(name)` | stdout |
| `assert_snapshot_stderr(name)` | stderr |
| `assert_snapshot_both(name)` | stdout + stderr (separated by a `---- stderr ----` marker) |

## Custom snapshot directory

The default directory is `<calling crate>/tests/snapshots`, via the
[`cli_snap_dir!()`](crate::cli_snap_dir) macro. Override it with
`CliTest::with_snap_dir(...)` if your fixtures live elsewhere.

## License

MIT OR Apache-2.0

[`assert_cmd`]: https://docs.rs/assert_cmd
[`tpt-snapshot-lite`]: https://docs.rs/tpt-snapshot-lite
