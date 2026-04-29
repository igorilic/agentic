import { Fragment } from "react";
import type { AgentStatus } from "../types/pipeline";
import AgentCard from "./AgentCard";
import Connector from "./Connector";

export type PipelineBarProps = {
  agents: string[];
  statuses: Record<string, AgentStatus>;
  activeIndex: number;
  // Reserved for downstream steps; accepted but not yet consumed:
  onReorder?: (from: number, to: number) => void;
  onInsert?: (atIndex: number, agentId?: string) => void;
  onRemove?: (atIndex: number) => void;
  onSkip?: (atIndex: number) => void;
};

export default function PipelineBar({
  agents,
  statuses,
  onInsert,
}: PipelineBarProps) {
  return (
    <div
      data-testid="pipeline-bar"
      className="flex h-[84px] items-center gap-3 bg-bg-surface border-b border-border-soft px-[18px]"
    >
      {agents.map((agent, i) => (
        <Fragment key={agent}>
          <AgentCard agent={agent} status={statuses[agent] ?? "queued"} />
          {i < agents.length - 1 && (
            <>
              <button
                type="button"
                data-testid={`pipeline-insert-${i + 1}`}
                aria-label={`Insert agent at position ${i + 1}`}
                onClick={() => onInsert?.(i + 1)}
                className="opacity-0 hover:opacity-100 focus:opacity-100 transition-opacity h-4 w-4 rounded-full border border-border-strong text-fg-muted text-[11px] leading-none flex items-center justify-center"
              >
                +
              </button>
              <Connector active={false} />
            </>
          )}
        </Fragment>
      ))}
      <button
        type="button"
        data-testid="pipeline-add-agent"
        onClick={() => onInsert?.(agents.length)}
        className="rounded-md border border-dashed border-border-strong px-3 py-1.5 text-xs font-semibold text-fg-muted"
      >
        + Add agent
      </button>
    </div>
  );
}
