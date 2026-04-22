use std::path::Path;

use agentic_core::{CoreError, PipelineRole, agents::parse_agent};

fn fixture(stem: &str) -> (String, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("agents")
        .join(format!("{stem}.md"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    (stem.to_string(), content)
}

#[test]
fn valid_frontmatter_parses_all_fields() {
    let (stem, content) = fixture("architect");
    let agent = parse_agent(&stem, &content).expect("parse");

    assert_eq!(agent.name, "architect");
    assert_eq!(agent.description, "Designs feature spec and produces atomic todo plans");
    assert_eq!(agent.model.as_deref(), Some("claude-opus-4-7"));
    assert_eq!(
        agent.tools.as_deref(),
        Some(
            &[
                "Read".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
                "Bash".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "WebSearch".to_string(),
                "WebFetch".to_string(),
            ][..]
        )
    );
    assert_eq!(agent.allowed_questions, Some(5));
    assert_eq!(agent.pipeline_role, PipelineRole::Step);
    assert_eq!(agent.timeout_seconds, Some(1800));
    assert!(
        agent.system_prompt.contains("You are the architect agent"),
        "system_prompt should contain the body: got '{}'",
        agent.system_prompt
    );
}

#[test]
fn missing_name_returns_parse_error() {
    let (stem, content) = fixture("missing_name");
    let result = parse_agent(&stem, &content);
    match result {
        Err(CoreError::Parse(msg)) => {
            // Serde's error mentions the missing field
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("name") || lower.contains("missing field"),
                "error should mention the missing 'name' field; got: {msg}"
            );
        }
        Ok(_) => panic!("expected Parse error for missing name, got Ok"),
        Err(other) => panic!("expected CoreError::Parse, got {other:?}"),
    }
}

#[test]
fn unknown_fields_are_ignored_forward_compat() {
    let (stem, content) = fixture("with_extras");
    let agent = parse_agent(&stem, &content).expect("parse despite unknown field");
    assert_eq!(agent.name, "with_extras");
    assert_eq!(agent.description, "Demonstrates unknown-field tolerance");
    assert_eq!(agent.model.as_deref(), Some("claude-sonnet-4-6"));
}

#[test]
fn pipeline_role_defaults_to_step_when_absent() {
    let (stem, content) = fixture("qa");
    let agent = parse_agent(&stem, &content).expect("parse minimal");
    assert_eq!(agent.pipeline_role, PipelineRole::Step);
    assert_eq!(agent.name, "qa");
    assert_eq!(agent.model, None);
    assert_eq!(agent.tools, None);
    assert_eq!(agent.allowed_questions, None);
    assert_eq!(agent.timeout_seconds, None);
}

#[test]
fn name_mismatch_with_filename_stem_returns_parse_error() {
    // File is name_mismatch.md but frontmatter says name: wrong_name
    let (stem, content) = fixture("name_mismatch");
    let result = parse_agent(&stem, &content);
    match result {
        Err(CoreError::Parse(msg)) => {
            assert!(
                msg.contains("name mismatch"),
                "error should say 'name mismatch'; got: {msg}"
            );
            assert!(
                msg.contains("wrong_name"),
                "error should mention the frontmatter name 'wrong_name'; got: {msg}"
            );
        }
        Ok(_) => panic!("expected name mismatch error, got Ok"),
        Err(other) => panic!("expected CoreError::Parse, got {other:?}"),
    }
}
