use agentic_core::{
    ActionRequired, BackendId, Event, EventEnvelope, ModelId, ProfileId, RunStatus, Severity,
    StepStatus, TicketRef, TokenUsage, ToolStream,
};

fn sample_events() -> Vec<Event> {
    vec![
        Event::RunStarted {
            ticket: TicketRef {
                kind: "github-issue".to_string(),
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
        },
        Event::TextDelta { content: "hello".to_string() },
        Event::ThinkingDelta { content: "pondering".to_string() },
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
        Event::RetryStarted { attempt: 2, reason: "rate limited".to_string() },
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
        let envelope =
            EventEnvelope::now("run1".to_string(), Some("step1".to_string()), original.clone());
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
        Event::RunStarted { ticket, profile, backend, model } => {
            assert_eq!(ticket.kind, "github-issue");
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
        Event::TextDelta { content: "x".to_string() },
    );
    ulid::Ulid::from_string(&envelope.event_id).expect("event_id parses as ULID");
}

#[test]
fn timestamp_ms_is_monotonic_across_consecutive_now_calls() {
    let a =
        EventEnvelope::now("run1".to_string(), None, Event::TextDelta { content: "a".to_string() });
    // 2ms sleep guarantees distinct millis on fast machines.
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b =
        EventEnvelope::now("run1".to_string(), None, Event::TextDelta { content: "b".to_string() });
    assert!(
        b.timestamp_ms > a.timestamp_ms,
        "expected b.timestamp_ms ({}) > a.timestamp_ms ({})",
        b.timestamp_ms,
        a.timestamp_ms
    );
}
