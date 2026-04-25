use std::time::{SystemTime, UNIX_EPOCH};

/// Unix epoch millis as `i64`.
///
/// Returns 0 if the system clock is before the Unix epoch (shouldn't happen
/// outside tests or misconfigured VMs).
///
/// # Overflow safety
/// `Duration::as_millis()` returns `u128`. We convert via `i64::try_from`
/// rather than the silent `as i64` truncation cast. The conversion will panic
/// only after the year 292_277_026_596 (≈ year 292 million), so the `expect`
/// is documentation, not a real runtime risk. SQLite stores this as INTEGER
/// (i64), so i64 is the correct type at the language boundary.
///
/// Note: the migrator uses `as_secs()` (seconds granularity); workspace
/// timestamps intentionally use millis for finer-grained ordering.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            i64::try_from(d.as_millis())
                .expect("timestamp millis fit in i64 for the next ~292 million years")
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_ms_returns_recent_timestamp() {
        let t = now_ms();
        // Should be > 2025-01-01 00:00:00 UTC in milliseconds.
        assert!(t > 1_735_689_600_000, "timestamp too small: {t}");
        // Sanity ceiling: should be < year 2100.
        assert!(t < 4_102_444_800_000, "timestamp too large: {t}");
    }
}
