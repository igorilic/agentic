//! Integration tests for Copilot CLI model list resolution.
//!
//! All async tests use fixture binaries so we never call the real copilot CLI.
//! Gated on unix because the fixtures are shell scripts.

#![cfg(feature = "testing")]
#![cfg(unix)]

use std::path::PathBuf;

use agentic_core::backends::copilot_cli::{
    models::{bundled_models, resolve_models},
    runner::CopilotRunner,
};

fn fixture_bin(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bin")
        .join(name)
}

#[test]
fn bundled_models_contains_expected_entries() {
    let models = bundled_models();
    assert!(models.iter().any(|m| m.0 == "claude-opus-4.6"));
    assert!(models.iter().any(|m| m.0 == "gpt-5.2"));
    assert!(!models.is_empty());
}

#[tokio::test]
async fn resolve_models_parses_flat_array_from_fake_copilot() {
    let runner = CopilotRunner::with_binary(fixture_bin("fake-copilot-models-list.sh"));
    let models = resolve_models(&runner).await;
    assert_eq!(
        models.iter().map(|m| m.0.clone()).collect::<Vec<_>>(),
        vec![
            "claude-opus-4.6".to_string(),
            "gpt-5".to_string(),
            "gpt-5.2".to_string()
        ]
    );
}

#[tokio::test]
async fn resolve_models_falls_back_on_nonzero_exit() {
    let runner = CopilotRunner::with_binary(fixture_bin("fake-copilot-models-fail.sh"));
    let models = resolve_models(&runner).await;
    // Should match bundled list exactly.
    assert_eq!(models, bundled_models());
}

#[tokio::test]
async fn resolve_models_falls_back_on_unparseable_output() {
    let runner = CopilotRunner::with_binary(fixture_bin("fake-copilot-models-garbage.sh"));
    let models = resolve_models(&runner).await;
    assert_eq!(models, bundled_models());
}

#[tokio::test]
async fn resolve_models_parses_object_with_models_array() {
    let runner = CopilotRunner::with_binary(fixture_bin("fake-copilot-models-object.sh"));
    let models = resolve_models(&runner).await;
    assert_eq!(
        models.iter().map(|m| m.0.clone()).collect::<Vec<_>>(),
        vec!["model-a".to_string(), "model-b".to_string()]
    );
}
