//! CLI process testing with integrated snapshot assertions.
//!
//! `tpt-cli-snap` bridges [`assert_cmd`] (which runs a binary and lets you
//! inspect its output) and [`tpt_snapshot_lite`] (which compares output against
//! `.snap` files). The result is readable, self-documenting binary-output tests:
//!
//! ```no_run
//! use tpt_cli_snap::CliTest;
//!
//! let outcome = CliTest::cargo_bin("my-binary")
//!     .unwrap()
//!     .arg("--format=json")
//!     .arg("status")
//!     .assert_snapshot("status_json");
//! outcome.assert_success();
//! ```
//!
//! Snapshots are stored under `<crate_root>/tests/snapshots/` by default; use the
//! [`cli_snap_dir!`](crate::cli_snap_dir) macro to point at the *calling* crate's
//! manifest directory, or pass an explicit directory with
//! [`CliTest::with_snap_dir`].
//!
//! Set `UPDATE_SNAPSHOTS=1` to (re)create snapshot files — the flag is passed
//! straight through to `tpt-snapshot-lite`.

use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;

use assert_cmd::cargo::CargoError;
use assert_cmd::Command;
use tpt_snapshot_lite::Snapshot;

/// Default snapshot directory for the *calling* crate, evaluated at the call
/// site's compile time: `<CARGO_MANIFEST_DIR>/tests/snapshots`.
///
/// This is the same directory `tpt-snapshot-lite` uses by default, so the two
/// crates share a snapshot layout.
///
/// # Examples
///
/// ```ignore
/// use tpt_cli_snap::{CliTest, cli_snap_dir};
///
/// let outcome = CliTest::cargo_bin("wm")
///     .assert_snapshot_with_dir("ls", cli_snap_dir!());
/// ```
#[macro_export]
macro_rules! cli_snap_dir {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots")
    };
}

/// The result of running a CLI command, with snapshot assertions already applied.
///
/// Returned by the `assert_snapshot*` family; use the chaining methods to also
/// assert exit code and signal behaviour.
pub struct CliOutcome {
    output: Output,
    cmd_name: String,
}

impl CliOutcome {
    /// Assert the process exited successfully (status `0`).
    ///
    /// # Panics
    ///
    /// Panics if the exit status is non-zero, including a dump of captured
    /// stdout/stderr.
    #[track_caller]
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.output.status.success(),
            "command `{}` did not exit successfully.\nstdout: {}\nstderr: {}",
            self.cmd_name,
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr),
        );
        self
    }

    /// Assert the process exited with the given code.
    #[track_caller]
    pub fn assert_code(&self, code: i32) -> &Self {
        let actual = self.output.status.code().unwrap_or(-1);
        assert!(
            actual == code,
            "command `{}` exited with {actual}, expected {code}",
            self.cmd_name,
        );
        self
    }

    /// Assert the process failed (non-zero exit).
    #[track_caller]
    pub fn assert_failure(&self) -> &Self {
        assert!(
            !self.output.status.success(),
            "command `{}` unexpectedly succeeded",
            self.cmd_name,
        );
        self
    }

    /// The raw captured process output.
    pub fn output(&self) -> &Output {
        &self.output
    }

    /// The captured stdout bytes.
    pub fn stdout(&self) -> &[u8] {
        &self.output.stdout
    }

    /// The captured stderr bytes.
    pub fn stderr(&self) -> &[u8] {
        &self.output.stderr
    }

    /// The exit status code (or `-1` if terminated by a signal).
    pub fn code(&self) -> i32 {
        self.output.status.code().unwrap_or(-1)
    }
}

/// Builder for a CLI test: a binary plus the args/env/stdin to run it with.
pub struct CliTest {
    command: Command,
    name: String,
    snap_dir: PathBuf,
}

impl CliTest {
    /// Run the binary produced by `cargo test` for the named crate (the common
    /// case — test your own binary or a workspace fixture binary).
    pub fn cargo_bin(name: &str) -> Result<Self, CargoError> {
        let command = Command::cargo_bin(name)?;
        Ok(Self {
            command,
            name: name.to_string(),
            snap_dir: PathBuf::from(cli_snap_dir!()),
        })
    }

    /// Wrap an arbitrary [`Command`] (e.g. a system binary or a pre-built path).
    pub fn command(command: Command) -> Self {
        let name = command.get_program().to_string_lossy().into_owned();
        Self {
            command,
            name,
            snap_dir: PathBuf::from(cli_snap_dir!()),
        }
    }

    /// Override the snapshot directory used by the `assert_snapshot*` methods.
    pub fn with_snap_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.snap_dir = dir.into();
        self
    }

    /// Append a single argument.
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.command.arg(arg);
        self
    }

    /// Append multiple arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// Set an environment variable for the child process.
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.command.env(key, value);
        self
    }

    /// Remove an environment variable from the child process.
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.command.env_remove(key);
        self
    }

    /// Feed `input` to the child process's stdin.
    pub fn stdin(mut self, input: impl AsRef<[u8]>) -> Self {
        self.command.write_stdin(input.as_ref().to_vec());
        self
    }

    /// Run the process and snapshot its **stdout** against `<name>.snap`.
    ///
    /// The snapshot flag (`UPDATE_SNAPSHOTS`) is passed straight through from the
    /// environment to `tpt-snapshot-lite`.
    #[track_caller]
    pub fn assert_snapshot(mut self, name: &str) -> CliOutcome {
        let output = self
            .command
            .output()
            .unwrap_or_else(|e| panic!("failed to run `{}`: {e}", self.name));
        let snap = Snapshot::new(name, self.snap_dir.to_str().expect("valid snap dir"));
        snap.assert_display(&String::from_utf8_lossy(&output.stdout));
        CliOutcome {
            output,
            cmd_name: self.name,
        }
    }

    /// Run the process and snapshot its **stderr** against `<name>.snap`.
    #[track_caller]
    pub fn assert_snapshot_stderr(mut self, name: &str) -> CliOutcome {
        let output = self
            .command
            .output()
            .unwrap_or_else(|e| panic!("failed to run `{}`: {e}", self.name));
        let snap = Snapshot::new(name, self.snap_dir.to_str().expect("valid snap dir"));
        snap.assert_display(&String::from_utf8_lossy(&output.stderr));
        CliOutcome {
            output,
            cmd_name: self.name,
        }
    }

    /// Run the process and snapshot the **combined stdout + stderr** against
    /// `<name>.snap`. A `---- stderr ----` separator distinguishes the two streams.
    #[track_caller]
    pub fn assert_snapshot_both(mut self, name: &str) -> CliOutcome {
        let output = self
            .command
            .output()
            .unwrap_or_else(|e| panic!("failed to run `{}`: {e}", self.name));
        let mut combined = String::new();
        combined.push_str("---- stdout ----\n");
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
        combined.push_str("---- stderr ----\n");
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        let snap = Snapshot::new(name, self.snap_dir.to_str().expect("valid snap dir"));
        snap.assert_display(&combined);
        CliOutcome {
            output,
            cmd_name: self.name,
        }
    }
}
