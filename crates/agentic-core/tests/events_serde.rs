use agentic_core::{
    ActionRequired, BackendId, CURRENT_SCHEMA_VERSION, Event, EventEnvelope, ModelId,
    PermissionDecision, PermissionRisk, PermissionSource, ProfileId, RunStatus, Severity,
    StepStatus, TicketKind, TicketRef, TokenUsage, ToolStream,
};

fn sample_events() -> Vec<Event> {
    vec![
        Event::RunStarted {
            ticket: TicketRef {
                kind: TicketKind::GithubIssue,
                reference: "#42".to_string(),
                title: Some("Add OAuth flow".to_string()),
            },
            profile: ProfileId("github".to_string()),
            backend: BackendId("claude-code".to_string()),
            model: ModelId("claude-opus-4-7".to_string()),
        },
        Event::RunComplete {
            status: RunStatus::Completed,
            duration_ms: 12345,
            summary: "done".to_string(),
        },
        Event::StepStarted {
            agent: "architect".to_string(),
            model: ModelId("claude-opus-4-7".to_string()),
        },
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "passed".to_string(),
            token_usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_input_tokens: 10,
                cache_creation_input_tokens: 5,
            },
            cost_usd: Some(0.012),
            duration_ms: 400,
        },
        Event::TextDelta {
            content: "hello".to_string(),
        },
        Event::ThinkingDelta {
            content: "pondering".to_string(),
        },
        Event::ToolUseStart {
            tool_call_id: "tc1".to_string(),
            tool_name: "Bash".to_string(),
            input: serde_json::json!({ "cmd": "ls" }),
        },
        Event::ToolUseDelta {
            tool_call_id: "tc1".to_string(),
            stream: ToolStream::Stdout,
            content: "file.txt".to_string(),
        },
        Event::ToolUseEnd {
            tool_call_id: "tc1".to_string(),
            exit_code: Some(0),
            duration_ms: 100,
        },
        Event::FileChange {
            path: "src/foo.rs".into(),
            before_hash: "deadbeef".to_string(),
            after_hash: "cafebabe".to_string(),
        },
        Event::Finding {
            finding_id: "f1".to_string(),
            severity: Severity::Warning,
            file: Some("src/foo.rs".into()),
            line: Some(42),
            message: "msg".to_string(),
            suggestion: Some("fix".to_string()),
        },
        Event::ClarifyingQuestion {
            question_id: "q1".to_string(),
            question: "why?".to_string(),
            suggested_answers: vec!["because".to_string()],
        },
        Event::RetryStarted {
            attempt: 2,
            reason: "rate limited".to_string(),
        },
        Event::Error {
            code: "rate_limited".to_string(),
            message: "429".to_string(),
            recoverable: true,
            retry_after_ms: Some(5000),
        },
        Event::UserActionNeeded {
            action: ActionRequired::TriageFindings {
                finding_ids: vec!["f1".to_string()],
            },
        },
    ]
}

#[test]
fn every_event_variant_roundtrips_through_json() {
    for original in sample_events() {
        let envelope = EventEnvelope::now(
            "run1".to_string(),
            Some("step1".to_string()),
            original.clone(),
        );
        let json = serde_json::to_string(&envelope).expect("serialize");
        let back: EventEnvelope = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, envelope, "roundtrip mismatch for {original:?}");
    }
}

#[test]
fn run_started_deserializes_from_fixture() {
    let fixture = include_str!("fixtures/events/run_started.json");
    let envelope: EventEnvelope = serde_json::from_str(fixture).expect("fixture deserializes");
    match &envelope.event {
        Event::RunStarted {
            ticket,
            profile,
            backend,
            model,
        } => {
            assert_eq!(ticket.kind, TicketKind::GithubIssue);
            assert_eq!(ticket.reference, "#42");
            assert_eq!(profile.0, "github");
            assert_eq!(backend.0, "claude-code");
            assert_eq!(model.0, "claude-opus-4-7");
        }
        other => panic!("expected RunStarted, got {other:?}"),
    }
}

#[test]
fn event_id_is_a_valid_ulid() {
    let envelope = EventEnvelope::now(
        "run1".to_string(),
        None,
        Event::TextDelta {
            content: "x".to_string(),
        },
    );
    ulid::Ulid::from_string(&envelope.event_id).expect("event_id parses as ULID");
}

#[test]
fn timestamp_ms_is_monotonic_across_consecutive_now_calls() {
    let a = EventEnvelope::now(
        "run1".to_string(),
        None,
        Event::TextDelta {
            content: "a".to_string(),
        },
    );
    // 2ms sleep guarantees distinct millis on fast machines.
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = EventEnvelope::now(
        "run1".to_string(),
        None,
        Event::TextDelta {
            content: "b".to_string(),
        },
    );
    assert!(
        b.timestamp_ms > a.timestamp_ms,
        "expected b.timestamp_ms ({}) > a.timestamp_ms ({})",
        b.timestamp_ms,
        a.timestamp_ms
    );
}

#[test]
fn retry_started_envelope_serializes_to_exact_wire_format() {
    // Regression guard: pins the JSON shape produced by serde's
    // (tag = "type", content = "data", rename_all = "PascalCase").
    // Any change to serde attributes on Event or EventEnvelope that
    // alters the wire format will fail this test.
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 1234567890,
        event: Event::RetryStarted {
            attempt: 2,
            reason: "rate limited".to_string(),
        },
    };
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert_eq!(
        json,
        r#"{"schema_version":1,"event_id":"01J8RZYX1K3PQXGT1WJYR8AZ7Q","run_id":"run1","step_id":null,"timestamp_ms":1234567890,"event":{"type":"RetryStarted","data":{"attempt":2,"reason":"rate limited"}}}"#
    );
}

#[test]
fn step_complete_envelope_serializes_to_exact_wire_format() {
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: Some("step1".to_string()),
        timestamp_ms: 1234567890,
        event: Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_input_tokens: 10,
                cache_creation_input_tokens: 5,
            },
            cost_usd: Some(0.012),
            duration_ms: 400,
        },
    };
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert_eq!(
        json,
        r#"{"schema_version":1,"event_id":"01J8RZYX1K3PQXGT1WJYR8AZ7Q","run_id":"run1","step_id":"step1","timestamp_ms":1234567890,"event":{"type":"StepComplete","data":{"status":"passed","summary":"ok","token_usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":10,"cache_creation_input_tokens":5},"cost_usd":0.012,"duration_ms":400}}}"#
    );
}

#[test]
fn finding_envelope_serializes_to_exact_wire_format() {
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: Some("step1".to_string()),
        timestamp_ms: 1234567890,
        event: Event::Finding {
            finding_id: "f1".to_string(),
            severity: Severity::Warning,
            file: Some(std::path::PathBuf::from("src/lib.rs")),
            line: Some(42),
            message: "watch out".to_string(),
            suggestion: Some("use ?".to_string()),
        },
    };
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert_eq!(
        json,
        r#"{"schema_version":1,"event_id":"01J8RZYX1K3PQXGT1WJYR8AZ7Q","run_id":"run1","step_id":"step1","timestamp_ms":1234567890,"event":{"type":"Finding","data":{"finding_id":"f1","severity":"warning","file":"src/lib.rs","line":42,"message":"watch out","suggestion":"use ?"}}}"#
    );
}

#[test]
fn user_action_needed_envelope_serializes_to_exact_wire_format() {
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 1234567890,
        event: Event::UserActionNeeded {
            action: ActionRequired::TriageFindings {
                finding_ids: vec!["f1".to_string(), "f2".to_string()],
            },
        },
    };
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert_eq!(
        json,
        r#"{"schema_version":1,"event_id":"01J8RZYX1K3PQXGT1WJYR8AZ7Q","run_id":"run1","step_id":null,"timestamp_ms":1234567890,"event":{"type":"UserActionNeeded","data":{"action":{"type":"triage_findings","data":{"finding_ids":["f1","f2"]}}}}}"#
    );
}

// --- P.1.1: PermissionRequest + PermissionResolved ---

#[test]
fn permission_request_round_trips_msgpack() {
    let event = Event::PermissionRequest {
        request_id: "req-01JZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
        agent: "developer".to_string(),
        tool: "Bash".to_string(),
        arg: "rm -rf node_modules".to_string(),
        scope: "shell.destructive".to_string(),
        risk: PermissionRisk::High,
        reason: "destructive shell".to_string(),
    };
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 1234567890,
        event: event.clone(),
    };
    let bytes = rmp_serde::to_vec_named(&envelope).expect("msgpack encode");
    let decoded: EventEnvelope = rmp_serde::from_slice(&bytes).expect("msgpack decode");
    assert_eq!(
        decoded, envelope,
        "PermissionRequest must round-trip through MessagePack"
    );
}

#[test]
fn permission_resolved_round_trips_msgpack() {
    let event = Event::PermissionResolved {
        request_id: "req-01JZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
        decision: PermissionDecision::AllowOnce,
        source: PermissionSource::User,
    };
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 1234567890,
        event: event.clone(),
    };
    let bytes = rmp_serde::to_vec_named(&envelope).expect("msgpack encode");
    let decoded: EventEnvelope = rmp_serde::from_slice(&bytes).expect("msgpack decode");
    assert_eq!(
        decoded, envelope,
        "PermissionResolved must round-trip through MessagePack"
    );
}

// Issue 1: RunStarted must carry agents field and round-trip correctly
#[test]
fn event_run_started_serializes_agents_field() {
    let event = Event::RunStarted {
        ticket: TicketRef {
            kind: TicketKind::FreeText,
            reference: "test-run".to_string(),
            title: None,
        },
        profile: ProfileId("default".to_string()),
        backend: BackendId("claude-code".to_string()),
        model: ModelId("claude-sonnet-4-6".to_string()),
        agents: vec![
            "spec-writer".to_string(),
            "planner".to_string(),
            "implementer-tdd".to_string(),
            "reviewer".to_string(),
        ],
    };
    let envelope = EventEnvelope::now("run1".to_string(), None, event.clone());
    let json = serde_json::to_string(&envelope).expect("serialize");
    let back: EventEnvelope = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, envelope, "RunStarted with agents must round-trip through JSON");
    // Verify the agents field is present in the JSON output
    assert!(json.contains("\"agents\""), "JSON must contain 'agents' field");
    assert!(json.contains("\"spec-writer\""), "JSON must contain agent names");
}

#[test]
fn event_run_started_agents_defaults_to_empty_on_legacy_json() {
    // Simulates deserializing old JSON that lacks the agents field
    let legacy_json = r#"{
        "schema_version": 1,
        "event_id": "01J8RZYX1K3PQXGT1WJYR8AZ7Q",
        "run_id": "run1",
        "step_id": null,
        "timestamp_ms": 1234567890,
        "event": {
            "type": "RunStarted",
            "data": {
                "ticket": {"kind": "free-text", "reference": "test", "title": null},
                "profile": "default",
                "backend": "claude-code",
                "model": "claude-sonnet-4-6"
            }
        }
    }"#;
    let envelope: EventEnvelope = serde_json::from_str(legacy_json).expect("deserialize legacy");
    match &envelope.event {
        Event::RunStarted { agents, .. } => {
            assert!(agents.is_empty(), "legacy RunStarted without agents field should default to []");
        }
        other => panic!("expected RunStarted, got {other:?}"),
    }
}

#[test]
fn permission_request_serializes_to_json_snake_case() {
    // Verifies: "type" tag is "PermissionRequest" (PascalCase per rename_all = "PascalCase"),
    // and nested enum fields use snake_case discriminants per serde(rename_all = "snake_case").
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 1234567890,
        event: Event::PermissionRequest {
            request_id: "req-01JZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            agent: "developer".to_string(),
            tool: "Bash".to_string(),
            arg: "rm -rf node_modules".to_string(),
            scope: "shell.destructive".to_string(),
            risk: PermissionRisk::High,
            reason: "destructive shell".to_string(),
        },
    };
    let value = serde_json::to_value(&envelope).expect("json serialize");
    assert_eq!(
        value["event"]["type"],
        serde_json::Value::String("PermissionRequest".to_string()),
        "event type tag must be PermissionRequest"
    );
    assert_eq!(
        value["event"]["data"]["risk"],
        serde_json::Value::String("high".to_string()),
        "risk enum must serialize as snake_case 'high'"
    );
}
