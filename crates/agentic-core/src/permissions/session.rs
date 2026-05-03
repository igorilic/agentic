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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Opaque run identifier. In the bus envelope this is the `run_id: String`
/// field. We alias it here for readability.
pub type RunId = str;

/// Inner map type: run_id → set of (tool, arg) pairs.
type AllowlistMap = Mutex<HashMap<String, HashSet<(String, String)>>>;

/// Per-run in-memory cache of user-approved `(tool, arg)` pairs.
///
/// Cheap to clone: the `Arc` is cloned, not the inner map.
#[derive(Debug, Clone, Default)]
pub struct SessionAllowlist {
    inner: Arc<AllowlistMap>,
}

impl SessionAllowlist {
    /// Create an empty allowlist.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a `(tool, arg)` pair under `run_id`.
    pub fn insert(&self, run_id: &RunId, tool: &str, arg: &str) {
        let mut guard = self.inner.lock().expect("SessionAllowlist mutex poisoned");
        guard
            .entry(run_id.to_string())
            .or_default()
            .insert((tool.to_string(), arg.to_string()));
    }

    /// Returns `true` if `(tool, arg)` was previously inserted under `run_id`.
    ///
    /// Matching is **exact-arg** — no glob or prefix logic.
    pub fn contains(&self, run_id: &RunId, tool: &str, arg: &str) -> bool {
        let guard = self.inner.lock().expect("SessionAllowlist mutex poisoned");
        guard
            .get(run_id)
            .map(|set| set.contains(&(tool.to_string(), arg.to_string())))
            .unwrap_or(false)
    }

    /// Remove all cached entries for `run_id`. Called when `Event::RunComplete`
    /// arrives for that run.
    pub fn drop_run(&self, run_id: &RunId) {
        let mut guard = self.inner.lock().expect("SessionAllowlist mutex poisoned");
        guard.remove(run_id);
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
}
