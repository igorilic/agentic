use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt};

static SUBSCRIBER_INSTALLED: OnceLock<()> = OnceLock::new();

/// Returns the resolved log filter string based on precedence:
/// explicit `filter` arg > `AGENTIC_LOG` env var > default `"info"`.
#[doc(hidden)]
pub fn resolved_filter(filter: Option<&str>) -> String {
    if let Some(f) = filter {
        return f.to_owned();
    }
    if let Ok(env_val) = std::env::var("AGENTIC_LOG") {
        if !env_val.is_empty() {
            return env_val;
        }
    }
    "info".to_owned()
}

/// Installs a global tracing subscriber configured for production use.
///
/// Only the **first** call in the process installs a subscriber. Subsequent
/// calls are no-ops; the `filter` argument on any call after the first is
/// **silently ignored**. If you need `init_test_subscriber`'s `with_test_writer()`
/// capture behaviour, ensure `init()` has not been called earlier in the same
/// process.
pub fn init(filter: Option<&str>) {
    SUBSCRIBER_INSTALLED.get_or_init(|| {
        let filter_str = resolved_filter(filter);
        let env_filter = EnvFilter::new(filter_str);
        fmt().with_env_filter(env_filter).init();
    });
}

/// Installs a test-friendly tracing subscriber with test writer.
///
/// Only the **first** call in the process installs a subscriber. Subsequent
/// calls are no-ops; if this is not the first call (e.g. because `init()` was
/// called earlier), the test-writer capture behaviour is **not** installed and
/// this call is **silently ignored**. To guarantee test-writer capture, ensure
/// no other `init`/`init_test_subscriber` call precedes this one in the process.
pub fn init_test_subscriber() {
    SUBSCRIBER_INSTALLED.get_or_init(|| {
        fmt()
            .with_test_writer()
            .with_env_filter(
                EnvFilter::try_from_env("AGENTIC_LOG")
                    .unwrap_or_else(|_| EnvFilter::new("debug")),
            )
            .init();
    });
}
