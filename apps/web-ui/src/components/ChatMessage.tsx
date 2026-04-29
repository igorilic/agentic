import type { ReactNode } from "react";

export type ChatMessageProps =
  | { kind: "user"; userName: string; timestamp: string; body: string }
  | { kind: "system"; body: string }
  | { kind: "agent"; agent: string; timestamp: string; body: string };

// Per-agent Tailwind name classes — literal strings required for JIT scanner.
const AGENT_NAME_CLASS: Record<string, string> = {
  architect: "text-agent-architect",
  developer: "text-agent-developer",
  qa: "text-agent-qa",
  reviewer: "text-agent-reviewer",
};

// Spec §3.4 line 219 — agent bubble background: rgba(<accent>, 0.04).
// Note: AgentCard uses 0.06 per §3.2 (active-card overlay) — different surface.
const AGENT_TINT_RGBA: Record<string, string> = {
  architect: "rgb(59 130 246 / 0.04)",  // --agent-architect #3b82f6
  developer: "rgb(16 185 129 / 0.04)",  // --agent-developer #10b981
  qa: "rgb(139 92 246 / 0.04)",         // --agent-qa #8b5cf6
  reviewer: "rgb(245 158 11 / 0.04)",   // --agent-reviewer #f59e0b
};
const TINT_FALLBACK = "rgb(245 158 11 / 0.04)"; // amber fallback for unknown agents

export default function ChatMessage(props: ChatMessageProps) {
  if (props.kind === "system") {
    return (
      <div
        data-testid="chat-message-system"
        className="text-center text-[11px] text-fg-subtle py-1"
      >
        {props.body}
      </div>
    );
  }

  if (props.kind === "user") {
    return (
      <div data-testid="chat-message-user" className="flex gap-3">
        <div
          data-testid="chat-message-user-avatar"
          className="h-7 w-7 rounded-full bg-zinc-200 flex-shrink-0"
          aria-hidden="true"
        />
        <div className="flex flex-col gap-1 min-w-0">
          <div className="flex items-baseline gap-2">
            <span className="text-[13px] font-semibold text-fg">{props.userName}</span>
            <span className="text-[11px] text-fg-subtle">{props.timestamp}</span>
          </div>
          <div className="text-sm text-fg leading-6">{renderInline(props.body)}</div>
        </div>
      </div>
    );
  }

  // agent variant
  // For unknown agents: name falls back to text-fg, border uses var(--fg-muted).
  const nameClass = AGENT_NAME_CLASS[props.agent] ?? "text-fg";
  const tintRgba = AGENT_TINT_RGBA[props.agent] ?? TINT_FALLBACK;
  const borderColor = AGENT_NAME_CLASS[props.agent]
    ? `var(--agent-${props.agent})`
    : "var(--fg-muted)";

  return (
    <div
      data-testid="chat-message-agent"
      data-agent={props.agent}
      className="flex gap-3"
    >
      <div
        className="h-7 w-7 rounded-full flex-shrink-0"
        style={{ backgroundColor: tintRgba }}
        aria-hidden="true"
      />
      <div className="flex flex-col gap-1 min-w-0">
        <div className="flex items-baseline gap-2">
          <span
            data-testid="chat-message-agent-name"
            className={`text-[13px] font-semibold ${nameClass}`}
          >
            {props.agent}
          </span>
          <span className="text-[11px] text-fg-subtle">{props.timestamp}</span>
        </div>
        <div
          data-testid="chat-message-agent-bubble"
          className="text-sm text-fg leading-6 px-3 py-2 rounded-md border-l-[3px]"
          style={{ backgroundColor: tintRgba, borderLeftColor: borderColor }}
        >
          {renderInline(props.body)}
        </div>
      </div>
    </div>
  );
}

// Spec §3.4 line 221 — highlight slash commands and @mentions in message bodies.
// Regex is lowercase-only per todo §W.4.2. Widening (case-insensitive, hyphens, digits)
// is a deliberate future decision to avoid unintended matches.
// System messages render plain text; only user + agent bodies call this helper.
function renderInline(text: string): ReactNode[] {
  const parts = text.split(/(\/[a-z]+|@[a-z]+)/g);
  return parts.map((part, i) => {
    if (part === "") return null;
    const isToken = /^(\/[a-z]+|@[a-z]+)$/.test(part);
    if (isToken) {
      return (
        <span
          key={i}
          data-testid="chat-token"
          className="bg-[rgba(253,230,138,0.4)] rounded-sm px-0.5"
        >
          {part}
        </span>
      );
    }
    return <span key={i}>{part}</span>;
  });
}
