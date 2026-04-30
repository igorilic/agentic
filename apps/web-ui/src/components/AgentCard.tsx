import { useState, useRef, useEffect } from "react";
import type { AgentStatus } from "../types/pipeline";
import { AGENT_LIBRARY } from "../types/pipeline";
import AgentIcon from "./AgentIcon";
import StatusDot from "./StatusDot";
import { getAgentAccent } from "../utils/agentAccents";

export type AgentCardProps = {
  agent: string;
  status: AgentStatus;
  index: number;
  skipped?: boolean;
  onRemove?: () => void;
  onSkip?: () => void;
  draggable?: boolean;
  dragging?: boolean;
  onDragStart?: (e: React.DragEvent<HTMLDivElement>) => void;
  onDragEnd?: (e: React.DragEvent<HTMLDivElement>) => void;
};

const AGENT_BG_CLASS: Record<string, string> = {
  architect: "bg-agent-architect",
  developer: "bg-agent-developer",
  qa: "bg-agent-qa",
  reviewer: "bg-agent-reviewer",
};

const AGENT_TINT_RGBA: Record<string, string> = {
  architect: "rgb(59 130 246 / 0.06)",   // --agent-architect #3b82f6
  developer: "rgb(16 185 129 / 0.06)",   // --agent-developer #10b981
  qa: "rgb(139 92 246 / 0.06)",          // --agent-qa #8b5cf6
  reviewer: "rgb(245 158 11 / 0.06)",    // --agent-reviewer #f59e0b
};
const TINT_FALLBACK = "rgb(245 158 11 / 0.06)"; // amber (status-active)

const STATUS_BORDER_CLASS: Record<AgentStatus, string> = {
  done: "border-status-done",
  active: "border-status-active",
  queued: "border-status-queued",
  failed: "border-status-failed",
  skipped: "border-border",
  errored: "border-border",
};

export default function AgentCard({
  agent,
  status,
  index,
  skipped = false,
  onRemove,
  onSkip,
  draggable: isDraggable = false,
  dragging = false,
  onDragStart,
  onDragEnd,
}: AgentCardProps) {
  const borderClass = STATUS_BORDER_CLASS[status] ?? "border-border";
  const avatarBgClass = AGENT_BG_CLASS[agent] ?? "bg-bg-surface-2";
  const lib = AGENT_LIBRARY.find((a) => a.id === agent);
  const displayName = lib?.name ?? agent;
  const accent = getAgentAccent(agent);

  const [menuOpen, setMenuOpen] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const onMouseDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", onMouseDown);
    return () => document.removeEventListener("mousedown", onMouseDown);
  }, [menuOpen]);

  return (
    <div
      data-testid={`agent-card-${agent}`}
      data-status={status}
      data-dragging={dragging ? "true" : "false"}
      data-skipped={skipped ? "true" : "false"}
      role="button"
      tabIndex={0}
      aria-label={`${agent} — ${status}`}
      draggable={isDraggable}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      className={`relative border ${borderClass} rounded-[10px] bg-bg-surface px-3 py-2 flex flex-col gap-1${skipped ? " opacity-50" : ""}`}
    >
      {status === "active" && (
        <div
          data-testid={`agent-card-${agent}-tint`}
          aria-hidden="true"
          className="absolute inset-0 -z-10 rounded-[10px]"
          style={{ backgroundColor: AGENT_TINT_RGBA[agent] ?? TINT_FALLBACK }}
        />
      )}

      <div className="flex items-start gap-2">
        <span
          data-testid={`agent-card-${agent}-step-number`}
          className="text-[11px] font-semibold text-fg-subtle tabular-nums w-4 text-right self-center"
        >
          {String(index + 1).padStart(2, "0")}
        </span>
        <div
          data-testid={`agent-card-${agent}-avatar`}
          className={`h-11 w-11 rounded-md flex-shrink-0 flex items-center justify-center ${avatarBgClass}`}
          style={{ backgroundColor: accent.bg, color: accent.fg }}
        >
          <AgentIcon agent={agent} size={18} />
        </div>

        <div className="flex flex-col gap-0.5 flex-1 min-w-0 pt-0.5">
          <div className="flex items-center gap-1">
            <span className={`text-[13px] font-semibold text-fg leading-none${skipped ? " line-through" : ""}`}>{displayName}</span>
            {status === "active" && (
              <span
                data-testid={`agent-card-${agent}-pulse`}
                aria-hidden="true"
                className="animate-pulse rounded-full h-1.5 w-1.5 bg-status-active"
              />
            )}
          </div>
          <StatusDot status={status} />
        </div>

        <div className="relative flex-shrink-0">
          <button
            type="button"
            data-testid={`agent-card-${agent}-menu`}
            onClick={() => setMenuOpen((o) => !o)}
            aria-label="Agent menu"
            aria-haspopup="true"
            aria-expanded={menuOpen}
            className="text-fg-muted hover:text-fg text-base leading-none px-0.5"
          >
            ⋯
          </button>

          {menuOpen && (
            <div
              ref={menuRef}
              data-testid={`agent-card-${agent}-menu-list`}
              role="menu"
              className="absolute right-0 top-full mt-1 z-10 w-44 rounded-lg border border-border-strong bg-bg-surface shadow-popover py-1"
              onKeyDown={(e) => { if (e.key === "Escape") setMenuOpen(false); }}
            >
              <button
                type="button"
                role="menuitem"
                data-testid={`agent-card-${agent}-menu-remove`}
                onClick={() => { onRemove?.(); setMenuOpen(false); }}
                className="w-full px-3 py-1.5 text-left text-[13px] text-fg hover:bg-bg-surface-2"
              >
                Remove
              </button>
              <button
                type="button"
                role="menuitem"
                data-testid={`agent-card-${agent}-menu-skip`}
                onClick={() => { onSkip?.(); setMenuOpen(false); }}
                className="w-full px-3 py-1.5 text-left text-[13px] text-fg hover:bg-bg-surface-2"
              >
                Skip this run
              </button>
              <button
                type="button"
                role="menuitem"
                data-testid={`agent-card-${agent}-menu-configure`}
                onClick={() => { setModalOpen(true); setMenuOpen(false); }}
                className="w-full px-3 py-1.5 text-left text-[13px] text-fg hover:bg-bg-surface-2"
              >
                Configure…
              </button>
            </div>
          )}
        </div>
      </div>

      {modalOpen && (
        <div
          data-testid="agent-configure-backdrop"
          className="fixed inset-0 z-20 bg-black/40 flex items-center justify-center"
          onClick={() => setModalOpen(false)}
        >
          <div
            data-testid="agent-configure-modal"
            role="dialog"
            aria-modal="true"
            aria-label="Configure agent"
            className="w-[420px] rounded-xl border border-border bg-bg-surface shadow-modal p-4"
            onClick={(e) => e.stopPropagation()}
          >
            <header className="text-[14px] font-semibold text-fg mb-2">
              Configure agent — not yet implemented
            </header>
            <button
              type="button"
              data-testid="agent-configure-close"
              onClick={() => setModalOpen(false)}
              className="rounded-md border border-border-strong px-3 py-1.5 text-xs font-semibold text-fg"
            >
              Close
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
