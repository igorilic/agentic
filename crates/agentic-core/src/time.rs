use std::time::{SystemTime, UNIX_EPOCH};

/// Unix epoch millis as `i64`. Saturates to 0 if the clock is before 1970
/// (extremely unlikely; matches the defensive fallback already used by the
/// migrator's seconds-granularity timestamp).
pub fn now_ms() -> i64 {
    unimplemented!()
}
