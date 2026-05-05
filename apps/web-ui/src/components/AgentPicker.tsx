import { useState } from "react";
import AgentIcon from "./AgentIcon";
import { getAgentAccent } from "../utils/agentAccents";
import { useDiscoverableAgents } from "../hooks/useDiscoverableAgents";

export type AgentPickerProps = {
  excludeIds: string[];
  onPick: (agentId: string) => void;
  onClose: () => void;
  width?: "default" | "narrow";
  initialQuery?: string;
};

export default function AgentPicker({ excludeIds, onPick, onClose, width = "default", initialQuery = "" }: AgentPickerProps) {
  const [query, setQuery] = useState(initialQuery);
  const { agents, isLoading, error } = useDiscoverableAgents();

  const visible = agents
    .filter((a) => !excludeIds.includes(a.name))
    .filter((a) => {
      const q = query.trim().toLowerCase();
      if (q === "") return true;
      return a.name.toLowerCase().includes(q) || (a.description ?? "").toLowerCase().includes(q);
    });

  return (
    <div
      data-testid="agent-picker"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onClose();
        }
      }}
      role="dialog"
      aria-label="Pick an agent"
      className={`${width === "narrow" ? "w-60" : "w-80"} rounded-xl border border-[rgb(0_0_0_/_0.08)] bg-bg-surface shadow-popover`}
    >
      <div className="border-b border-border-soft p-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search agents…"
          className="w-full rounded-md border border-border bg-bg-surface px-2 py-1.5 text-[13px] font-sans focus:outline-none focus:ring-2 focus:ring-blue-500"
          autoFocus
        />
      </div>
      <ul className="max-h-72 overflow-y-auto py-1">
        {isLoading && (
          <li data-testid="agent-picker-loading" className="px-3 py-4 text-[13px] text-fg-muted text-center">
            Loading agents…
          </li>
        )}
        {!isLoading && error && (
          <li data-testid="agent-picker-error" className="px-3 py-4 text-[13px] text-red-600">
            Failed to list agents: {error}
          </li>
        )}
        {!isLoading && !error && visible.length === 0 && (
          <li data-testid="agent-picker-empty" className="px-3 py-4 text-[13px] text-fg-muted">
            No agents discovered. Run `agentic-cli init` or `agentic-cli init --copilot`.
          </li>
        )}
        {!isLoading && !error && visible.map((agent) => (
          <li key={agent.name}>
            <button
              type="button"
              data-testid={`agent-picker-row-${agent.name}`}
              onClick={() => onPick(agent.name)}
              className="flex w-full items-center gap-3 px-3 py-2 hover:bg-[rgb(0_0_0_/_0.04)] focus:bg-[rgb(0_0_0_/_0.04)] focus:outline-none text-left"
            >
              <span
                className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
                style={{ backgroundColor: getAgentAccent(agent.name).bg, color: getAgentAccent(agent.name).fg }}
                aria-hidden="true"
              >
                <AgentIcon agent={agent.name} size={16} />
              </span>
              <span className="flex flex-col leading-tight flex-1 min-w-0">
                <span className="flex items-center gap-2">
                  <span className="text-[13px] font-semibold text-fg">{agent.name}</span>
                  <span
                    data-testid={`agent-source-chip-${agent.name}`}
                    className={`text-[10px] border rounded px-1 py-0.5 leading-none ${agent.source === "project" ? "border-border" : "border-border-soft"} text-fg-muted`}
                  >
                    {agent.source}
                  </span>
                </span>
                {agent.description && (
                  <span className="text-[11px] text-fg-muted">{agent.description}</span>
                )}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
