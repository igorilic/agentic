use agentic_core::{CoreError, PipelineConfig};

#[test]
fn default_pipeline_parses_with_four_steps_in_order() {
    let toml_str = r#"
[pipelines.default]
steps = [
  { agent = "architect",      stop_on_failure = true,  allowed_questions = 5 },
  { agent = "tdd-developer",  stop_on_failure = true,  qa_fix_loop_cap = 3 },
  { agent = "qa",             stop_on_failure = false },
  { agent = "reviewer",       stop_on_failure = false },
]
"#;
    let config = PipelineConfig::parse_str(toml_str).expect("parse default");
    let default = config.default_pipeline();
    assert_eq!(default.steps.len(), 4);
    let agents: Vec<&str> = default.steps.iter().map(|s| s.agent.as_str()).collect();
    assert_eq!(agents, vec!["architect", "tdd-developer", "qa", "reviewer"]);
    assert_eq!(default.steps[0].allowed_questions, Some(5));
    assert_eq!(default.steps[0].stop_on_failure, true);
    assert_eq!(default.steps[1].qa_fix_loop_cap, Some(3));
    assert_eq!(default.steps[2].stop_on_failure, false);
    assert_eq!(default.steps[3].stop_on_failure, false);
}

#[test]
fn hotfix_pipeline_parses_alongside_default() {
    let toml_str = r#"
[pipelines.default]
steps = [{ agent = "architect", stop_on_failure = true }]

[pipelines.hotfix]
steps = [
  { agent = "troubleshooter", stop_on_failure = true },
  { agent = "tdd-developer",  stop_on_failure = true,  qa_fix_loop_cap = 1 },
  { agent = "qa",             stop_on_failure = false },
]
"#;
    let config = PipelineConfig::parse_str(toml_str).expect("parse");
    assert_eq!(config.pipelines.len(), 2);
    assert!(config.pipelines.contains_key("default"));
    let hotfix = config.pipelines.get("hotfix").expect("hotfix present");
    assert_eq!(hotfix.steps.len(), 3);
    assert_eq!(hotfix.steps[0].agent, "troubleshooter");
    assert_eq!(hotfix.steps[1].qa_fix_loop_cap, Some(1));
}

#[test]
fn missing_file_returns_builtin_default_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let config = PipelineConfig::load(tmp.path()).expect("load returns builtin default");
    let default = config.default_pipeline();
    assert_eq!(default.steps.len(), 4);
    let agents: Vec<&str> = default.steps.iter().map(|s| s.agent.as_str()).collect();
    assert_eq!(agents, vec!["architect", "tdd-developer", "qa", "reviewer"]);
    // Built-in default also sets the documented spec §10.4 parameters
    assert_eq!(default.steps[0].allowed_questions, Some(5));
    assert_eq!(default.steps[1].qa_fix_loop_cap, Some(3));
}

#[test]
fn unknown_top_level_key_returns_parse_error() {
    let toml_str = r#"
[pipelines.default]
steps = [{ agent = "architect", stop_on_failure = true }]

[unknown_section]
foo = "bar"
"#;
    let result = PipelineConfig::parse_str(toml_str);
    match result {
        Err(CoreError::Parse(msg)) => {
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("unknown") || lower.contains("unexpected") || lower.contains("deny"),
                "expected unknown-field error, got: {msg}"
            );
        }
        Ok(_) => panic!("expected Parse error for unknown top-level key, got Ok"),
        Err(other) => panic!("expected CoreError::Parse, got {other:?}"),
    }
}
