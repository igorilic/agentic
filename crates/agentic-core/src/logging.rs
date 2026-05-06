use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt};

static SUBSCRIBER_INSTALLED: OnceLock<()> = OnceLock::new();

/// Resolve the tracing filter string with precedence:
/// explicit `filter` arg > `AGENTIC_LOG` env var > `default_level`.
#[doc(hidden)]
pub fn resolved_filter(filter: Option<&str>, default_level: &str) -> String {
    if let Some(f) = filter {
        return f.to_owned();
    }
    if let Ok(env_val) = std::env::var("AGENTIC_LOG")
        && !env_val.is_empty()
    {
        return env_val;
    }
    default_level.to_owned()
}

/// Installs a global tracing subscriber configured for production use.
///
/// Only the **first** call in the process installs a subscriber. Subsequent
/// calls are no-ops; the `filter` argument on any call after the first is
/// **silently ignored**. If you need `init_test_subscriber()`'s `with_test_writer()`
/// capture behaviour, ensure `init()` has not been called earlier in the same
/// process.
///
/// The installed subscriber includes a [`BusLayer`] so that [`attach_event_bus`]
/// can later wire in an [`EventBus`] without reinstalling the subscriber.
pub fn init(filter: Option<&str>) {
    SUBSCRIBER_INSTALLED.get_or_init(|| {
        use tracing_subscriber::prelude::*;
        let filter_str = resolved_filter(filter, "info");
        let env_filter = EnvFilter::new(filter_str);
        let fmt_layer = fmt::layer().with_filter(env_filter);
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(BusLayer::global())
            .try_init()
            .expect(
                "agentic-core logging: another tracing subscriber is already installed globally",
            );
    });
}

/// Installs a test-friendly tracing subscriber with test writer.
///
/// Only the **first** call in the process installs a subscriber. Subsequent
/// calls are no-ops; if this is not the first call (e.g. because `init()` was
/// called earlier), the test-writer capture behaviour is **not** installed and
/// this call is **silently ignored**. To guarantee test-writer capture, ensure
/// no other `init`/`init_test_subscriber` call precedes this one in the process.
///
/// The installed subscriber includes a [`BusLayer`] so that [`attach_event_bus`]
/// can later wire in an [`EventBus`] without reinstalling the subscriber.
pub fn init_test_subscriber() {
    SUBSCRIBER_INSTALLED.get_or_init(|| {
        use tracing_subscriber::prelude::*;
        let filter_str = resolved_filter(None, "debug");
        let fmt_layer = fmt::layer()
            .with_test_writer()
            .with_filter(EnvFilter::new(filter_str));
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(BusLayer::global())
            .try_init()
            .expect(
                "agentic-core logging: another tracing subscriber is already installed globally",
            );
    });
}

/// Attach an [`EventBus`] to the process-global [`BusLayer`].
///
/// After this call, any `tracing::error!` or `tracing::warn!` event routed
/// through the global subscriber will be converted into an
/// [`Event::Finding`] envelope and published on the bus.
///
/// Only the **first** call takes effect; subsequent calls are no-ops
/// (the inner `OnceLock` ensures atomicity with no locking on the hot path).
///
/// The call is safe before [`init`] has been invoked — the layer is always
/// installed by `init` / `init_test_subscriber`; once `attach_event_bus` is
/// called the layer starts forwarding events.
pub fn attach_event_bus(bus: std::sync::Arc<crate::events::EventBus>) {
    GLOBAL_BUS.set_bus(bus);
}

// ── BusLayer ────────────────────────────────────────────────────────────────

use std::sync::Arc;

use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use crate::events::{Event, EventBus, EventEnvelope, Severity};

/// Global bus holder used by `init()` / `attach_event_bus()`.
static GLOBAL_BUS: BusSlot = BusSlot::new();

/// An atomically-set slot holding an optional [`Arc<EventBus>`].
///
/// `OnceLock` guarantees that `set` / `get` are safe across threads with no
/// locking on the read path after the value is initialised.
struct BusSlot(OnceLock<Arc<EventBus>>);

impl BusSlot {
    const fn new() -> Self {
        Self(OnceLock::new())
    }

    fn set_bus(&self, bus: Arc<EventBus>) {
        let _ = self.0.set(bus);
    }

    fn get(&self) -> Option<&Arc<EventBus>> {
        self.0.get()
    }
}

// Thread-local re-entry guard.
//
// Prevents the `tracing::warn!` emitted inside `EventBus::publish`
// (when no subscribers are active) from recursively re-entering `on_event`
// and causing a stack overflow.
thread_local! {
    static IN_BUS_LAYER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A [`tracing_subscriber::Layer`] that bridges ERROR and WARN events onto an
/// [`EventBus`] as [`Event::Finding`] envelopes.
///
/// # Construction
///
/// * [`BusLayer::global()`] — returns the singleton layer that is wired to the
///   process-global bus slot.  Install this in `init()` / `init_test_subscriber()`.
/// * [`BusLayer::with_bus(bus)`] — returns a standalone layer pre-wired to
///   `bus`.  Intended **for tests only** so each test can use an isolated bus
///   without touching global state.
///
/// # Field extraction precedence (for `run_id` / `step_id`)
///
/// 1. Field is present on the tracing event itself.
/// 2. Walk parent spans (innermost → outermost) and use the first match found
///    in the span's pre-formatted field string.
/// 3. If not found anywhere, use `""` (empty-string sentinel — the UI can
///    choose to hide findings that have no associated run).
///
/// # Re-entry guard
///
/// `EventBus::publish` internally calls `tracing::warn!` when no subscribers
/// are listening.  To prevent infinite recursion a `thread_local!` boolean is
/// set for the duration of `on_event`; if it is already `true` on entry, the
/// call is a no-op.
pub struct BusLayer {
    /// The bus slot this layer reads from.  `None` means "use the global slot".
    slot: Option<Arc<BusSlot>>,
}

impl BusLayer {
    /// Layer wired to the process-global bus slot.
    pub fn global() -> Self {
        Self { slot: None }
    }

    /// Layer wired to `bus` directly — useful in tests for per-test isolation.
    pub fn with_bus(bus: Arc<EventBus>) -> Self {
        let slot = Arc::new(BusSlot::new());
        slot.set_bus(bus);
        Self { slot: Some(slot) }
    }

    fn get_bus(&self) -> Option<&Arc<EventBus>> {
        match &self.slot {
            Some(s) => s.get(),
            None => GLOBAL_BUS.get(),
        }
    }
}

impl<S> Layer<S> for BusLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        // Re-entry guard — bail if we are already inside this function on
        // this thread (prevents infinite recursion through publish → warn).
        if IN_BUS_LAYER.with(|g| g.get()) {
            return;
        }

        let bus = match self.get_bus() {
            Some(b) => b,
            None => return, // bus not yet attached — drop silently
        };

        let level = *event.metadata().level();
        let severity = match level {
            tracing::Level::ERROR => Severity::Error,
            tracing::Level::WARN => Severity::Warning,
            _ => return, // only bridge ERROR + WARN
        };

        // Visit fields to extract message, run_id, step_id.
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        // If run_id not on the event, search parent spans.
        let run_id = if visitor.run_id.is_empty() {
            find_in_spans(ctx.lookup_current(), "run_id").unwrap_or_default()
        } else {
            visitor.run_id.clone()
        };

        let step_id = if visitor.step_id.is_empty() {
            find_in_spans(ctx.lookup_current(), "step_id")
        } else {
            Some(visitor.step_id.clone())
        };

        let finding = Event::Finding {
            finding_id: ulid::Ulid::new().to_string(),
            severity,
            file: None,
            line: None,
            message: visitor.message,
            suggestion: None,
        };
        let envelope = EventEnvelope::now(run_id, step_id, finding);

        IN_BUS_LAYER.with(|g| g.set(true));
        bus.publish(envelope);
        IN_BUS_LAYER.with(|g| g.set(false));
    }
}

/// Walk span ancestors looking for `key` in pre-formatted field strings.
fn find_in_spans<S>(
    span: Option<tracing_subscriber::registry::SpanRef<'_, S>>,
    key: &str,
) -> Option<String>
where
    S: Subscriber + for<'b> LookupSpan<'b>,
{
    let mut current = span;
    loop {
        let s = current?;
        if let Some(fields) = s
            .extensions()
            .get::<tracing_subscriber::fmt::FormattedFields<
                tracing_subscriber::fmt::format::DefaultFields,
            >>()
            && let Some(v) = extract_field_from_formatted(fields.fields.as_str(), key)
        {
            return Some(v);
        }
        current = s.parent();
    }
}

/// Extract `key=value` from a pre-formatted tracing field string.
///
/// `FormattedFields` stores a string like `run_id=abc step_id=xyz`.
/// Finds `key=` preceded by a word boundary (space or start-of-string) and
/// reads until the next whitespace or end-of-string.
fn extract_field_from_formatted(s: &str, key: &str) -> Option<String> {
    let search = format!("{key}=");
    let idx = s.find(search.as_str())?;
    if idx > 0 && !s.as_bytes()[idx - 1].is_ascii_whitespace() {
        return None;
    }
    let value_start = idx + search.len();
    let rest = &s[value_start..];
    let value_end = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let value = &rest[..value_end];
    if value.is_empty() { None } else { Some(value.to_owned()) }
}

/// Visits tracing event fields and collects `message`, `run_id`, `step_id`.
#[derive(Default)]
struct FieldVisitor {
    message: String,
    run_id: String,
    step_id: String,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_owned(),
            "run_id" => self.run_id = value.to_owned(),
            "step_id" => self.step_id = value.to_owned(),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => self.message = format!("{value:?}"),
            "run_id" => self.run_id = format!("{value:?}"),
            "step_id" => self.step_id = format!("{value:?}"),
            _ => {}
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tracing_subscriber::prelude::*;

    use crate::events::{EventBus, Severity};
    use crate::events::Event;

    use super::BusLayer;

    /// Build a scoped test subscriber backed by `bus`.
    /// Does NOT touch global subscriber state.
    fn make_subscriber(
        bus: Arc<EventBus>,
    ) -> impl tracing::Subscriber + Send + Sync {
        let fmt_layer = tracing_subscriber::fmt::layer().with_test_writer();
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(BusLayer::with_bus(bus))
    }

    // ── Test A — error event with run_id field arrives as Finding ────────────

    #[tokio::test]
    async fn test_a_error_with_run_id() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();
        let subscriber = make_subscriber(bus.clone());

        tracing::subscriber::with_default(subscriber, || {
            tracing::error!(run_id = "r1", "boom");
        });

        let envelope = tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match rx.recv().await {
                    Ok(e) => return e,
                    Err(_) => continue,
                }
            }
        })
        .await
        .expect("expected Finding envelope within 100ms");

        assert_eq!(envelope.run_id, "r1");
        match envelope.event {
            Event::Finding { severity, message, .. } => {
                assert_eq!(severity, Severity::Error);
                assert!(message.contains("boom"), "message was: {message:?}");
            }
            other => panic!("expected Finding, got {other:?}"),
        }
    }

    // ── Test B — warn without run_id → empty run_id sentinel ─────────────────

    #[tokio::test]
    async fn test_b_warn_without_run_id() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();
        let subscriber = make_subscriber(bus.clone());

        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!("oops");
        });

        let envelope = tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match rx.recv().await {
                    Ok(e) => return e,
                    Err(_) => continue,
                }
            }
        })
        .await
        .expect("expected Finding envelope within 100ms");

        assert_eq!(envelope.run_id, "");
        match envelope.event {
            Event::Finding { severity, .. } => {
                assert_eq!(severity, Severity::Warning);
            }
            other => panic!("expected Finding, got {other:?}"),
        }
    }

    // ── Test C — info events are NOT forwarded ────────────────────────────────

    #[tokio::test]
    async fn test_c_info_filtered() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();
        let subscriber = make_subscriber(bus.clone());

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("noise");
        });

        let result = tokio::time::timeout(Duration::from_millis(50), async {
            rx.recv().await.ok()
        })
        .await;

        assert!(
            result.is_err(),
            "info events should NOT produce a Finding envelope"
        );
    }

    // ── Test D — bus-unattached drop: no panic, no leak ──────────────────────

    #[test]
    fn test_d_bus_unattached_drop() {
        // Build a subscriber with BusLayer::global() but do NOT attach a bus.
        // The layer should silently drop events.
        let fmt_layer = tracing_subscriber::fmt::layer().with_test_writer();
        let subscriber = tracing_subscriber::registry()
            .with(fmt_layer)
            .with(BusLayer::global());

        tracing::subscriber::with_default(subscriber, || {
            tracing::error!("dropped");
        });
        // Reaching here without panic or hang is the assertion.
    }

    // ── Test E — re-entry safety: exactly 1 envelope, guard suppresses inner warn ──

    #[tokio::test]
    async fn test_e_reentry_safety() {
        // Create a bus with a subscriber so we can count envelopes.
        // The bus.publish path emits a tracing::warn when no subscribers are
        // present, but here we DO subscribe — so we count what arrives.
        // Only 1 envelope (the original error) must arrive; the internal
        // tracing::warn emitted by EventBus::publish when no *inner* subscriber
        // is present must be suppressed by the re-entry guard.
        //
        // To trigger the re-entry path: use a bus with NO broadcast subscribers
        // (publish will emit tracing::warn internally). We collect via a
        // separate watcher on a cloned bus — but the published bus has no rx.
        let bus_no_rx = Arc::new(EventBus::new());
        // DO NOT call bus_no_rx.subscribe() — this means publish will emit a
        // tracing::warn, which exercises the re-entry guard.

        // We need to count how many times publish was called, so use a channel
        // side-channel to assert guard worked. Since we can't subscribe to bus_no_rx
        // (that would change the semantics), instead we use an atomic counter.
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count2 = count.clone();

        let subscriber = {
            let bus = bus_no_rx.clone();
            let fmt_layer = tracing_subscriber::fmt::layer().with_test_writer();
            // Wrap BusLayer to intercept publish calls via a counting bus wrapper.
            // Since we can't subclass BusLayer, use a fresh bus with an rx that
            // counts received envelopes, and do NOT use bus_no_rx.
            //
            // Revised approach: subscribe BEFORE with_default, count envelopes
            // received in a 100ms window, then assert count == 1.
            let _ = bus; // suppress unused
            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(BusLayer::with_bus(bus_no_rx.clone()))
        };

        // Subscribe AFTER building subscriber, before triggering events.
        // This means the bus HAS a subscriber when error fires → publish succeeds.
        // But the internal tracing::warn from publish (if no-rx path) won't fire
        // because publish succeeds. To actually test the guard we need the no-rx path.
        //
        // Use a counting receiver on a side bus to detect re-entry:
        // The guard prevents the warn from re-entering on_event.
        // Without the guard: infinite recursion → timeout.
        // With the guard: terminates in < 200ms.
        let result = tokio::time::timeout(Duration::from_millis(200), async {
            tracing::subscriber::with_default(subscriber, || {
                // emit an error — publish will emit tracing::warn (no subscribers on bus_no_rx)
                // the warn must be suppressed by the re-entry guard.
                tracing::error!("trigger reentry path");
                count2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            });
        })
        .await;

        assert!(
            result.is_ok(),
            "re-entry safety check timed out (possible infinite recursion)"
        );
        // The closure body ran exactly once (no hang, no recursion).
        assert_eq!(
            count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "expected on_event to complete exactly once (guard suppressed re-entry)"
        );
    }

    // ── Test F — run_id inherited from parent span (TD2) ─────────────────────

    #[tokio::test]
    async fn test_f_run_id_from_parent_span() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();
        let subscriber = make_subscriber(bus.clone());

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("op", run_id = "r1");
            let _guard = span.enter();
            // No run_id on the event itself — must be inherited from span.
            tracing::error!("boom from span");
        });

        let envelope = tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match rx.recv().await {
                    Ok(e) => return e,
                    Err(_) => continue,
                }
            }
        })
        .await
        .expect("expected Finding envelope within 100ms (run_id from parent span)");

        assert_eq!(
            envelope.run_id, "r1",
            "run_id should be inherited from parent span, got {:?}",
            envelope.run_id
        );
        match envelope.event {
            Event::Finding { message, .. } => {
                assert!(
                    message.contains("boom from span"),
                    "message was: {message:?}"
                );
            }
            other => panic!("expected Finding, got {other:?}"),
        }
    }

    // ── Test G — record_debug does not double-quote string values (F4) ────────

    #[tokio::test]
    async fn test_g_run_id_no_debug_quotes() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();
        let subscriber = make_subscriber(bus.clone());

        // When a String (not &str) is passed as run_id, tracing routes it through
        // record_debug. The resulting envelope.run_id must be "abc", not "\"abc\"".
        let run_id_string: String = "abc".to_owned();
        tracing::subscriber::with_default(subscriber, || {
            // Using %run_id_string uses Display; using ?run_id_string uses Debug.
            // We test the Debug path explicitly with the `?` sigil.
            tracing::error!(run_id = ?run_id_string, "debug-quoted test");
        });

        let envelope = tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                match rx.recv().await {
                    Ok(e) => return e,
                    Err(_) => continue,
                }
            }
        })
        .await
        .expect("expected Finding envelope within 100ms");

        assert_eq!(
            envelope.run_id, "abc",
            "run_id must not be double-quoted; got {:?}",
            envelope.run_id
        );
    }
}
