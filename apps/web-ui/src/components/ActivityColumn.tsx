import type { EventEnvelope } from "../types/event";
import type { PermissionRequest } from "../types/pipeline";
import ActivityHeader, {
  type ActivityFilter,
  type ActivityCounts,
} from "./ActivityHeader";
import LogRow from "./LogRow";
import ToolCallCard from "./ToolCallCard";
import PermissionCard from "./PermissionCard";

export type ActivityColumnProps = {
  events: EventEnvelope[];
  filter: ActivityFilter;
  onFilterChange: (filter: ActivityFilter) => void;
  pendingPermissions?: PermissionRequest[];
  onPermissionDecision?: (
    permId: string,
    decision: "once" | "session" | "deny",
  ) => void;
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

function formatTime(ms: number): string {
  const d = new Date(ms);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

function classify(
  env: EventEnvelope,
  permsById: Map<string, PermissionRequest>,
): ActivityRow {
  const type = env.event.type;
  if (type === "TextDelta") return { kind: "filtered" };

  if (type === "PermissionRequest") {
    const data = (env.event.data ?? {}) as { permId?: unknown };
    if (typeof data.permId === "string") {
      const perm = permsById.get(data.permId);
      if (perm !== undefined) {
        return { kind: "permission", id: env.event_id, permission: perm };
      }
    }
    return { kind: "filtered" };
  }

  const t = formatTime(env.timestamp_ms);
  const agent = env.step_id ?? "system";

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

  if (type === "Finding") {
    const data = (env.event.data ?? {}) as { message?: unknown; title?: unknown; severity?: unknown };
    const message =
      typeof data.message === "string"
        ? data.message
        : typeof data.title === "string"
          ? data.title
          : "Finding";
    return {
      kind: "error",
      id: env.event_id,
      t,
      agent,
      message,
    };
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

  return {
    kind: "info",
    id: env.event_id,
    t,
    agent,
    message: type,
  };
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
  pendingPermissions,
  onPermissionDecision,
}: ActivityColumnProps) {
  const permsById = new Map(
    (pendingPermissions ?? []).map((p) => [p.id, p]),
  );

  const rows = events.map((env) => classify(env, permsById));

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
                      onPermissionDecision?.(row.permission.id, decision)
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
