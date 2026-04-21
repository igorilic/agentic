use std::sync::OnceLock;

use tracing_subscriber::{EnvFilter, fmt};

static GLOBAL_SUBSCRIBER: OnceLock<()> = OnceLock::new();
static TEST_SUBSCRIBER: OnceLock<()> = OnceLock::new();

/// Returns the resolved log filter string based on precedence:
/// explicit `filter` arg > `AGENTIC_LOG` env var > default `"info"`.
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
/// Calling this function more than once in the same process is a no-op.
pub fn init(filter: Option<&str>) {
    GLOBAL_SUBSCRIBER.get_or_init(|| {
        let filter_str = resolved_filter(filter);
        let env_filter = EnvFilter::new(filter_str);
        fmt().with_env_filter(env_filter).init();
    });
}

/// Installs a test-friendly tracing subscriber with test writer.
/// Calling this function more than once in the same process is a no-op.
pub fn init_test_subscriber() {
    TEST_SUBSCRIBER.get_or_init(|| {
        let _ = fmt()
            .with_test_writer()
            .with_env_filter(EnvFilter::new("debug"))
            .try_init();
    });
}
