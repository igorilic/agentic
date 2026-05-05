use agentic_core::BackendKind;

#[test]
fn backend_kind_id_str_returns_stable_backend_id() {
    assert_eq!(BackendKind::ClaudeCode.id_str(), "claude-code");
    assert_eq!(BackendKind::CopilotCli.id_str(), "copilot-cli");
}

#[test]
fn backend_kind_parse_round_trips_id_str_values() {
    assert_eq!(
        BackendKind::parse("claude-code").unwrap(),
        BackendKind::ClaudeCode
    );
    assert_eq!(
        BackendKind::parse("copilot-cli").unwrap(),
        BackendKind::CopilotCli
    );
    let err = BackendKind::parse("scripted").unwrap_err();
    assert!(
        err.contains("claude-code"),
        "error should mention 'claude-code', got: {err}"
    );
    assert!(
        err.contains("copilot-cli"),
        "error should mention 'copilot-cli', got: {err}"
    );
}

#[test]
fn backend_kind_serializes_via_serde_to_id_str() {
    let val = serde_json::to_value(&BackendKind::ClaudeCode).unwrap();
    assert_eq!(val, serde_json::json!("claude-code"));

    let val = serde_json::to_value(&BackendKind::CopilotCli).unwrap();
    assert_eq!(val, serde_json::json!("copilot-cli"));
}

#[test]
fn backend_kind_deserializes_from_kebab_case_strings() {
    let kind: BackendKind = serde_json::from_value(serde_json::json!("claude-code")).unwrap();
    assert_eq!(kind, BackendKind::ClaudeCode);

    let kind: BackendKind = serde_json::from_value(serde_json::json!("copilot-cli")).unwrap();
    assert_eq!(kind, BackendKind::CopilotCli);
}
