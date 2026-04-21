/// A log emitted inside a traced_test span is captured.
/// This test lives in its own binary to avoid subscriber conflicts with
/// the idempotent_init_test_subscriber test in logging.rs.
#[tracing_test::traced_test]
#[test]
fn init_test_subscriber_captures_emitted_logs() {
    tracing::info!("hello from agentic-core");
    assert!(logs_contain("hello from agentic-core"));
}
