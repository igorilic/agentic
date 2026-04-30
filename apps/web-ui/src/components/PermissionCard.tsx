import type { PermissionRequest } from "../types/pipeline";

export type PermissionCardProps = {
  permission: PermissionRequest;
  onDecision: (decision: "once" | "session" | "deny") => void;
};

const RISK_PILL_CLASS: Record<PermissionRequest["risk"], string> = {
  high:   "bg-red-100 text-red-700",
  medium: "bg-amber-100 text-amber-700",
  low:    "bg-zinc-100 text-zinc-700",
};

const RISK_LABEL: Record<PermissionRequest["risk"], string> = {
  high:   "HIGH RISK",
  medium: "MEDIUM",
  low:    "LOW",
};

export default function PermissionCard({ permission, onDecision }: PermissionCardProps) {
  const { agent, arg, scope, risk, reason } = permission;
  return (
    <div
      data-testid="permission-card"
      className="rounded-[10px] border border-[#fca5a5] border-l-[3px] px-3.5 py-3 flex flex-col gap-2"
      style={{ backgroundColor: "rgba(252, 165, 165, 0.06)" }}
    >
      {/* Header */}
      <div className="flex items-center gap-2">
        <svg
          data-testid="permission-card-warn-icon"
          viewBox="0 0 16 16"
          className="h-4 w-4 text-red-600"
          fill="currentColor"
          aria-hidden="true"
        >
          <path d="M8 1l7 13H1z M8 6v4 M8 11.5v.5" stroke="white" strokeWidth="1" />
        </svg>
        <span className="text-[13px] font-semibold text-fg">
          {agent} requests permission
        </span>
        <span
          data-testid="permission-card-risk"
          className={`ml-auto rounded px-2 py-0.5 text-[10px] font-semibold uppercase ${RISK_PILL_CLASS[risk]}`}
        >
          {RISK_LABEL[risk]}
        </span>
      </div>

      {/* Command preview */}
      <pre
        data-testid="permission-card-command"
        className="rounded bg-black px-3 py-2 text-[12px] font-mono text-[#a7f3d0] overflow-x-auto"
      >
        $ {arg}
      </pre>

      {/* Reason + scope */}
      <div className="flex items-center gap-2 text-[11px] text-fg-muted">
        <span>{reason}</span>
        <span
          data-testid="permission-card-scope"
          className="ml-auto rounded bg-bg-surface-2 px-1.5 py-0.5 text-fg-muted"
        >
          {scope}
        </span>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1">
        <button
          type="button"
          data-testid="permission-card-allow-once"
          onClick={() => onDecision("once")}
          className="rounded-md bg-[#18181b] px-3 py-1.5 text-xs font-semibold text-white"
        >
          Allow once
        </button>
        <button
          type="button"
          data-testid="permission-card-allow-session"
          onClick={() => onDecision("session")}
          className="rounded-md px-3 py-1.5 text-xs font-semibold text-fg hover:bg-bg-surface-2"
        >
          Allow for session
        </button>
        <button
          type="button"
          data-testid="permission-card-deny"
          onClick={() => onDecision("deny")}
          className="ml-auto rounded-md px-3 py-1.5 text-xs font-semibold text-red-600 hover:bg-bg-surface-2"
        >
          Deny
        </button>
      </div>
    </div>
  );
}
