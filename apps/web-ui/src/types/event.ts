/** Envelope from the Tauri `agentic://event` channel. */

/**
 * Rust wire format: `Event` uses `#[serde(tag = "type", content = "data")]`.
 * All payload fields nest under `data`, NOT at the top level.
 */
export type PermissionRequestEvent = {
  type: "PermissionRequest";
  data: {
    request_id: string;
    agent: string;
    tool: string;
    arg: string;
    scope: string;
    risk: "low" | "medium" | "high";
    reason: string;
  };
};

export type PermissionResolvedEvent = {
  type: "PermissionResolved";
  data: {
    request_id: string;
    decision: "allow_once" | "allow_session" | "deny" | "timed_out";
    source:
      | "user"
      | "allowlist_config"
      | "denylist_config"
      | "session_allowlist"
      | "timeout"
      | "cancelled";
  };
};

export type EventEnvelope = {
  schema_version: number;
  event_id: string;
  run_id: string;
  step_id: string | null;
  timestamp_ms: number;
  event:
    | PermissionRequestEvent
    | PermissionResolvedEvent
    | { type: string; data?: unknown }; // catch-all for variants not yet typed
};

/**
 * Type guard: narrows `env.event` to `PermissionRequestEvent`.
 * Required because the catch-all `{ type: string; data?: unknown }` in the union
 * prevents TypeScript from narrowing via `if (event.type === "PermissionRequest")` alone —
 * the catch-all's `type: string` subsumes the literal, so `data` stays `unknown`.
 */
export function isPermissionRequestEvent(
  event: EventEnvelope["event"],
): event is PermissionRequestEvent {
  return event.type === "PermissionRequest";
}

/**
 * Type guard: narrows `env.event` to `PermissionResolvedEvent`.
 */
export function isPermissionResolvedEvent(
  event: EventEnvelope["event"],
): event is PermissionResolvedEvent {
  return event.type === "PermissionResolved";
}
