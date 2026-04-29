import type { AgentStatus } from "../types/pipeline";

export type AgentCardProps = {
  agent: string;
  status: AgentStatus;
  onMenuClick?: () => void;
};

const AGENT_BG_CLASS: Record<string, string> = {
  architect: "bg-agent-architect",
  developer: "bg-agent-developer",
  qa: "bg-agent-qa",
  reviewer: "bg-agent-reviewer",
};

const STATUS_BORDER_CLASS: Record<AgentStatus, string> = {
  done: "border-status-done",
  active: "border-status-active",
  queued: "border-status-queued",
  failed: "border-status-failed",
  skipped: "border-border",
  errored: "border-border",
};

export default function AgentCard({ agent, status, onMenuClick }: AgentCardProps) {
  const borderClass = STATUS_BORDER_CLASS[status] ?? "border-border";
  const avatarBgClass = AGENT_BG_CLASS[agent] ?? "bg-bg-surface-2";

  return (
    <div
      data-testid={`agent-card-${agent}`}
      data-status={status}
      className={`relative border ${borderClass} rounded-[10px] bg-bg-surface px-3 py-2 flex flex-col gap-1`}
    >
      {status === "active" && (
        <div
          data-testid={`agent-card-${agent}-tint`}
          className="absolute inset-0 -z-10 rounded-[10px]"
          style={{ backgroundColor: "rgb(245 158 11 / 0.06)" }}
        />
      )}

      <div className="flex items-start gap-2">
        <div
          data-testid={`agent-card-${agent}-avatar`}
          className={`h-11 w-11 rounded-md flex-shrink-0 flex items-center justify-center ${avatarBgClass}`}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <rect x="2" y="2" width="12" height="12" rx="2" fill="white" fillOpacity="0.7" />
          </svg>
        </div>

        <div className="flex flex-col gap-0.5 flex-1 min-w-0 pt-0.5">
          <div className="flex items-center gap-1">
            <span className="text-[13px] font-semibold text-fg leading-none">{agent}</span>
            {status === "active" && (
              <span
                data-testid={`agent-card-${agent}-pulse`}
                className="animate-pulse rounded-full h-1.5 w-1.5 bg-status-active"
              />
            )}
          </div>
          <span className="text-[10px] uppercase tracking-[0.05em] text-fg-muted leading-none">
            {status}
          </span>
        </div>

        <button
          type="button"
          data-testid={`agent-card-${agent}-menu`}
          onClick={onMenuClick}
          aria-label="Agent menu"
          className="flex-shrink-0 text-fg-muted hover:text-fg text-base leading-none px-0.5"
        >
          ⋯
        </button>
      </div>
    </div>
  );
}
