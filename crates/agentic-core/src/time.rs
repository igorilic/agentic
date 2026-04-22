use std::time::{SystemTime, UNIX_EPOCH};

/// Unix epoch millis as `i64`. Saturates to 0 if the clock is before 1970
/// (extremely unlikely; matches the defensive fallback already used by the
/// migrator's seconds-granularity timestamp).
///
/// Note: the migrator uses `as_secs()` (seconds granularity); workspace
/// timestamps intentionally use millis for finer-grained ordering.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
