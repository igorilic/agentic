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
  onInsert?: (atIndex: number, agentId: string) => void;
  onRemove?: (atIndex: number) => void;
  onSkip?: (atIndex: number) => void;
};

export default function PipelineBar({ agents, statuses }: PipelineBarProps) {
  return (
    <div
      data-testid="pipeline-bar"
      className="flex h-[84px] items-center gap-3 bg-bg-surface border-b border-border-soft px-[18px]"
    >
      {agents.map((agent, i) => (
        <Fragment key={agent}>
          <AgentCard agent={agent} status={statuses[agent] ?? "queued"} />
          {i < agents.length - 1 && <Connector active={false} />}
        </Fragment>
      ))}
      <button
        type="button"
        data-testid="pipeline-add-agent"
        className="rounded-md border border-dashed border-border-strong px-3 py-1.5 text-xs font-semibold text-fg-muted"
      >
        + Add agent
      </button>
    </div>
  );
}
