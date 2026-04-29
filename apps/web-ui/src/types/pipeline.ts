import type { StepInfo, StepStatus } from "./run";

export type RunStateOverall = "idle" | "running" | "completed" | "failed";
export type AgentStatus = "queued" | "active" | "done" | "skipped" | "errored" | "failed";

export interface AgentInstance {
  id: string;
  status: AgentStatus;
  startedAt?: number;
  endedAt?: number;
}

export interface PermissionRequest {
  id: string;
  agent: string;
  tool: string;
  arg: string;
  scope: "shell.destructive" | "fs.write" | "network.outbound" | string;
  risk: "low" | "medium" | "high";
  reason: string;
  t: number;
}

export interface ActionItem {
  id: string;
  kind: "issue" | "warning" | "followup";
  title: string;
  description?: string;
  fromAgent: string;
}

export interface IssueTicket {
  id: string;
  title: string;
  labels: string[];
  body: string[];
  acceptance: string[];
}

const STEP_STATUS_MAP: Record<StepStatus, AgentStatus> = {
  pending: "queued",
  running: "active",
  passed: "done",
  failed: "failed",
  needs_triage: "errored",
  skipped: "skipped",
};

export function agentInstanceFromStep(step: StepInfo): AgentInstance {
  return {
    id: step.agent,
    status: STEP_STATUS_MAP[step.status],
  };
}
