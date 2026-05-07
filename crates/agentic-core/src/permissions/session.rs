//! Per-run session allowlist for the permission gate.
//!
//! `SessionAllowlist` tracks `(tool, exact-arg)` pairs that the user approved
//! with `Decision::AllowSession` for a given [`RunId`]. When the same
//! `(tool, arg)` pair is presented again within the same run, the gate
//! short-circuits and returns `AnnotateAllow { source: SessionAllowlist }`
//! without any bus interaction.
//!
//! # Scope and lifetime
//! - **Per-run**: each run_id has its own set of cached patterns. Two
//!   concurrent runs never share session state.
//! - **In-memory only**: no persistence. Cache is lost on process restart.
//! - **Exact-arg match** (Q2 minimality): `("Bash", "ls -la")` does NOT
//!   match `("Bash", "ls -la /tmp")`. No glob, prefix, or regex matching.
//! - **Cleared on `RunComplete`**: the async gate subscribes to the bus and
//!   calls [`SessionAllowlist::drop_run`] when `Event::RunComplete` arrives
//!   for that run.
//! - **Bounded map**: at most [`DEFAULT_RUNS_CAP`] distinct run_ids are
//!   retained. When a new run pushes the count over the cap, the least-recently
//!   inserted-to run is evicted. This bounds memory in long-lived Tauri sessions
//!   where runs exit abnormally and `RunComplete` is never emitted. See GH #102.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// Opaque run identifier. In the bus envelope this is the `run_id: String`
/// field. We alias it here for readability.
pub type RunId = str;

/// Default maximum number of distinct run_ids retained in the allowlist.
///
/// Memory worst-case: 64 runs × 10 patterns × ~100 B per entry ≈ 64 KB
/// resident in a long-lived Tauri session. Beyond this cap the least-recently
/// inserted-to run is evicted. Each entry is small (two short strings in a
/// HashSet), so 64 is generous. See GH #102.
pub const DEFAULT_RUNS_CAP: usize = 64;

/// Shared mutable state behind the `SessionAllowlist` `Arc`.
///
/// LRU invariant: `lru_order.front()` = oldest (least recently touched),
/// `lru_order.back()` = newest. Only `insert` updates the ordering; `contains`
/// is read-only and must NOT touch `lru_order` (see `contains` doc comment).
#[derive(Debug)]
struct Inner {
    /// run_id → set of approved (tool, arg) pairs.
    by_run: HashMap<String, HashSet<(String, String)>>,
    /// LRU ordering: front = oldest, back = newest (most recently inserted to).
    lru_order: VecDeque<String>,
    /// Maximum number of distinct run_ids retained. When exceeded, the oldest
    /// entry (front of `lru_order`) is evicted.
    runs_cap: usize,
}

/// Per-run in-memory cache of user-approved `(tool, arg)` pairs.
///
/// Cheap to clone: the `Arc` is cloned, not the inner state.
///
/// The map is bounded to [`DEFAULT_RUNS_CAP`] distinct run_ids (or a custom
/// cap via [`SessionAllowlist::with_runs_cap`]). When a new run pushes the
/// count over the cap, the least-recently-inserted-to run is evicted so that
/// abnormal process exits that skip `RunComplete` do not leak indefinitely.
#[derive(Debug, Clone)]
pub struct SessionAllowlist {
    inner: Arc<Mutex<Inner>>,
}

impl Default for SessionAllowlist {
    fn default() -> Self {
        Self::with_runs_cap(DEFAULT_RUNS_CAP)
    }
}

impl SessionAllowlist {
    /// Create an empty allowlist with [`DEFAULT_RUNS_CAP`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty allowlist with an explicit run-map cap.
    ///
    /// Prefer [`SessionAllowlist::new`] for typical call sites. This
    /// constructor exists for tests and future callers that need a custom cap.
    pub fn with_runs_cap(runs_cap: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                by_run: HashMap::new(),
                lru_order: VecDeque::new(),
                runs_cap,
            })),
        }
    }

    /// Insert a `(tool, arg)` pair under `run_id`.
    ///
    /// Touches the LRU order: `run_id` is moved to the back (most recent).
    /// If the resulting map size exceeds the cap, the oldest run is evicted.
    pub fn insert(&self, run_id: &RunId, tool: &str, arg: &str) {
        let mut guard = self.inner.lock().expect("SessionAllowlist mutex poisoned");

        // Touch: remove prior position then push to back (most-recent).
        guard.lru_order.retain(|id| id != run_id);
        guard.lru_order.push_back(run_id.to_string());

        // Insert into the by_run map.
        guard
            .by_run
            .entry(run_id.to_string())
            .or_default()
            .insert((tool.to_string(), arg.to_string()));

        // Evict LRU runs while over the map cap.
        let runs_cap = guard.runs_cap;
        while guard.by_run.len() > runs_cap {
            if let Some(oldest) = guard.lru_order.pop_front() {
                guard.by_run.remove(&oldest);
            } else {
                break;
            }
        }
    }

    /// Returns `true` if `(tool, arg)` was previously inserted under `run_id`.
    ///
    /// Matching is **exact-arg** — no glob or prefix logic.
    ///
    /// **Read-only**: this method does NOT update the LRU order. Only
    /// `insert` counts as a "touch". This ensures a `contains` call on the
    /// hot path does not accidentally prevent an idle run from being evicted.
    ///
    /// # Allocation note
    ///
    /// The `(tool, arg)` strings are only heap-allocated when the `run_id`
    /// bucket exists. The most common case on the hot path is that no session
    /// entries have been inserted for the run (cold run or no `AllowSession`
    /// decisions yet), so this early-return avoids both allocations entirely.
    pub fn contains(&self, run_id: &RunId, tool: &str, arg: &str) -> bool {
        let guard = self.inner.lock().expect("SessionAllowlist mutex poisoned");
        let Some(set) = guard.by_run.get(run_id) else {
            // Most common case: no bucket for this run_id — skip both allocations.
            return false;
        };
        // Only allocate the owned tuple when we have a bucket to consult.
        set.contains(&(tool.to_string(), arg.to_string()))
    }

    /// Remove all cached entries for `run_id`. Called when `Event::RunComplete`
    /// arrives for that run.
    ///
    /// Also removes `run_id` from `lru_order` so no phantom entry remains to
    /// interfere with eviction accounting.
    pub fn drop_run(&self, run_id: &RunId) {
        let mut guard = self.inner.lock().expect("SessionAllowlist mutex poisoned");
        guard.by_run.remove(run_id);
        guard.lru_order.retain(|id| id != run_id);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // -----------------------------------------------------------------------
    // Test S.1: insert then contains — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn insert_then_contains_returns_true() {
        let al = SessionAllowlist::new();
        al.insert("run-1", "Bash", "ls -la");
        assert!(al.contains("run-1", "Bash", "ls -la"));
    }

    // -----------------------------------------------------------------------
    // Test S.2: unknown pair returns false
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_pair_returns_false() {
        let al = SessionAllowlist::new();
        al.insert("run-1", "Bash", "ls -la");
        // Different arg — exact match only.
        assert!(!al.contains("run-1", "Bash", "ls -la /tmp"));
        // Different tool.
        assert!(!al.contains("run-1", "Read", "ls -la"));
        // Completely unknown run.
        assert!(!al.contains("run-2", "Bash", "ls -la"));
    }

    // -----------------------------------------------------------------------
    // Test S.3: drop_run removes only the targeted run_id
    // -----------------------------------------------------------------------

    #[test]
    fn drop_run_removes_only_targeted_run() {
        let al = SessionAllowlist::new();
        al.insert("run-1", "Bash", "ls");
        al.insert("run-2", "Bash", "ls");

        al.drop_run("run-1");

        assert!(
            !al.contains("run-1", "Bash", "ls"),
            "run-1 should be cleared"
        );
        assert!(
            al.contains("run-2", "Bash", "ls"),
            "run-2 must be unaffected"
        );
    }

    // -----------------------------------------------------------------------
    // Test S.4: drop_run on unknown run_id is a no-op (no panic)
    // -----------------------------------------------------------------------

    #[test]
    fn drop_run_on_unknown_run_is_noop() {
        let al = SessionAllowlist::new();
        // Must not panic.
        al.drop_run("does-not-exist");
    }

    // -----------------------------------------------------------------------
    // Test S.5: concurrent inserts from two threads do not deadlock
    // -----------------------------------------------------------------------

    #[test]
    fn concurrent_inserts_do_not_deadlock() {
        let al = SessionAllowlist::new();
        let al2 = al.clone();

        let h1 = thread::spawn(move || {
            for i in 0..100 {
                al.insert("run-thread-1", "Bash", &format!("cmd-{i}"));
            }
        });
        let h2 = thread::spawn(move || {
            for i in 0..100 {
                al2.insert("run-thread-2", "Bash", &format!("cmd-{i}"));
            }
        });

        h1.join().expect("thread 1 panicked");
        h2.join().expect("thread 2 panicked");
    }

    // -----------------------------------------------------------------------
    // Test S.6: bounded map evicts oldest run when cap exceeded
    // -----------------------------------------------------------------------

    #[test]
    fn bounded_map_evicts_oldest_run() {
        let al = SessionAllowlist::with_runs_cap(3);
        al.insert("r1", "Bash", "cmd1");
        al.insert("r2", "Bash", "cmd2");
        al.insert("r3", "Bash", "cmd3");
        // r4 pushes us over the cap of 3; r1 (oldest) must be evicted.
        al.insert("r4", "Bash", "cmd4");

        assert!(!al.contains("r1", "Bash", "cmd1"), "r1 should be evicted");
        assert!(al.contains("r2", "Bash", "cmd2"), "r2 must survive");
        assert!(al.contains("r3", "Bash", "cmd3"), "r3 must survive");
        assert!(al.contains("r4", "Bash", "cmd4"), "r4 must survive");
    }

    // -----------------------------------------------------------------------
    // Test S.7: LRU touch on insert promotes run to back
    // -----------------------------------------------------------------------

    #[test]
    fn lru_touch_on_insert_promotes_run_to_back() {
        let al = SessionAllowlist::with_runs_cap(2);
        al.insert("r1", "Bash", "cmd1");
        al.insert("r2", "Bash", "cmd2");
        // Second insert for r1 touches it — moves r1 to back, so r2 is now oldest.
        al.insert("r1", "Bash", "cmd1b");
        // r3 pushes us over cap; r2 (now oldest) must be evicted.
        al.insert("r3", "Bash", "cmd3");

        assert!(
            !al.contains("r2", "Bash", "cmd2"),
            "r2 should be evicted (was LRU)"
        );
        assert!(
            al.contains("r1", "Bash", "cmd1"),
            "r1 must survive (was touched)"
        );
        assert!(
            al.contains("r1", "Bash", "cmd1b"),
            "r1 second pair must survive"
        );
        assert!(al.contains("r3", "Bash", "cmd3"), "r3 must survive");
    }

    // -----------------------------------------------------------------------
    // Test S.8: contains does NOT touch LRU order
    // -----------------------------------------------------------------------

    #[test]
    fn contains_does_not_touch_lru_order() {
        let al = SessionAllowlist::with_runs_cap(2);
        al.insert("r1", "Bash", "cmd1");
        al.insert("r2", "Bash", "cmd2");
        // Calling contains on r1 must NOT count as a touch.
        let _ = al.contains("r1", "Bash", "cmd1");
        // r3 pushes us over cap; r1 (still oldest, contains didn't touch) is evicted.
        al.insert("r3", "Bash", "cmd3");

        assert!(
            !al.contains("r1", "Bash", "cmd1"),
            "r1 should be evicted (contains did not touch)"
        );
        assert!(al.contains("r2", "Bash", "cmd2"), "r2 must survive");
        assert!(al.contains("r3", "Bash", "cmd3"), "r3 must survive");
    }

    // -----------------------------------------------------------------------
    // Test S.9: drop_run removes from lru_order (no phantom entry)
    // -----------------------------------------------------------------------

    #[test]
    fn drop_run_removes_from_lru_order() {
        let al = SessionAllowlist::with_runs_cap(2);
        al.insert("r1", "Bash", "cmd1");
        al.insert("r2", "Bash", "cmd2");
        // drop_run("r1") frees one slot; cap is now 1 used of 2.
        al.drop_run("r1");
        // r3 fits without eviction (1 used → 2 used, still ≤ cap).
        al.insert("r3", "Bash", "cmd3");
        // r4 now pushes to 3 > cap=2, evicting r2 (oldest remaining).
        al.insert("r4", "Bash", "cmd4");

        assert!(!al.contains("r1", "Bash", "cmd1"), "r1 was dropped");
        assert!(!al.contains("r2", "Bash", "cmd2"), "r2 should be evicted");
        assert!(al.contains("r3", "Bash", "cmd3"), "r3 must survive");
        assert!(al.contains("r4", "Bash", "cmd4"), "r4 must survive");
    }

    // -----------------------------------------------------------------------
    // Test S.10: default cap is 64 — 65th run evicts exactly one entry
    // -----------------------------------------------------------------------

    #[test]
    fn default_cap_is_64() {
        assert_eq!(DEFAULT_RUNS_CAP, 64, "DEFAULT_RUNS_CAP must be 64");

        let al = SessionAllowlist::new();
        // Insert 64 runs — all must fit.
        for i in 0..64 {
            al.insert(&format!("run-{i}"), "Bash", "x");
        }
        for i in 0..64 {
            assert!(
                al.contains(&format!("run-{i}"), "Bash", "x"),
                "run-{i} must survive within cap"
            );
        }
        // 65th run evicts run-0 (the oldest).
        al.insert("run-64", "Bash", "x");
        assert!(
            !al.contains("run-0", "Bash", "x"),
            "run-0 should be evicted by the 65th insert"
        );
        assert!(al.contains("run-64", "Bash", "x"), "run-64 must be present");
    }
}
