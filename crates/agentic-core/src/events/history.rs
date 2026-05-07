use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::events::{EventBus, EventEnvelope};

/// Default per-run envelope cap. Larger than the webview's display buffer
/// (MAX_EVENTS = 500 in useTauriEvents) to ensure replay covers any
/// realistic mid-run attach window.
pub const DEFAULT_HISTORY_CAP: usize = 1000;

/// Default maximum number of distinct run_ids retained in the buffer.
///
/// Memory worst-case: 32 runs × 1 000 envelopes × ~500 B ≈ 16 MB resident
/// in a long-lived Tauri session. Beyond this cap the least-recently-published
/// run is evicted. See GH #64.
pub const DEFAULT_RUNS_CAP: usize = 32;

struct Inner {
    by_run: HashMap<String, Vec<EventEnvelope>>,
    /// Per-run sliding-window size.
    cap: usize,
    /// Maximum number of distinct run_ids retained. When exceeded, the
    /// least-recently-touched run is removed from `by_run`.
    runs_cap: usize,
    /// LRU ordering: front = oldest (least recently touched), back = newest.
    /// Only write-side publishes update this ordering; `get` is read-only
    /// and must NOT touch lru_order (see `get` doc comment).
    lru_order: VecDeque<String>,
}

/// Per-run ring buffer of recent envelopes. Subscribes to the bus and
/// records each envelope under its run_id, capped at `cap` per run.
/// The map itself is bounded to `runs_cap` distinct run_ids; when a new
/// run exceeds the cap the least-recently-published run is evicted (LRU).
///
/// On replay (e.g., webview reattach), the buffer's `get(run_id)`
/// returns the recorded prefix in order.
pub struct EventHistoryBuffer {
    inner: Arc<Mutex<Inner>>,
    /// Spawned subscriber task. Kept for graceful shutdown if needed.
    _subscriber: JoinHandle<()>,
}

impl EventHistoryBuffer {
    /// Spawn with explicit per-run envelope cap, defaulting to
    /// [`DEFAULT_RUNS_CAP`] for the run-map size.
    pub fn spawn(bus: &EventBus, cap: usize) -> Self {
        Self::spawn_with_runs_cap(bus, cap, DEFAULT_RUNS_CAP)
    }

    /// Spawn with explicit per-run envelope cap AND explicit runs-map cap.
    /// Prefer `spawn(bus, cap)` or `spawn_default(bus)` for typical call sites.
    pub fn spawn_with_runs_cap(bus: &EventBus, cap: usize, runs_cap: usize) -> Self {
        let inner = Arc::new(Mutex::new(Inner {
            by_run: HashMap::new(),
            cap,
            runs_cap,
            lru_order: VecDeque::new(),
        }));
        let inner_clone = inner.clone();
        let mut rx = bus.subscribe();
        let subscriber = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(envelope) => {
                        let mut guard = inner_clone.lock().await;
                        let cap = guard.cap;
                        let runs_cap = guard.runs_cap;
                        let run_id = envelope.run_id.clone();

                        // Touch: remove prior position and push to back (most-recent).
                        guard.lru_order.retain(|id| id != &run_id);
                        guard.lru_order.push_back(run_id.clone());

                        // Insert/append with per-run sliding window.
                        let entry = guard.by_run.entry(run_id).or_default();
                        entry.push(envelope);
                        if entry.len() > cap {
                            let drop_count = entry.len() - cap;
                            entry.drain(..drop_count);
                        }

                        // Evict LRU runs while over the map cap.
                        while guard.by_run.len() > runs_cap {
                            if let Some(oldest) = guard.lru_order.pop_front() {
                                guard.by_run.remove(&oldest);
                            } else {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "EventHistoryBuffer: lagged");
                    }
                }
            }
        });
        Self {
            inner,
            _subscriber: subscriber,
        }
    }

    /// Spawn using both [`DEFAULT_HISTORY_CAP`] and [`DEFAULT_RUNS_CAP`].
    pub fn spawn_default(bus: &EventBus) -> Self {
        Self::spawn_with_runs_cap(bus, DEFAULT_HISTORY_CAP, DEFAULT_RUNS_CAP)
    }

    /// Snapshot of buffered envelopes for the given run_id. Returns empty
    /// vec if no envelopes for that run.
    ///
    /// **Read-only**: this method does NOT update the LRU order. Only
    /// publishing a new envelope for a run counts as a "touch". This
    /// ensures a replay query on the frontend does not accidentally
    /// prevent an idle run from being evicted.
    pub async fn get(&self, run_id: &str) -> Vec<EventEnvelope> {
        let guard = self.inner.lock().await;
        guard.by_run.get(run_id).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
impl EventHistoryBuffer {
    /// Test-only constructor: build a buffer from pre-seeded state without
    /// spawning a real subscriber on a real bus. Useful for asserting
    /// `get(...)` semantics in isolation.
    ///
    /// The placeholder JoinHandle is a no-op task so the field invariant
    /// (`_subscriber` is a live JoinHandle) is preserved.
    pub(crate) fn from_inner_for_test(
        by_run: HashMap<String, Vec<EventEnvelope>>,
        cap: usize,
        runs_cap: usize,
        lru_order: VecDeque<String>,
    ) -> Self {
        let inner = Arc::new(Mutex::new(Inner {
            by_run,
            cap,
            runs_cap,
            lru_order,
        }));
        let _subscriber = tokio::spawn(async {});
        Self { inner, _subscriber }
    }
}

#[cfg(test)]
fn mk_env(run_id: &str, event_id: &str) -> crate::events::EventEnvelope {
    use crate::events::Event;
    crate::events::EventEnvelope {
        schema_version: 1,
        event_id: event_id.to_string(),
        run_id: run_id.to_string(),
        step_id: None,
        timestamp_ms: 0,
        event: Event::TextDelta {
            content: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};

    use super::{EventHistoryBuffer, mk_env};

    /// U1 — `get` returns clones, preserves order.
    /// Build from pre-seeded state with a single run_id "r1" carrying 3 distinct
    /// envelopes. Assert length=3, order preserved, and calling twice yields
    /// equal vecs (clones don't drain).
    #[tokio::test]
    async fn get_returns_clones_preserves_order() {
        let mut by_run = HashMap::new();
        by_run.insert(
            "r1".to_string(),
            vec![mk_env("r1", "e1"), mk_env("r1", "e2"), mk_env("r1", "e3")],
        );
        let buf = EventHistoryBuffer::from_inner_for_test(
            by_run,
            1000,
            32,
            VecDeque::from(["r1".to_string()]),
        );

        let first = buf.get("r1").await;
        assert_eq!(first.len(), 3);
        assert_eq!(first[0].event_id, "e1");
        assert_eq!(first[1].event_id, "e2");
        assert_eq!(first[2].event_id, "e3");

        // Calling again yields equal results — clones don't drain.
        let second = buf.get("r1").await;
        assert_eq!(first, second);
    }

    /// U2 — `get` returns empty for unknown run_id.
    #[tokio::test]
    async fn get_returns_empty_for_unknown_run_id() {
        let mut by_run = HashMap::new();
        by_run.insert("r1".to_string(), vec![mk_env("r1", "e1")]);
        let buf = EventHistoryBuffer::from_inner_for_test(by_run, 1000, 32, VecDeque::new());

        let result = buf.get("r2").await;
        assert!(result.is_empty());
    }

    /// U3 — by_run vs lru_order out-of-sync is permitted but does not crash.
    /// The `get` path only consults `by_run`; divergent `lru_order` is irrelevant
    /// for reads.
    #[tokio::test]
    async fn get_tolerates_lru_order_divergence() {
        let mut by_run = HashMap::new();
        let env1 = mk_env("r1", "e1");
        by_run.insert("r1".to_string(), vec![env1.clone()]);
        // lru_order intentionally empty — out of sync with by_run.
        let buf = EventHistoryBuffer::from_inner_for_test(by_run, 1000, 32, VecDeque::new());

        let result = buf.get("r1").await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event_id, "e1");
    }
}
