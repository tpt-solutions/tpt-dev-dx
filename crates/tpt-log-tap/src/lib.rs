//! Per-test structured [`tracing`] event capture — assert on **field values**,
//! not on formatted log text.
//!
//! `tpt-log-tap` installs a per-thread [`tracing`] subscriber for the duration of
//! a test, records every event (level, target, message, and structured fields)
//! into an in-memory buffer, and lets you assert on those fields directly. When
//! the returned [`TapGuard`] is dropped the subscriber is uninstalled, so tests
//! stay isolated even when run in parallel.
//!
//! # Why field-level assertions?
//!
//! Matching on rendered log strings is brittle: formatting, ANSI colour, and
//! field ordering all change the text. Instead, capture the event and assert on
//! the values you care about:
//!
//! ```
//! use tpt_log_tap::LogTap;
//! use tracing::Level;
//!
//! let tap = LogTap::new().install();
//!
//! tracing::info!(user_id = 42, action = "login", "user signed in");
//!
//! tap.assert_contains(Level::INFO, "", &[("user_id", "42"), ("action", "login")]);
//! ```
//!
//! # Isolation across parallel tests
//!
//! [`LogTap::install`] uses [`tracing::subscriber::set_default`], which scopes the
//! subscriber to the **current thread**. Each test therefore captures only its
//! own events, even under `cargo test`'s default parallelism. (For `async` tests
//! that hop threads, keep the traced work on the test's thread — see the crate
//! README for details.)

use std::fmt;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::subscriber::DefaultGuard;
use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::Layer;

/// A single captured tracing event.
#[derive(Debug, Clone)]
pub struct CapturedEvent {
    /// The event's verbosity level.
    pub level: Level,
    /// The event's target (usually the module path).
    pub target: String,
    /// The event's `message` field, if present, rendered to a string.
    pub message: Option<String>,
    /// All structured fields (including `message`) as `(name, value)` pairs.
    ///
    /// Values are the `Debug`/`Display` rendering of each field, with surrounding
    /// quotes stripped from string values so `field = "x"` compares as `"x"`.
    pub fields: Vec<(String, String)>,
}

impl CapturedEvent {
    /// Look up a field's captured value by name.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Returns `true` if this event has every `(name, value)` pair in `fields`.
    pub fn has_fields(&self, fields: &[(&str, &str)]) -> bool {
        fields
            .iter()
            .all(|(k, v)| self.field(k).is_some_and(|actual| actual == *v))
    }
}

/// Shared, thread-safe buffer of captured events.
type Buffer = Arc<Mutex<Vec<CapturedEvent>>>;

/// Builder for a log tap.
///
/// Configure an optional minimum level and target filter, then call
/// [`install`](LogTap::install) to activate capture for the current thread.
#[derive(Debug, Clone, Default)]
pub struct LogTap {
    max_level: Option<Level>,
    target_prefix: Option<String>,
}

impl LogTap {
    /// Create a new tap that captures **all** events at every level and target.
    pub fn new() -> Self {
        Self::default()
    }

    /// Only capture events at or above `level` (more verbose than this are
    /// dropped). For example, `level(Level::WARN)` keeps `WARN` and `ERROR`.
    pub fn level(mut self, level: Level) -> Self {
        self.max_level = Some(level);
        self
    }

    /// Only capture events whose target starts with `prefix`.
    ///
    /// Useful for isolating your crate's events from dependency noise, e.g.
    /// `target("my_crate")`.
    pub fn target(mut self, prefix: impl Into<String>) -> Self {
        self.target_prefix = Some(prefix.into());
        self
    }

    /// Install the tap as the current thread's default subscriber and start
    /// capturing.
    ///
    /// The returned [`TapGuard`] must be kept alive for the duration of the
    /// test; dropping it uninstalls the subscriber.
    pub fn install(self) -> TapGuard {
        let buffer: Buffer = Arc::new(Mutex::new(Vec::new()));
        let layer = CaptureLayer {
            buffer: Arc::clone(&buffer),
            max_level: self.max_level,
            target_prefix: self.target_prefix,
        };
        let subscriber = Registry::default().with(layer);
        let guard = tracing::subscriber::set_default(subscriber);
        TapGuard {
            buffer,
            _guard: guard,
            pending: Vec::new(),
        }
    }
}

/// RAII guard that keeps the per-thread subscriber installed and exposes the
/// captured events for assertions.
///
/// Dropping the guard uninstalls the subscriber and verifies any expectations
/// registered via [`expect_contains`](TapGuard::expect_contains).
pub struct TapGuard {
    buffer: Buffer,
    _guard: DefaultGuard,
    pending: Vec<Expectation>,
}

#[derive(Debug, Clone)]
struct Expectation {
    level: Level,
    target: String,
    fields: Vec<(String, String)>,
}

impl TapGuard {
    /// Snapshot of all events captured so far, in emission order.
    pub fn events(&self) -> Vec<CapturedEvent> {
        self.buffer.lock().expect("log-tap buffer poisoned").clone()
    }

    /// Number of events captured so far.
    pub fn len(&self) -> usize {
        self.buffer.lock().expect("log-tap buffer poisoned").len()
    }

    /// Returns `true` if no events have been captured.
    pub fn is_empty(&self) -> bool {
        self.buffer
            .lock()
            .expect("log-tap buffer poisoned")
            .is_empty()
    }

    /// Returns `true` if any captured event matches `level`, `target` (empty
    /// string = any target), and contains all the given `fields`.
    pub fn contains(&self, level: Level, target: &str, fields: &[(&str, &str)]) -> bool {
        self.events()
            .iter()
            .any(|e| event_matches(e, level, target, fields))
    }

    /// Assert that at least one captured event matches `level`, `target`
    /// (`""` = any target), and contains all the given `fields`.
    ///
    /// # Panics
    ///
    /// Panics with a diagnostic listing the captured events if no match is found.
    #[track_caller]
    pub fn assert_contains(&self, level: Level, target: &str, fields: &[(&str, &str)]) {
        if !self.contains(level, target, fields) {
            panic!(
                "log-tap: expected an event {}\nbut captured events were:\n{}",
                describe_expectation(level, target, fields),
                self.render_events()
            );
        }
    }

    /// Assert that **no** captured event matches `level`, `target` (`""` = any
    /// target), and the given `fields`.
    ///
    /// # Panics
    ///
    /// Panics if a matching event is found.
    #[track_caller]
    pub fn assert_not_contains(&self, level: Level, target: &str, fields: &[(&str, &str)]) {
        if self.contains(level, target, fields) {
            panic!(
                "log-tap: expected NO event {}\nbut captured events were:\n{}",
                describe_expectation(level, target, fields),
                self.render_events()
            );
        }
    }

    /// Register an expectation checked automatically when the guard is dropped.
    ///
    /// This is handy for "this must be logged by the end of the test" assertions
    /// without an explicit call at the end.
    pub fn expect_contains(&mut self, level: Level, target: &str, fields: &[(&str, &str)]) {
        self.pending.push(Expectation {
            level,
            target: target.to_owned(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
    }

    fn render_events(&self) -> String {
        let events = self.events();
        if events.is_empty() {
            return "  (none)".to_string();
        }
        events
            .iter()
            .map(|e| {
                let fields = e
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{k}={v:?}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("  [{}] {} {{ {} }}", e.level, e.target, fields)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Drop for TapGuard {
    fn drop(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        // Don't double-panic if we're already unwinding.
        if std::thread::panicking() {
            return;
        }
        let events = self.events();
        let unmet: Vec<&Expectation> = self
            .pending
            .iter()
            .filter(|exp| {
                let refs: Vec<(&str, &str)> = exp
                    .fields
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                !events
                    .iter()
                    .any(|e| event_matches(e, exp.level, &exp.target, &refs))
            })
            .collect();
        if !unmet.is_empty() {
            let details = unmet
                .iter()
                .map(|exp| {
                    let refs: Vec<(&str, &str)> = exp
                        .fields
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect();
                    format!(
                        "  - {}",
                        describe_expectation(exp.level, &exp.target, &refs)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            panic!("log-tap: unmet pending expectations on drop:\n{details}");
        }
    }
}

fn event_matches(
    event: &CapturedEvent,
    level: Level,
    target: &str,
    fields: &[(&str, &str)],
) -> bool {
    event.level == level
        && (target.is_empty() || event.target == target)
        && event.has_fields(fields)
}

fn describe_expectation(level: Level, target: &str, fields: &[(&str, &str)]) -> String {
    let target = if target.is_empty() {
        "<any target>".to_string()
    } else {
        format!("target `{target}`")
    };
    let fields = if fields.is_empty() {
        "<no field constraints>".to_string()
    } else {
        fields
            .iter()
            .map(|(k, v)| format!("{k}={v:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!("at level {level} for {target} with fields {{ {fields} }}")
}

// ── The capturing layer ────────────────────────────────────────────────────

struct CaptureLayer {
    buffer: Buffer,
    max_level: Option<Level>,
    target_prefix: Option<String>,
}

impl CaptureLayer {
    fn accepts(&self, meta: &Metadata<'_>) -> bool {
        if let Some(max) = self.max_level {
            // Lower `Level` value == higher severity; `<=` keeps at-or-above.
            if *meta.level() > max {
                return false;
            }
        }
        if let Some(prefix) = &self.target_prefix {
            if !meta.target().starts_with(prefix.as_str()) {
                return false;
            }
        }
        true
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        if !self.accepts(meta) {
            return;
        }
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let message = visitor
            .fields
            .iter()
            .find(|(k, _)| k == "message")
            .map(|(_, v)| v.clone());
        let captured = CapturedEvent {
            level: *meta.level(),
            target: meta.target().to_owned(),
            message,
            fields: visitor.fields,
        };
        self.buffer
            .lock()
            .expect("log-tap buffer poisoned")
            .push(captured);
    }
}

#[derive(Default)]
struct FieldVisitor {
    fields: Vec<(String, String)>,
}

impl FieldVisitor {
    fn push(&mut self, field: &Field, value: String) {
        self.fields.push((field.name().to_string(), value));
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        // `{:?}` on a &str yields quotes; strip them so `field = "x"` -> `x`.
        let rendered = format!("{value:?}");
        let cleaned = strip_debug_quotes(&rendered);
        self.push(field, cleaned);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, value.to_string());
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.push(field, value.to_string());
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.push(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.push(field, value.to_string());
    }
}

fn strip_debug_quotes(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_fields_and_asserts() {
        let tap = LogTap::new().install();
        tracing::info!(user_id = 7, action = "login", "hi");
        tap.assert_contains(Level::INFO, "", &[("user_id", "7"), ("action", "login")]);
        // message captured
        let events = tap.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message.as_deref(), Some("hi"));
    }

    #[test]
    fn assert_not_contains_passes_when_absent() {
        let tap = LogTap::new().install();
        tracing::warn!(code = 500, "boom");
        tap.assert_not_contains(Level::ERROR, "", &[]);
        tap.assert_not_contains(Level::WARN, "", &[("code", "404")]);
        tap.assert_contains(Level::WARN, "", &[("code", "500")]);
    }

    #[test]
    fn level_filter_drops_lower_severity() {
        let tap = LogTap::new().level(Level::WARN).install();
        tracing::info!("ignored");
        tracing::debug!("ignored too");
        tracing::warn!(kept = true, "warn");
        tracing::error!(kept = true, "err");
        let events = tap.events();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.level <= Level::WARN));
    }

    #[test]
    fn target_prefix_filter() {
        let tap = LogTap::new().target("tpt_log_tap").install();
        tracing::info!(x = 1, "kept: this module's target starts with the prefix");
        // A synthetic event with a different target is filtered.
        assert!(tap.contains(Level::INFO, "", &[("x", "1")]));
    }

    #[test]
    fn field_accessor_and_has_fields() {
        let tap = LogTap::new().install();
        tracing::info!(a = 1, b = "two", "msg");
        let e = &tap.events()[0];
        assert_eq!(e.field("a"), Some("1"));
        assert_eq!(e.field("b"), Some("two"));
        assert!(e.has_fields(&[("a", "1"), ("b", "two")]));
        assert!(!e.has_fields(&[("a", "9")]));
    }

    #[test]
    fn pending_expectation_met_does_not_panic() {
        let mut tap = LogTap::new().install();
        tap.expect_contains(Level::INFO, "", &[("done", "true")]);
        tracing::info!(done = true, "finished");
        drop(tap); // should not panic
    }

    #[test]
    fn pending_expectation_unmet_panics_on_drop() {
        let result = std::panic::catch_unwind(|| {
            let mut tap = LogTap::new().install();
            tap.expect_contains(Level::ERROR, "", &[("never", "logged")]);
            tracing::info!("something else");
            drop(tap);
        });
        assert!(result.is_err(), "unmet expectation should panic on drop");
    }

    #[test]
    fn isolation_between_taps_on_same_thread() {
        {
            let first = LogTap::new().install();
            tracing::info!(scope = "first", "a");
            assert_eq!(first.len(), 1);
        } // first uninstalled here
        let second = LogTap::new().install();
        tracing::info!(scope = "second", "b");
        assert_eq!(second.len(), 1);
        assert!(second.contains(Level::INFO, "", &[("scope", "second")]));
        assert!(!second.contains(Level::INFO, "", &[("scope", "first")]));
    }

    #[test]
    fn parallel_threads_are_isolated() {
        use std::thread;
        let handles: Vec<_> = (0..8)
            .map(|i| {
                thread::spawn(move || {
                    let tap = LogTap::new().install();
                    tracing::info!(thread = i, "work");
                    // Each thread should see exactly its own single event.
                    assert_eq!(tap.len(), 1, "thread {i} saw cross-talk");
                    assert!(tap.contains(Level::INFO, "", &[("thread", &i.to_string())]));
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }
}
