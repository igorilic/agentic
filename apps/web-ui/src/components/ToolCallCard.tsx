import { useState } from "react";

// Per-agent Tailwind name classes — literal strings required for JIT scanner.
const AGENT_COLOR_CLASS: Record<string, string> = {
  architect: "text-agent-architect",
  developer: "text-agent-developer",
  qa: "text-agent-qa",
  reviewer: "text-agent-reviewer",
};

export type ToolCallCardProps = {
  agent: string;
  tool: string;
  arg: string;
  result: string;
  details?: string;
};

export default function ToolCallCard({ agent, tool, arg, result, details }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false);
  const agentClass = AGENT_COLOR_CLASS[agent] ?? "text-fg";

  const isOk = result === "OK";
  const isErr = result === "error";
  const chipKey = isOk ? "ok" : isErr ? "error" : "neutral";
  const chipClass = isOk
    ? "bg-green-100 text-green-700"
    : isErr
      ? "bg-red-100 text-red-700"
      : "bg-bg-surface-2 text-fg-muted";

  return (
    <div
      data-testid="tool-call-card"
      className="rounded-lg border border-border px-3 py-2.5 bg-bg-surface"
    >
      <div className="flex items-center gap-2 text-[12px] font-mono">
        <span data-testid="tool-call-card-agent" className={`font-semibold ${agentClass}`}>
          {agent}
        </span>
        <span className="text-fg">
          {tool}(<span className="text-fg-muted">{arg}</span>)
        </span>
        <span
          data-testid={`result-chip-${chipKey}`}
          className={`ml-auto rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${chipClass}`}
        >
          {result}
        </span>
        {details !== undefined && (
          <button
            type="button"
            data-testid="tool-call-card-toggle"
            aria-expanded={expanded}
            aria-label="Toggle details"
            onClick={() => setExpanded((e) => !e)}
            className="text-fg-muted text-xs px-1"
          >
            {expanded ? "▼" : "▶"}
          </button>
        )}
      </div>
      {details !== undefined && expanded && (
        <pre
          data-testid="tool-call-card-body"
          className="mt-2 max-h-[200px] overflow-y-auto text-[11px] font-mono text-fg whitespace-pre-wrap"
        >
          {details}
        </pre>
      )}
    </div>
  );
}
