#![cfg(test)]

#[test]
#[should_panic(
    expected = "agentic-core logging: another tracing subscriber is already installed globally"
)]
fn init_panics_with_clear_message_when_foreign_subscriber_already_installed() {
    // Install a foreign subscriber FIRST.
    let subscriber = tracing_subscriber::registry();
    tracing::subscriber::set_global_default(subscriber)
        .expect("first set_global_default should succeed");

    // Now agentic_core::init should panic with an actionable message.
    agentic_core::logging::init(None);
}
