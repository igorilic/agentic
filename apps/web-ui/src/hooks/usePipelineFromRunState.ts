import { useMemo } from "react";
import type { RunState } from "../types/run";
import type { AgentStatus } from "../types/pipeline";
import { agentInstanceFromStep } from "../types/pipeline";

export type UsePipelineFromRunStateResult = {
  pipelineAgents: string[];
  pipelineStatuses: Record<string, AgentStatus>;
  activeIndex: number;
};

export function usePipelineFromRunState(runState: RunState): UsePipelineFromRunStateResult {
  const pipelineAgents = useMemo(() => runState.steps.map((s) => s.agent), [runState]);

  const pipelineStatuses: Record<string, AgentStatus> = useMemo(() => {
    const out: Record<string, AgentStatus> = {};
    for (const step of runState.steps) {
      out[step.agent] = agentInstanceFromStep(step).status;
    }
    return out;
  }, [runState]);

  const activeIndex = useMemo(
    () => runState.steps.findIndex((s) => s.status === "running"),
    [runState],
  );

  return { pipelineAgents, pipelineStatuses, activeIndex };
}
