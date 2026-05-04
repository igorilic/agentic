/** Envelope from the Tauri `agentic://event` channel. */

export type PermissionRequestEvent = {
  type: "PermissionRequest";
  request_id: string;
  agent: string;
  tool: string;
  arg: string;
  scope: string;
  risk: "low" | "medium" | "high";
  reason: string;
};

export type PermissionResolvedEvent = {
  type: "PermissionResolved";
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

export type EventEnvelope = {
  schema_version: number;
  event_id: string;
  run_id: string;
  step_id: string | null;
  timestamp_ms: number;
  event: {
    type: string;
    data?: unknown;
  };
};
