import { invoke } from "@tauri-apps/api/core";
import type { EventEnvelope } from "../types/event";
import { isPermissionRequestEvent } from "../types/event";
import type { PermissionRequest } from "../types/pipeline";
import ActivityHeader, {
  type ActivityFilter,
  type ActivityCounts,
} from "./ActivityHeader";
import LogRow from "./LogRow";
import ToolCallCard from "./ToolCallCard";
import PermissionCard from "./PermissionCard";
import { usePermissionRequests } from "../hooks/usePermissionRequests";

export type ActivityColumnProps = {
  events: EventEnvelope[];
  filter: ActivityFilter;
  onFilterChange: (filter: ActivityFilter) => void;
  /** Active run id, threaded from App.tsx. Decisions are skipped (with a
   * console.warn) when undefined (e.g. before any run has started). */
  runId?: string;
  /** Active step id — optional; backend accepts Option<String>. */
  stepId?: string;
  // TODO(tech-debt): a dedicated permissionError slot on App.tsx would be
  // cleaner than surfacing IPC errors via the same channel as history errors.
  // Deferred: App.tsx has no error setter today; add in a future cleanup step.
};

type VisibleRow =
  | { kind: "info"; id: string; t: string; agent: string; message: string }
  | { kind: "error"; id: string; t: string; agent: string; message: string }
  | {
      kind: "tool";
      id: string;
      t: string;
      agent: string;
      tool: string;
      arg: string;
      result: string;
      details?: string;
    }
  | { kind: "permission"; id: string; permission: PermissionRequest };

type ActivityRow = VisibleRow | { kind: "filtered" };

const ERROR_TYPE_RE = /(Error|Failed|Exception)/i;

// Event types that are pure noise — chain-of-thought deltas, stream chunks,
// or events that are paired with a start event and would produce duplicate rows.
const FILTERED_TYPES = new Set([
  "TextDelta",
  "ThinkingDelta",
  "ToolUseDelta",
  "ToolUseEnd",
]);

function formatTime(ms: number): string {
  const d = new Date(ms);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

function resolveAgent(
  env: EventEnvelope,
  stepAgents: Map<string, string>,
): string {
  if (env.step_id === null) return "system";
  return stepAgents.get(env.step_id) ?? "system";
}

function buildStepAgentMap(events: EventEnvelope[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const env of events) {
    if (env.event.type === "StepStarted" && env.step_id !== null) {
      const data = (env.event.data ?? {}) as { agent?: unknown };
      if (typeof data.agent === "string") {
        map.set(env.step_id, data.agent);
      }
    }
  }
  return map;
}

function classify(
  env: EventEnvelope,
  permsById: Map<string, PermissionRequest>,
  stepAgents: Map<string, string>,
): ActivityRow {
  const type = env.event.type;

  if (FILTERED_TYPES.has(type)) return { kind: "filtered" };

  if (isPermissionRequestEvent(env.event)) {
    // Narrowed: env.event is PermissionRequestEvent; data.request_id is fully typed.
    const perm = permsById.get(env.event.data.request_id);
    if (perm !== undefined) {
      return { kind: "permission", id: env.event_id, permission: perm };
    }
    return { kind: "filtered" };
  }

  const t = formatTime(env.timestamp_ms);
  const agent = resolveAgent(env, stepAgents);

  // Backward-compat: legacy dev-mock "ToolCall" events (pre-real-backend format)
  if (type === "ToolCall") {
    const data = (env.event.data ?? {}) as {
      tool?: unknown;
      arg?: unknown;
      result?: unknown;
      details?: unknown;
    };
    return {
      kind: "tool",
      id: env.event_id,
      t,
      agent,
      tool: typeof data.tool === "string" ? data.tool : "?",
      arg: typeof data.arg === "string" ? data.arg : "",
      result: typeof data.result === "string" ? data.result : "",
      details: typeof data.details === "string" ? data.details : undefined,
    };
  }

  // Real backend: ToolUseStart is the card-producing event
  if (type === "ToolUseStart") {
    const data = (env.event.data ?? {}) as {
      tool_name?: unknown;
      input?: unknown;
      tool_call_id?: unknown;
    };
    const tool = typeof data.tool_name === "string" ? data.tool_name : "?";
    const arg =
      typeof data.input === "string"
        ? data.input
        : data.input !== undefined
          ? JSON.stringify(data.input)
          : "";
    return {
      kind: "tool",
      id: env.event_id,
      t,
      agent,
      tool,
      arg,
      result: "OK",
      details: undefined,
    };
  }

  if (type === "Finding") {
    const data = (env.event.data ?? {}) as {
      message?: unknown;
      title?: unknown;
      severity?: unknown;
    };
    const message =
      typeof data.message === "string"
        ? data.message
        : typeof data.title === "string"
          ? data.title
          : "Finding";
    // Only severity "error" (case-insensitive) gets the red chip / Errors-tab
    // visibility. Warning / info / unknown severities render as plain info rows
    // in All — they are observations, not failures.
    const severity =
      typeof data.severity === "string" ? data.severity.toLowerCase() : "info";
    const kind: "error" | "info" = severity === "error" ? "error" : "info";
    return { kind, id: env.event_id, t, agent, message };
  }

  if (ERROR_TYPE_RE.test(type)) {
    const data = (env.event.data ?? {}) as { message?: unknown };
    return {
      kind: "error",
      id: env.event_id,
      t,
      agent,
      message: typeof data.message === "string" ? data.message : type,
    };
  }

  // Per-type human-readable message translations
  let message = type;
  if (type === "RunStarted") {
    message = "Run started";
  } else if (type === "RunComplete") {
    const data = (env.event.data ?? {}) as {
      status?: unknown;
      duration_ms?: unknown;
    };
    const status =
      typeof data.status === "string" ? data.status : "completed";
    const seconds =
      typeof data.duration_ms === "number"
        ? Math.round(data.duration_ms / 1000)
        : null;
    message =
      seconds !== null ? `Run ${status} in ${seconds}s` : `Run ${status}`;
  } else if (type === "StepStarted") {
    message = "Started";
  } else if (type === "StepComplete") {
    const data = (env.event.data ?? {}) as {
      status?: unknown;
      duration_ms?: unknown;
      summary?: unknown;
    };
    const status =
      typeof data.status === "string" ? data.status : "passed";
    const seconds =
      typeof data.duration_ms === "number"
        ? Math.round(data.duration_ms / 1000)
        : null;
    const summary =
      typeof data.summary === "string" && data.summary.length > 0
        ? data.summary
        : null;
    if (summary) {
      message = summary;
    } else if (seconds !== null) {
      message = `${status} in ${seconds}s`;
    } else {
      message = status;
    }
  } else if (type === "FileChange") {
    const data = (env.event.data ?? {}) as { path?: unknown };
    message =
      typeof data.path === "string"
        ? `Changed ${data.path}`
        : "File changed";
  } else if (type === "ClarifyingQuestion") {
    const data = (env.event.data ?? {}) as { question?: unknown };
    message =
      typeof data.question === "string"
        ? `? ${data.question}`
        : "Clarifying question";
  }

  return { kind: "info", id: env.event_id, t, agent, message };
}

function isVisible(row: ActivityRow): row is VisibleRow {
  return row.kind !== "filtered";
}

function matchesFilter(row: VisibleRow, filter: ActivityFilter): boolean {
  if (filter === "all") return true;
  if (filter === "tool") return row.kind === "tool";
  if (filter === "error") return row.kind === "error";
  if (filter === "perm") return row.kind === "permission";
  return false;
}

export default function ActivityColumn({
  events,
  filter,
  onFilterChange,
  runId,
  stepId,
}: ActivityColumnProps) {
  const pendingPermissions = usePermissionRequests();
  const permsById = new Map(
    pendingPermissions.map((p) => [p.requestId, p]),
  );

  async function handleDecision(
    requestId: string,
    decision: "once" | "session" | "deny",
  ) {
    if (!runId) {
      console.warn("permission_decide called with no runId; skipping IPC");
      return;
    }
    try {
      await invoke("permission_decide", { requestId, decision, runId, stepId });
    } catch (err) {
      // Surface IPC errors to the console for now. Tech-debt: wire a dedicated
      // permissionError setter when App.tsx gains an error state slot.
      console.error("permission_decide failed:", err);
    }
  }

  // Build step_id → agent name map from StepStarted events before classifying.
  // Real backend uses ULIDs as step_id; agent name lives in StepStarted.event.data.agent.
  const stepAgents = buildStepAgentMap(events);

  const rows = events.map((env) => classify(env, permsById, stepAgents));

  const counts: ActivityCounts = {
    all: rows.filter((r) => r.kind !== "filtered").length,
    tool: rows.filter((r) => r.kind === "tool").length,
    perm: rows.filter((r) => r.kind === "permission").length,
    error: rows.filter((r) => r.kind === "error").length,
  };

  const visible = rows.filter(isVisible).filter((r) => matchesFilter(r, filter));

  return (
    <div
      data-testid="activity-column"
      className="flex flex-col h-full bg-bg-surface border-r border-border-soft"
    >
      <ActivityHeader
        counts={counts}
        filter={filter}
        onFilterChange={onFilterChange}
      />
      <ul
        data-testid="event-list"
        aria-live="polite"
        aria-relevant="additions"
        className="flex-1 min-h-0 overflow-y-auto px-4 py-3 flex flex-col gap-2"
      >
        {visible.length === 0 ? (
          <li className="text-fg-muted italic text-sm">
            No events match this filter.
          </li>
        ) : (
          visible.map((row) => {
            if (row.kind === "permission") {
              return (
                <li key={row.id} data-testid="event-row">
                  <PermissionCard
                    permission={row.permission}
                    onDecision={(decision) =>
                      void handleDecision(row.permission.requestId, decision)
                    }
                  />
                </li>
              );
            }
            if (row.kind === "tool") {
              return (
                <li key={row.id} data-testid="event-row">
                  <ToolCallCard
                    agent={row.agent}
                    tool={row.tool}
                    arg={row.arg}
                    result={row.result}
                    details={row.details}
                  />
                </li>
              );
            }
            return (
              <li key={row.id} data-testid="event-row">
                <LogRow
                  level={row.kind}
                  t={row.t}
                  agent={row.agent}
                  message={row.message}
                />
              </li>
            );
          })
        )}
      </ul>
    </div>
  );
}
