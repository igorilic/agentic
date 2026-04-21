use agentic_core::logging;

/// Calling init_test_subscriber twice in the same process must not panic.
#[test]
fn idempotent_init_test_subscriber_does_not_panic_on_second_call() {
    logging::init_test_subscriber();
    logging::init_test_subscriber();
}

/// A log emitted after init_test_subscriber is captured by tracing_test.
#[tracing_test::traced_test]
#[test]
fn init_test_subscriber_captures_emitted_logs() {
    logging::init_test_subscriber();
    tracing::info!("hello from agentic-core");
    assert!(logs_contain("hello from agentic-core"));
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
