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

/**
 * Runtime type guard for a single `EventEnvelope`.
 *
 * Validates only the envelope-level fields required by the type definition.
 * The inner `event.data` is deliberately left as `unknown` — the catch-all
 * variant in the union explicitly allows arbitrary data payloads.
 */
export function isEventEnvelope(value: unknown): value is EventEnvelope {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const v = value as Record<string, unknown>;
  if (typeof v["event_id"] !== "string" || v["event_id"] === "") return false;
  if (typeof v["run_id"] !== "string") return false;
  if (typeof v["schema_version"] !== "number") return false;
  if (typeof v["timestamp_ms"] !== "number") return false;
  // step_id must be string | null — undefined is not acceptable.
  if (v["step_id"] !== null && typeof v["step_id"] !== "string") return false;
  // event must be an object with a string `type` field.
  const evt = v["event"];
  if (evt === null || typeof evt !== "object" || Array.isArray(evt)) return false;
  if (typeof (evt as Record<string, unknown>)["type"] !== "string") return false;
  return true;
}

/**
 * Runtime type guard for an array of `EventEnvelope` values.
 *
 * Returns false if the outer value is not an array, or if any element
 * fails `isEventEnvelope`. Use this to validate the response from the
 * `get_event_history` IPC command before trusting the cast.
 */
export function isEventEnvelopeArray(value: unknown): value is EventEnvelope[] {
  if (!Array.isArray(value)) return false;
  return value.every(isEventEnvelope);
}
