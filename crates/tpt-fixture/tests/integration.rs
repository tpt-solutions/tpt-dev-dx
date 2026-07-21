//! End-to-end tests for `tpt-fixture` using the `#[tpt_fixture]` macro.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use tpt_fixture::tpt_fixture;

// ── A suite-scoped fixture ────────────────────────────────────────────────

static DB_INITS: AtomicUsize = AtomicUsize::new(0);
static DB_TEARDOWNS: AtomicUsize = AtomicUsize::new(0);

#[tpt_fixture(scope = "suite", name = "db")]
fn db() -> (String, Box<dyn FnOnce() + Send>) {
    DB_INITS.fetch_add(1, Ordering::SeqCst);
    // Return a teardown closure via the `(resource, teardown)` tuple form.
    (
        "shared-db".to_string(),
        Box::new(|| {
            DB_TEARDOWNS.fetch_add(1, Ordering::SeqCst);
        }),
    )
}

#[tpt_fixture]
#[test]
fn suite_test_one(db: Arc<String>) {
    assert_eq!(db.as_str(), "shared-db");
}

#[tpt_fixture]
#[test]
fn suite_test_two(db: Arc<String>) {
    assert_eq!(db.as_str(), "shared-db");
}

// ── A test-scoped fixture (fresh each test, teardown per test) ─────────────

static COUNTER: AtomicU32 = AtomicU32::new(0);
static COUNTER_DROPS: AtomicUsize = AtomicUsize::new(0);

#[tpt_fixture(scope = "test", name = "counter")]
fn counter() -> (u32, Box<dyn FnOnce() + Send>) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    (
        n,
        Box::new(move || {
            COUNTER_DROPS.fetch_add(1, Ordering::SeqCst);
        }),
    )
}

#[tpt_fixture]
#[test]
fn test_scope_first(counter: Arc<u32>) {
    assert_eq!(*counter, 0);
}

#[tpt_fixture]
#[test]
fn test_scope_second(counter: Arc<u32>) {
    // A separate resource (test scope) — but the global counter is process-shared,
    // so this sees the next value.
    assert_eq!(*counter, 1);
}

// ── An async fixture with async init via block_on ─────────────────────────

#[tpt_fixture(scope = "suite", name = "async_res")]
async fn async_res() -> u64 {
    let v = async { 99u64 }.await;
    v
}

#[tpt_fixture]
#[tokio::test]
async fn async_fixture_test(async_res: Arc<u64>) {
    assert_eq!(*async_res, 99);
}

// ── Teardown verification ──────────────────────────────────────────────────

#[tpt_fixture]
#[test]
fn verify_shared_reference_is_the_same_instance(db: Arc<String>) {
    assert_eq!(db.as_str(), "shared-db");
    // All suite tests share the same singleton — init must have fired exactly once
    // regardless of which test happened to run first.
    assert_eq!(
        DB_INITS.load(Ordering::SeqCst),
        1,
        "suite fixture must initialise exactly once"
    );
}

// This test deliberately runs last-ish and triggers teardown. Because tests run
// in parallel, it only asserts that at least the suite fixture was torn down.
#[test]
fn verify_teardown_fires_on_shutdown() {
    tpt_fixture::shutdown();
    // The suite `db` fixture should have been torn down exactly once.
    assert!(
        DB_TEARDOWNS.load(Ordering::SeqCst) >= 1,
        "suite fixture teardown should have fired by shutdown()"
    );
}
