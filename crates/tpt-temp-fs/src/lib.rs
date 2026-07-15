use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// An ephemeral temporary directory that is automatically deleted when dropped.
///
/// # Example
/// ```rust
/// # use tpt_temp_fs::TempDir;
/// let dir = TempDir::new().unwrap();
/// dir.write_file("hello.txt", "world").unwrap();
/// assert!(dir.child("hello.txt").exists());
/// // deleted automatically when `dir` is dropped
/// ```
pub struct TempDir {
    path: PathBuf,
    persistent: bool,
}

impl TempDir {
    /// Create a new temporary directory with a random name inside `std::env::temp_dir()`.
    pub fn new() -> io::Result<Self> {
        Self::with_prefix("tpt-")
    }

    /// Create a new temporary directory with a specific prefix.
    pub fn with_prefix(prefix: &str) -> io::Result<Self> {
        let base = std::env::temp_dir();
        // Generate a unique suffix using a mix of time + random-ish data.
        let suffix = unique_suffix();
        let path = base.join(format!("{prefix}{suffix}"));
        fs::create_dir_all(&path)?;
        Ok(Self {
            path,
            persistent: false,
        })
    }

    /// The root path of this temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Resolve a relative path inside this directory without creating anything.
    pub fn child(&self, rel: impl AsRef<Path>) -> PathBuf {
        self.path.join(rel)
    }

    /// Write `content` to `rel` path, creating parent directories as needed.
    pub fn write_file(&self, rel: impl AsRef<Path>, content: impl AsRef<[u8]>) -> io::Result<()> {
        let dest = self.path.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, content)
    }

    /// Create a subdirectory (and any missing parents) inside this temporary directory.
    pub fn create_dir(&self, rel: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir_all(self.path.join(rel))
    }

    /// Keep the directory on disk after this value is dropped.
    ///
    /// Returns the path so callers can reference it after the guard is gone.
    pub fn into_persistent(mut self) -> PathBuf {
        self.persistent = true;
        self.path.clone()
    }

    /// Scaffold a directory tree from a JSON or YAML string.
    ///
    /// The string must be a mapping where:
    /// - keys are relative paths
    /// - values are file contents (strings)
    /// - keys ending in `/` create empty directories
    ///
    /// Tries JSON first, falls back to YAML.
    ///
    /// # Example (YAML)
    /// ```ignore
    /// dir.scaffold_from_str("
    ///   config.json: '{\"key\":\"value\"}'
    ///   subdir/:
    ///   subdir/file.txt: hello
    /// ").unwrap();
    /// ```
    #[cfg(feature = "scaffold")]
    pub fn scaffold_from_str(&self, s: &str) -> io::Result<()> {
        let map = parse_scaffold(s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        for (key, value) in map {
            if key.ends_with('/') {
                self.create_dir(&key[..key.len() - 1])?;
            } else {
                self.write_file(&key, value.as_bytes())?;
            }
        }
        Ok(())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.persistent && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    // Mix in the thread id as a second source of uniqueness.
    let tid = format!("{:?}", std::thread::current().id());
    let tid_hash: u64 = tid
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("{nanos:08x}{tid_hash:08x}")
}

#[cfg(feature = "scaffold")]
fn parse_scaffold(s: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    // Try JSON first.
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(s) {
        return json_value_to_map(val);
    }
    // Fall back to YAML.
    let val: serde_yaml::Value =
        serde_yaml::from_str(s).map_err(|e| format!("YAML parse error: {e}"))?;
    yaml_value_to_map(val)
}

#[cfg(feature = "scaffold")]
fn json_value_to_map(
    val: serde_json::Value,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    match val {
        serde_json::Value::Object(m) => m
            .into_iter()
            .map(|(k, v)| {
                let content = match v {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Null => String::new(),
                    other => other.to_string(),
                };
                Ok((k, content))
            })
            .collect(),
        _ => Err("scaffold JSON must be an object mapping paths to contents".into()),
    }
}

#[cfg(feature = "scaffold")]
fn yaml_value_to_map(
    val: serde_yaml::Value,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    match val {
        serde_yaml::Value::Mapping(m) => m
            .into_iter()
            .map(|(k, v)| {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    other => format!("{other:?}"),
                };
                let content = match v {
                    serde_yaml::Value::String(s) => s,
                    serde_yaml::Value::Null => String::new(),
                    other => format!("{other:?}"),
                };
                Ok((key, content))
            })
            .collect(),
        _ => Err("scaffold YAML must be a mapping of paths to contents".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_cleans_up() {
        let path = {
            let dir = TempDir::new().unwrap();
            let p = dir.path().to_path_buf();
            assert!(p.exists());
            p
        };
        assert!(!path.exists(), "directory should be deleted on drop");
    }

    #[test]
    fn write_and_read_file() {
        let dir = TempDir::new().unwrap();
        dir.write_file("hello.txt", "world").unwrap();
        assert_eq!(fs::read_to_string(dir.child("hello.txt")).unwrap(), "world");
    }

    #[test]
    fn nested_write_creates_parents() {
        let dir = TempDir::new().unwrap();
        dir.write_file("a/b/c.txt", "deep").unwrap();
        assert!(dir.child("a/b/c.txt").exists());
    }

    #[test]
    fn into_persistent_survives_drop() {
        let path = TempDir::new().unwrap().into_persistent();
        assert!(path.exists());
        fs::remove_dir_all(&path).unwrap();
    }

    #[test]
    #[cfg(feature = "scaffold")]
    fn scaffold_from_yaml() {
        let dir = TempDir::new().unwrap();
        dir.scaffold_from_str("config.txt: hello\nsubdir/:\n")
            .unwrap();
        assert_eq!(
            fs::read_to_string(dir.child("config.txt")).unwrap(),
            "hello"
        );
        assert!(dir.child("subdir").is_dir());
    }

    #[test]
    #[cfg(feature = "scaffold")]
    fn scaffold_from_json() {
        let dir = TempDir::new().unwrap();
        dir.scaffold_from_str(r#"{"file.txt": "content"}"#).unwrap();
        assert_eq!(
            fs::read_to_string(dir.child("file.txt")).unwrap(),
            "content"
        );
    }
}
