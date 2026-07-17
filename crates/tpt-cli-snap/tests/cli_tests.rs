//! Integration tests for `tpt-cli-snap`.
//!
//! The fixture binary `cli-fixture` lives in a separate workspace crate (it
//! can't reuse this crate's `CARGO_BIN_EXE_*` env var), so we resolve its built
//! binary via [`fixture_cmd`] and wrap it with [`CliTest::command`].

use assert_cmd::Command;
use tpt_cli_snap::CliTest;

/// Locate the workspace fixture binary `cli-fixture` without relying on
/// `CARGO_BIN_EXE_*` (which cargo only sets for the crate that builds a binary).
///
/// The fixture is built as a separate workspace crate; its binary lands next to
/// this integration-test executable in `target/<profile>/cli-fixture[.exe]`.
fn fixture_cmd() -> Command {
    let exe = std::env::current_exe().expect("current test exe path");
    let bin_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile> dir")
        .to_path_buf();
    let mut path = bin_dir.join("cli-fixture");
    if !path.exists() {
        path.set_extension(std::env::consts::EXE_EXTENSION);
    }
    assert!(
        path.exists(),
        "fixture binary not found at {}",
        path.display()
    );
    Command::new(path)
}

#[test]
fn runs_fixture_and_snapshots_stdout() {
    // First run creates the snapshot; subsequent runs assert.
    let outcome = CliTest::command(fixture_cmd())
        .arg("hello")
        .assert_snapshot("cli_fixture_hello");
    outcome.assert_success();
}

#[test]
fn snapshots_stderr_variant() {
    let outcome = CliTest::command(fixture_cmd())
        .arg("err")
        .assert_snapshot_stderr("cli_fixture_err");
    outcome.assert_failure();
}

#[test]
fn snapshots_both_streams() {
    let outcome = CliTest::command(fixture_cmd())
        .arg("both")
        .assert_snapshot_both("cli_fixture_both");
    outcome.assert_success();
}

#[test]
fn exit_code_chaining() {
    let outcome = CliTest::command(fixture_cmd())
        .arg("code")
        .arg("3")
        .assert_snapshot("cli_fixture_code");
    outcome.assert_code(3);
}

#[test]
fn env_and_stdin_passthrough() {
    let outcome = CliTest::command(fixture_cmd())
        .arg("env")
        .arg("CLI_FIXTURE_VAR")
        .env("CLI_FIXTURE_VAR", "world")
        .stdin("piped-input")
        .assert_snapshot("cli_fixture_env_stdin");
    outcome.assert_success();
    // The fixture echoes the env var value; verify via raw output.
    let out = String::from_utf8_lossy(outcome.stdout());
    assert!(out.contains("world"), "expected env var in output: {out}");
    assert!(
        out.contains("stdin: piped-input"),
        "expected stdin echo: {out}"
    );
}

#[test]
fn explicit_snap_dir_override() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let snap_dir = format!("{dir}/tests/snapshots");
    let outcome = CliTest::command(fixture_cmd())
        .with_snap_dir(snap_dir)
        .arg("hello")
        .assert_snapshot("cli_fixture_hello_override");
    outcome.assert_success();
}
