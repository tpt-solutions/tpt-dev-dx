use std::fs;
use std::path::{Path, PathBuf};

/// Controls snapshot behaviour: read from `UPDATE_SNAPSHOTS` env var at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Assert that the stored snapshot matches the current value.
    Assert,
    /// Overwrite (or create) the snapshot file unconditionally.
    Update,
}

fn current_mode() -> Mode {
    if std::env::var("UPDATE_SNAPSHOTS")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        Mode::Update
    } else {
        Mode::Assert
    }
}

/// A named snapshot.
///
/// Snap files are stored under `<crate_root>/tests/snapshots/<name>.snap`,
/// where `<crate_root>` is determined by the `CARGO_MANIFEST_DIR` env var
/// at **test** compile time (passed in by the macro).
pub struct Snapshot {
    name: String,
    snap_dir: PathBuf,
}

impl Snapshot {
    /// Create a snapshot with the given name and snap directory (typically
    /// `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots")`).
    pub fn new(name: &str, snap_dir: &str) -> Self {
        Self {
            name: name.to_owned(),
            snap_dir: PathBuf::from(snap_dir),
        }
    }

    fn snap_path(&self) -> PathBuf {
        self.snap_dir.join(format!("{}.snap", self.name))
    }

    /// Assert that `value`'s `Display` output matches the stored snapshot.
    ///
    /// - If the snap file does not exist, it is created and the test **panics**
    ///   with a message asking you to re-run.
    /// - If `UPDATE_SNAPSHOTS=1`, the snap file is overwritten and the test passes.
    pub fn assert_display(&self, value: &dyn std::fmt::Display) {
        self.assert_str(&value.to_string());
    }

    /// Assert that `value`'s `Debug` output matches the stored snapshot.
    pub fn assert_debug(&self, value: &dyn std::fmt::Debug) {
        self.assert_str(&format!("{value:#?}"));
    }

    fn assert_str(&self, actual: &str) {
        let path = self.snap_path();
        match current_mode() {
            Mode::Update => {
                write_snap(&path, actual);
            }
            Mode::Assert => {
                if !path.exists() {
                    write_snap(&path, actual);
                    panic!(
                        "Snapshot '{}' did not exist — it has been created at {}.\n\
                         Re-run the test to verify.",
                        self.name,
                        path.display()
                    );
                }
                let stored = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Failed to read snapshot {}: {e}", path.display()));
                // Normalise line endings for cross-platform compatibility.
                let stored = stored.replace("\r\n", "\n");
                let actual = actual.replace("\r\n", "\n");
                if stored != actual {
                    panic!(
                        "Snapshot '{}' mismatch.\n\
                         --- stored ({}) ---\n{}\n\
                         --- actual ---\n{}\n\
                         Set UPDATE_SNAPSHOTS=1 to accept the new output.",
                        self.name,
                        path.display(),
                        stored,
                        actual
                    );
                }
            }
        }
    }
}

fn write_snap(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Failed to create snapshot dir {}: {e}", parent.display()));
    }
    fs::write(path, content)
        .unwrap_or_else(|e| panic!("Failed to write snapshot {}: {e}", path.display()));
}

// ── Macros ───────────────────────────────────────────────────────────────────

/// Assert that a value's `Display` output matches a named snapshot file.
///
/// Snap files are stored at `<crate_root>/tests/snapshots/<name>.snap`.
///
/// # Example
/// ```rust,ignore
/// assert_snapshot!("greeting", &format!("Hello, {}!", name));
/// ```
#[macro_export]
macro_rules! assert_snapshot {
    ($name:expr, $value:expr) => {{
        let snap = $crate::Snapshot::new(
            $name,
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots"),
        );
        snap.assert_display(&$value);
    }};
}

/// Assert that a value's `Debug` output matches a named snapshot file.
#[macro_export]
macro_rules! assert_snapshot_debug {
    ($name:expr, $value:expr) => {{
        let snap = $crate::Snapshot::new(
            $name,
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots"),
        );
        snap.assert_debug(&$value);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Serialise tests that touch UPDATE_SNAPSHOTS to avoid races.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn tmp_snap_dir() -> (tempfile_compat::Dir, String) {
        let d = tempfile_compat::Dir::new();
        let s = d.path().to_string_lossy().into_owned();
        (d, s)
    }

    // Minimal inline tempdir so we have zero external test deps.
    mod tempfile_compat {
        use std::{
            fs,
            path::PathBuf,
            sync::atomic::{AtomicU64, Ordering},
        };
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        pub struct Dir(PathBuf);
        impl Dir {
            pub fn new() -> Self {
                let n = COUNTER.fetch_add(1, Ordering::SeqCst);
                let p =
                    std::env::temp_dir().join(format!("snap-test-{}-{}", std::process::id(), n));
                fs::create_dir_all(&p).unwrap();
                Dir(p)
            }
            pub fn path(&self) -> &std::path::Path {
                &self.0
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn creates_snap_on_first_run() {
        let (dir, dir_s) = tmp_snap_dir();
        let snap = Snapshot::new("first_run", &dir_s);
        let result = std::panic::catch_unwind(|| snap.assert_display(&"hello"));
        assert!(result.is_err(), "should panic asking for re-run");
        assert!(dir.path().join("first_run.snap").exists());
    }

    #[test]
    fn matches_existing_snap() {
        let (dir, dir_s) = tmp_snap_dir();
        write_snap(&dir.path().join("existing.snap"), "hello");
        let snap = Snapshot::new("existing", &dir_s);
        snap.assert_display(&"hello"); // should not panic
    }

    #[test]
    fn panics_on_mismatch() {
        let _lock = ENV_LOCK.lock().unwrap();
        env::remove_var("UPDATE_SNAPSHOTS");
        let (dir, dir_s) = tmp_snap_dir();
        write_snap(&dir.path().join("mismatch.snap"), "old");
        let snap = Snapshot::new("mismatch", &dir_s);
        let result = std::panic::catch_unwind(|| snap.assert_display(&"new"));
        assert!(result.is_err(), "should panic on mismatch");
        let msg = result.unwrap_err();
        let msg_str = msg
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| msg.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            msg_str.contains("mismatch"),
            "panic message should contain 'mismatch': {msg_str}"
        );
    }

    #[test]
    fn update_mode_overwrites() {
        let _lock = ENV_LOCK.lock().unwrap();
        let (dir, dir_s) = tmp_snap_dir();
        write_snap(&dir.path().join("update.snap"), "old");
        env::set_var("UPDATE_SNAPSHOTS", "1");
        let snap = Snapshot::new("update", &dir_s);
        snap.assert_display(&"new");
        env::remove_var("UPDATE_SNAPSHOTS");
        let stored = fs::read_to_string(dir.path().join("update.snap")).unwrap();
        assert_eq!(stored, "new");
    }
}
