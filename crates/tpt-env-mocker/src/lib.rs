use std::env;
use std::sync::{Mutex, MutexGuard};

// Re-export the attribute macro when the `macros` feature is enabled.
#[cfg(feature = "macros")]
pub use tpt_env_mocker_macros::tpt_env;

/// Global mutex that serialises all tests that touch environment variables.
///
/// Any test using [`MockEnv`] or `#[tpt_env(...)]` holds this lock for the
/// duration of its body, preventing env-var races in parallel async tests.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// A builder for constructing an environment mock.
///
/// # Example
/// ```rust
/// use tpt_env_mocker::MockEnv;
///
/// let _guard = MockEnv::new()
///     .set("DATABASE_URL", "postgres://localhost/test")
///     .set("LOG_LEVEL", "debug")
///     .lock();
///
/// assert_eq!(std::env::var("DATABASE_URL").unwrap(), "postgres://localhost/test");
/// // `_guard` dropped here → env vars restored
/// ```
#[derive(Default)]
pub struct MockEnv {
    ops: Vec<Op>,
}

#[derive(Clone)]
enum Op {
    Set(String, String),
    Remove(String),
}

impl MockEnv {
    /// Create a new builder with no pending operations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue setting `key` to `value` when [`lock`](Self::lock) is called.
    pub fn set(mut self, key: &str, value: &str) -> Self {
        self.ops.push(Op::Set(key.to_owned(), value.to_owned()));
        self
    }

    /// Queue removing `key` from the environment when [`lock`](Self::lock) is called.
    pub fn remove(mut self, key: &str) -> Self {
        self.ops.push(Op::Remove(key.to_owned()));
        self
    }

    /// Acquire the global env lock, apply all queued operations, and return a
    /// guard that restores the original values on drop.
    ///
    /// # Panics
    /// Does not panic if the global mutex is poisoned: a previous test that
    /// panicked while holding the lock is automatically recovered so subsequent
    /// tests still run.
    pub fn lock(self) -> EnvGuard {
        // If a previous test panicked while holding the lock, recover the
        // inner guard so subsequent tests still run.
        let lock = ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Snapshot originals before applying ops.
        let original_values: Vec<(String, Option<String>)> = self
            .ops
            .iter()
            .map(|op| {
                let key = match op {
                    Op::Set(k, _) | Op::Remove(k) => k.clone(),
                };
                let original = env::var(&key).ok();
                (key, original)
            })
            .collect();

        // Apply ops.
        for op in self.ops {
            match op {
                Op::Set(k, v) => {
                    // SAFETY: we hold the global lock so no other code modifies
                    // the environment concurrently within this process.
                    unsafe { env::set_var(&k, &v) };
                }
                Op::Remove(k) => {
                    unsafe { env::remove_var(&k) };
                }
            }
        }

        EnvGuard {
            _lock: lock,
            original_values,
        }
    }

    /// Recover from a poisoned global mutex.
    ///
    /// Call this in test setup if a previous test panicked while holding the lock.
    pub fn recover_poison() {
        let _unused = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    }
}

/// RAII guard returned by [`MockEnv::lock`].
///
/// When dropped, all environment variables modified by the associated
/// [`MockEnv`] are restored to their original values (or removed if they
/// did not exist before).
pub struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    original_values: Vec<(String, Option<String>)>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, original) in &self.original_values {
            match original {
                Some(val) => unsafe { env::set_var(key, val) },
                None => unsafe { env::remove_var(key) },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_and_restores_env_var() {
        let key = "TPT_TEST_VAR_RESTORE";
        env::remove_var(key);
        {
            let _g = MockEnv::new().set(key, "hello").lock();
            assert_eq!(env::var(key).unwrap(), "hello");
        }
        assert!(env::var(key).is_err(), "should be restored (removed)");
    }

    #[test]
    fn restores_original_value() {
        let key = "TPT_TEST_VAR_ORIGINAL";
        unsafe { env::set_var(key, "original") };
        {
            let _g = MockEnv::new().set(key, "overridden").lock();
            assert_eq!(env::var(key).unwrap(), "overridden");
        }
        assert_eq!(env::var(key).unwrap(), "original");
    }

    #[test]
    fn remove_op() {
        let key = "TPT_TEST_VAR_REMOVE";
        unsafe { env::set_var(key, "present") };
        {
            let _g = MockEnv::new().remove(key).lock();
            assert!(env::var(key).is_err());
        }
        assert_eq!(env::var(key).unwrap(), "present");
    }

    #[test]
    fn multiple_vars() {
        let k1 = "TPT_MULTI_A";
        let k2 = "TPT_MULTI_B";
        {
            let _g = MockEnv::new().set(k1, "a").set(k2, "b").lock();
            assert_eq!(env::var(k1).unwrap(), "a");
            assert_eq!(env::var(k2).unwrap(), "b");
        }
        assert!(env::var(k1).is_err());
        assert!(env::var(k2).is_err());
    }
}
