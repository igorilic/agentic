use agentic_core::logging;

/// Calling init_test_subscriber twice in the same process must not panic.
#[test]
fn idempotent_init_test_subscriber_does_not_panic_on_second_call() {
    logging::init_test_subscriber();
    logging::init_test_subscriber();
}

/// Calling init() then init_test_subscriber() (or vice-versa) must not panic.
/// The second call is a guaranteed no-op; which subscriber "wins" is not asserted
/// because test execution order is nondeterministic.
#[test]
fn cross_call_init_then_test_subscriber_does_not_panic() {
    agentic_core::init(Some("info"));
    agentic_core::init_test_subscriber();
    // Reverse order can't be tested in the same binary because OnceLock is per-process
    // and the previous test/test-order is nondeterministic; the key invariant is no panic.
}

/// resolved_filter honors explicit arg > AGENTIC_LOG env > default "info".
#[test]
fn init_honors_agentic_log_env_var() {
    // Explicit arg wins over env var.
    // SAFETY: test-only single-threaded env mutation; no concurrent env reads.
    unsafe { std::env::set_var("AGENTIC_LOG", "warn") };
    let filter = logging::resolved_filter(Some("debug"));
    assert_eq!(filter, "debug");

    // Env var wins over default.
    let filter = logging::resolved_filter(None);
    assert_eq!(filter, "warn");

    // SAFETY: test-only single-threaded env mutation.
    unsafe { std::env::remove_var("AGENTIC_LOG") };

    // Default when nothing set.
    let filter = logging::resolved_filter(None);
    assert_eq!(filter, "info");
}
