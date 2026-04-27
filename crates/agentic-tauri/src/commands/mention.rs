use agentic_core::events::{Event, EventEnvelope};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};
use ulid::Ulid;

use super::events::EventBusState;

/// The frontend channel name for mention events. Frontend chat listens here;
/// cockpit Stepper does NOT subscribe to this channel.
pub const MENTION_EVENT_CHANNEL: &str = "agentic://mention-event";

/// Maximum accepted body length. Mirrors the chat-message bound to keep IPC
/// payloads predictable.
const MAX_BODY_LEN: usize = 4096;

/// Validate an agent name. Mirrors the frontend regex `[a-zA-Z0-9_-]+` so the
/// server enforces the same contract a malicious or buggy frontend could
/// otherwise bypass.
fn is_valid_agent_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Debug, Serialize)]
pub struct MentionResult {
    pub run_id: String,
    pub agent: String,
    /// Whether a real backend dispatch was attempted.
    ///
    /// Lifecycle:
    /// - Phase 11.4 (current): always `false` — the IPC contract is wired but
    ///   the backend is a synthetic event-stream stub. Frontend MUST treat
    ///   `false` as "no real run started" and surface a [STUB] indicator.
    /// - Phase 11.5+: flips to `true` once `Backend::execute` is wired and a
    ///   real `RunStarted` envelope has been published to the EventBus. The
    ///   field stays `true` for the lifetime of the result; it is not a
    ///   running-state flag.
    pub dispatched: bool,
}

/// Tauri command: dispatch a single-agent run via @mention.
///
/// Phase 11.4 MVP — wires the IPC contract but the actual `Backend::execute`
/// invocation is a STUB. Synthesises a fake event stream on the dedicated
/// `agentic://mention-event` channel to demonstrate routing.
#[tauri::command]
pub async fn mention_agent<R: Runtime>(
    app: AppHandle<R>,
    _state: State<'_, EventBusState>,
    agent: String,
    body: String,
) -> Result<MentionResult, String> {
    let agent = agent.trim().to_string();
    if agent.is_empty() {
        return Err("agent is empty".to_string());
    }
    if !is_valid_agent_name(&agent) {
        return Err(format!(
            "agent name contains invalid characters (allowed: alphanumeric, '_', '-'): {agent}"
        ));
    }

    // Trim the body server-side as well as the frontend so the contract is
    // symmetric — a non-Tauri caller (e.g., integration tests) gets the same
    // normalisation as the React parser.
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err("body is empty".to_string());
    }
    if body.len() > MAX_BODY_LEN {
        return Err(format!(
            "body exceeds {MAX_BODY_LEN} bytes (got {})",
            body.len()
        ));
    }

    let run_id = Ulid::new().to_string().to_lowercase();
    let agent_clone = agent.clone();
    let body_clone = body.clone();
    let run_id_clone = run_id.clone();

    // Spawn a stub event stream. Phase 11.5+ replaces with real Backend::execute.
    tokio::spawn(async move {
        let envs = vec![
            EventEnvelope::now(
                run_id_clone.clone(),
                None,
                Event::TextDelta {
                    content: format!("[STUB] @{agent_clone} received: {body_clone}"),
                },
            ),
            EventEnvelope::now(
                run_id_clone.clone(),
                None,
                Event::TextDelta {
                    content: "[STUB] @mention dispatch is Phase 11.5+ — no real backend yet"
                        .to_string(),
                },
            ),
        ];
        for env in envs {
            let _ = app.emit(MENTION_EVENT_CHANNEL, &env);
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });

    Ok(MentionResult {
        run_id,
        agent,
        dispatched: false,
    })
}
