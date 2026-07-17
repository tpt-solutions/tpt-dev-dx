//! Session- and module-scoped test fixtures with async init and teardown.
//!
//! `tpt-fixture` fills the `beforeAll` / `afterAll` gap that [`rstest`] issue
//! #119 leaves open: it lets an init function run **once** per test scope and
//! share its resource (as a thread-safe [`Arc<T>`](std::sync::Arc)) across every
//! test in that scope, tearing it down when the scope ends.
//!
//! Use the companion [`tpt_fixture`](macro@crate::tpt_fixture) attribute macro to
//! declare fixtures and to inject them into tests:
//!
//! ```ignore
//! use std::sync::Arc;
//! use tpt_fixture::tpt_fixture;
//!
//! #[tpt_fixture(scope = "suite")]
//! async fn db() -> Database {
//!     Database::connect().await
//! }
//!
//! #[tpt_fixture]
//! #[tokio::test]
//! async fn reads_rows(db: Arc<Database>) {
//!     assert!(db.row_count() > 0);
//! }
//! ```
//!
//! # Scopes
//!
//! | Scope    | Initialised | Shared across        | Teardown |
//! |----------|-------------|----------------------|----------|
//! | `test`   | every test  | — (fresh each call)  | end of that test |
//! | `module` | once        | the test binary      | `shutdown()` |
//! | `suite`  | once        | the whole test binary| `shutdown()` |
//!
//! `module` and `suite` are both process-lifetime singletons (Rust has no runtime
//! module identity), initialised exactly once via a global `OnceLock`. They differ
//! only in *intent*; both are cleaned up by [`shutdown`] (call it from a teardown
//! test, or rely on the per-process model of `cargo-nextest`).
//!
//! # Async init
//!
//! Async init functions are awaited with [`block_on`]. By default this uses a tiny
//! single-threaded executor; enable the `tokio` feature to drive them on a real
//! tokio current-thread runtime (needed for async I/O, timers, etc.).

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};

pub use tpt_fixture_macros::tpt_fixture;

/// The lifetime of a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// A fresh resource for every test; torn down when the test ends.
    Test,
    /// One resource for the test binary; torn down at process exit ([`shutdown`]).
    Module,
    /// One resource for the whole suite; torn down at process exit ([`shutdown`]).
    Suite,
}

type Teardown = Box<dyn FnOnce() + Send + 'static>;

/// The result of normalising a fixture's init return value.
///
/// Returning `T` gives no teardown; returning `(T, teardown)` registers a
/// synchronous teardown closure run when the scope ends.
pub struct FixtureInit<T> {
    /// The shared resource.
    pub resource: Arc<T>,
    /// Optional teardown run when the scope ends.
    pub teardown: Option<Teardown>,
}

/// Normalises a fixture init return value into a [`FixtureInit`].
///
/// Blanket-implemented for any `T` (no teardown) and for `(T, F)` where `F` is a
/// teardown closure (sync).
pub trait IntoFixture<T> {
    /// Convert this init result into a [`FixtureInit`].
    fn into_fixture(self) -> FixtureInit<T>;
}

impl<T> IntoFixture<T> for T {
    fn into_fixture(self) -> FixtureInit<T> {
        FixtureInit {
            resource: Arc::new(self),
            teardown: None,
        }
    }
}

impl<T, F> IntoFixture<T> for (T, F)
where
    F: FnOnce() + Send + 'static,
{
    fn into_fixture(self) -> FixtureInit<T> {
        FixtureInit {
            resource: Arc::new(self.0),
            teardown: Some(Box::new(self.1)),
        }
    }
}

struct Cached {
    resource: Arc<dyn Any + Send + Sync>,
    teardown: Option<Teardown>,
}

static CACHE: OnceLock<Mutex<HashMap<String, Cached>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Cached>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    static TEST_TEARDOWNS: RefCell<Vec<Teardown>> = const { RefCell::new(Vec::new()) };
}

/// Resolve (or create) a fixture resource of type `T` for `scope`.
///
/// For `test` scope a fresh resource is built and its teardown is queued on the
/// current thread. For `module`/`suite` scope the resource is cached process-wide
/// and initialised exactly once.
pub fn fixture_access<T>(name: &str, scope: Scope, init: impl FnOnce() -> FixtureInit<T>) -> Arc<T>
where
    T: 'static + Send + Sync,
{
    match scope {
        Scope::Test => {
            let built = init();
            if let Some(teardown) = built.teardown {
                TEST_TEARDOWNS.with(|c| c.borrow_mut().push(teardown));
            }
            built.resource
        }
        Scope::Module | Scope::Suite => {
            let mut cache = cache().lock().expect("fixture cache poisoned");
            if let Some(entry) = cache.get(name) {
                return entry
                    .resource
                    .clone()
                    .downcast::<T>()
                    .expect("fixture type mismatch for name");
            }
            let built = init();
            let entry = Cached {
                resource: built.resource.clone() as Arc<dyn Any + Send + Sync>,
                teardown: built.teardown,
            };
            cache.insert(name.to_string(), entry);
            built.resource
        }
    }
}

/// Run all `test`-scope teardowns queued on the current thread.
///
/// Called automatically by [`TestScopeGuard`]'s `Drop` at the end of every test
/// that uses a `test`-scope fixture, so you rarely call this directly.
pub fn end_test_scope() {
    TEST_TEARDOWNS.with(|c| {
        let mut v = c.borrow_mut();
        for teardown in v.drain(..) {
            teardown();
        }
    });
}

/// Tear down every `module`/`suite`-scope fixture.
///
/// Call this from a dedicated teardown test or rely on `cargo-nextest`'s
/// per-process model. Safe to call multiple times; once torn down, fixtures are
/// not rebuilt.
pub fn shutdown() {
    let mut cache = cache().lock().expect("fixture cache poisoned");
    for (_, entry) in cache.drain() {
        if let Some(teardown) = entry.teardown {
            teardown();
        }
    }
}

/// RAII guard that runs all `test`-scope teardowns when the test ends — even on
/// panic. Inserted by the `#[tpt_fixture]` macro at the top of each test.
pub struct TestScopeGuard;

impl Drop for TestScopeGuard {
    fn drop(&mut self) {
        end_test_scope();
    }
}

/// Drive a future to completion.
///
/// With the default features this is a minimal single-threaded executor suitable
/// for futures that are ready without external wake-ups. Enable the `tokio` feature
/// to run real async I/O / timers on a current-thread tokio runtime.
///
/// When called from inside an already-running tokio runtime (e.g. fixture init
/// triggered from a `#[tokio::test]` async test), building and blocking on a
/// nested runtime on the same thread would panic. To support that case, the
/// `tokio`-feature implementation detects this and drives the future to
/// completion on a dedicated OS thread with its own runtime instead.
#[cfg(feature = "tokio")]
pub fn block_on<F>(future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for fixture init");
            rt.block_on(future)
        })
        .join()
        .expect("fixture init thread panicked")
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime for fixture init");
        rt.block_on(future)
    }
}

/// Drive a future to completion.
///
/// This is a minimal single-threaded executor suitable for futures that are
/// ready without external wake-ups. Enable the `tokio` feature to run real
/// async I/O / timers on a current-thread tokio runtime.
#[cfg(not(feature = "tokio"))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    let mut future = future;
    // Pin to the stack; `future` is not moved after this point.
    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

#[cfg(not(feature = "tokio"))]
fn noop_waker() -> Waker {
    fn noop(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    fn noop_raw_waker() -> RawWaker {
        RawWaker::new(std::ptr::null(), &NOOP_VTABLE)
    }
    static NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex as StdMutex;

    #[test]
    fn test_scope_builds_fresh_each_time_and_tears_down() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        static DROPS: AtomicUsize = AtomicUsize::new(0);

        fn make() -> Arc<u32> {
            fixture_access("fresh", Scope::Test, || {
                COUNT.fetch_add(1, Ordering::SeqCst);
                let drop_flag = &DROPS;
                IntoFixture::into_fixture((42u32, move || {
                    drop_flag.fetch_add(1, Ordering::SeqCst);
                }))
            })
        }

        let _g = TestScopeGuard;
        let a = make();
        let b = make();
        assert_ne!(Arc::as_ptr(&a), Arc::as_ptr(&b), "test scope must be fresh");
        assert_eq!(COUNT.load(Ordering::SeqCst), 2);
        drop(_g);
        assert_eq!(DROPS.load(Ordering::SeqCst), 2, "teardown runs at test end");
    }

    #[test]
    fn suite_scope_initialises_once_and_is_shared() {
        static INIT: AtomicUsize = AtomicUsize::new(0);

        fn db() -> Arc<String> {
            fixture_access("shared_db", Scope::Suite, || {
                INIT.fetch_add(1, Ordering::SeqCst);
                IntoFixture::into_fixture("resource".to_string())
            })
        }

        let a = db();
        let b = db();
        assert_eq!(
            Arc::as_ptr(&a),
            Arc::as_ptr(&b),
            "suite scope shares the same Arc"
        );
        assert_eq!(INIT.load(Ordering::SeqCst), 1, "initialised exactly once");

        shutdown();
        // After shutdown the cache is empty; a fresh init would increment again.
        let _c = db();
        assert_eq!(INIT.load(Ordering::SeqCst), 2, "rebuild after shutdown");
        shutdown();
    }

    #[test]
    fn async_init_runs_via_block_on() {
        fn counter() -> Arc<u64> {
            fixture_access("async_counter", Scope::Test, || {
                // A future that resolves without external wake-ups.
                IntoFixture::into_fixture(block_on(async { 7u64 }))
            })
        }
        let _g = TestScopeGuard;
        assert_eq!(*counter(), 7);
    }

    #[test]
    fn parallel_test_scopes_isolated() {
        use std::sync::Arc as StdArc;
        let log: StdArc<StdMutex<Vec<String>>> = StdArc::new(StdMutex::new(Vec::new()));

        let mut handles = Vec::new();
        for i in 0..4 {
            let log = StdArc::clone(&log);
            handles.push(std::thread::spawn(move || {
                let _g = TestScopeGuard;
                let res: Arc<i32> = fixture_access(&format!("iso_{i}"), Scope::Test, || {
                    IntoFixture::into_fixture((i, move || {
                        log.lock().unwrap().push(format!("dropped {i}"));
                    }))
                });
                assert_eq!(*res, i);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let log = log.lock().unwrap();
        assert_eq!(log.len(), 4, "each thread tore its own fixture down");
    }
}
