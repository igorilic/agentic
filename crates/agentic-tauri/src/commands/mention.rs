use agentic_core::events::{Event, EventEnvelope};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};
use ulid::Ulid;

use super::events::EventBusState;

/// The frontend channel name for mention events. Frontend chat listens here;
/// cockpit Stepper does NOT subscribe to this channel.
pub const MENTION_EVENT_CHANNEL: &str = "agentic://mention-event";

#[derive(Debug, Serialize)]
pub struct MentionResult {
    pub run_id: String,
    pub agent: String,
    /// True when a real backend dispatch was attempted. Phase 11.4 stub: always
    /// false. Phase 11.5+ sets this to true when `Backend::execute` is wired.
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
    if agent.trim().is_empty() {
        return Err("agent is empty".to_string());
    }
    if body.trim().is_empty() {
        return Err("body is empty".to_string());
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
                    content: format!("@{} received: {}", agent_clone, body_clone),
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
