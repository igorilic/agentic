use std::time::Duration;

use reqwest::Client;

/// Shared HTTP client with sensible defaults for ticket-source calls.
/// Reused across github / gitlab / jira to avoid creating multiple
/// connection pools.
pub(crate) fn shared_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("agentic-core/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest::Client::build with default settings cannot fail")
}
