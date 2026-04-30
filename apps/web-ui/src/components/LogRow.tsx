// Per-agent Tailwind name classes — literal strings required for JIT scanner.
const AGENT_COLOR_CLASS: Record<string, string> = {
  architect: "text-agent-architect",
  developer: "text-agent-developer",
  qa: "text-agent-qa",
  reviewer: "text-agent-reviewer",
};

export type LogLevel = "info" | "status" | "error";

export type LogRowProps = {
  level: LogLevel;
  t: string;
  agent: string;
  message: string;
};

export default function LogRow({ level, t, agent, message }: LogRowProps) {
  const agentClass = AGENT_COLOR_CLASS[agent] ?? "text-fg";
  return (
    <div
      data-testid={`log-row-${level}`}
      className="flex items-baseline gap-2 font-mono text-[12px]"
    >
      <span className="text-fg-subtle">[{t}]</span>
      {level === "error" && (
        <span
          data-testid="log-row-level-chip"
          className="rounded bg-red-500 px-1 py-0.5 text-[10px] font-semibold uppercase text-white"
        >
          ERROR
        </span>
      )}
      <span data-testid="log-row-agent" className={`font-semibold ${agentClass}`}>
        {agent}
      </span>
      <span className="text-fg">{message}</span>
    </div>
  );
}
