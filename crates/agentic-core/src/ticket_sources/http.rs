use std::time::Duration;

use reqwest::Client;

/// Build a fresh `reqwest::Client` with sensible defaults for ticket-source
/// calls (30s timeout, branded user-agent). Each `TicketSource::new` call
/// constructs its own client; the connection pool is per-source-instance,
/// not shared across sources. For MVP that's fine — we expect one source
/// per pipeline run. If pool sharing matters in the future, hoist this to
/// a `OnceLock<Client>` singleton.
pub(crate) fn shared_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("agentic-core/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest::Client::build with default settings cannot fail")
}
