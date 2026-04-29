import { useState } from "react";
import { AGENT_LIBRARY } from "../types/pipeline";

export type AgentPickerProps = {
  excludeIds: string[];
  onPick: (agentId: string) => void;
  onClose: () => void;
};

export default function AgentPicker({ excludeIds, onPick, onClose }: AgentPickerProps) {
  const [query, setQuery] = useState("");

  const visible = AGENT_LIBRARY.filter((a) => !excludeIds.includes(a.id)).filter((a) => {
    const q = query.trim().toLowerCase();
    if (q === "") return true;
    return a.name.toLowerCase().includes(q) || a.id.toLowerCase().includes(q);
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
      className="w-80 rounded-xl border border-[rgb(0_0_0_/_0.08)] bg-bg-surface shadow-modal"
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
        {visible.map((agent) => (
          <li key={agent.id}>
            <button
              type="button"
              data-testid={`agent-picker-row-${agent.id}`}
              onClick={() => onPick(agent.id)}
              className="flex w-full items-center gap-3 px-3 py-2 hover:bg-[rgb(0_0_0_/_0.04)] focus:bg-[rgb(0_0_0_/_0.04)] focus:outline-none text-left"
            >
              <span className="h-8 w-8 rounded-md bg-bg-surface-2" aria-hidden="true" />
              <span className="flex flex-col leading-tight">
                <span className="text-[13px] font-semibold text-fg">{agent.name}</span>
                <span className="text-[11px] text-fg-muted">{agent.desc}</span>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
