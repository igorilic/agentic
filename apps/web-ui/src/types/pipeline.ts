import type { StepInfo, StepStatus } from "./run";

export type AgentLibraryEntry = {
  id: string;
  name: string;
  icon: string;
  desc: string;
};

export const AGENT_LIBRARY: readonly AgentLibraryEntry[] = [
  { id: "architect",  name: "Architect",     icon: "blueprint", desc: "Designs system & breaks down work" },
  { id: "developer",      name: "Developer",     icon: "code",      desc: "Writes code & tests" },
  { id: "tdd-developer", name: "Developer",     icon: "code",      desc: "Writes code & tests (TDD)" },
  { id: "qa",         name: "QA",            icon: "check",     desc: "Runs tests, checks edge cases" },
  { id: "reviewer",   name: "Reviewer",      icon: "eye",       desc: "Code review & feedback" },
  { id: "researcher", name: "Researcher",    icon: "book",      desc: "Gathers context, reads docs" },
  { id: "security",   name: "Security",      icon: "shield",    desc: "Audits for vulnerabilities" },
  { id: "perf",       name: "Performance",   icon: "gauge",     desc: "Profiles & optimizes hot paths" },
  { id: "docs",       name: "Doc Writer",    icon: "doc",       desc: "Updates README, API docs" },
  { id: "designer",   name: "Designer",      icon: "palette",   desc: "UX & visual review" },
  { id: "db",         name: "DB Migrator",   icon: "database",  desc: "Schema migrations & data" },
  { id: "devops",     name: "DevOps",        icon: "cloud",     desc: "CI/CD & deploy config" },
  { id: "a11y",       name: "Accessibility", icon: "a11y",      desc: "WCAG compliance pass" },
] as const;

export type RunStateOverall = "idle" | "running" | "completed" | "failed";
export type AgentStatus = "queued" | "active" | "done" | "skipped" | "errored" | "failed";

export interface AgentInstance {
  id: string;
  status: AgentStatus;
  startedAt?: number;
  endedAt?: number;
}

export interface PermissionRequest {
  requestId: string;
  agent: string;
  tool: string;
  arg: string;
  scope: "shell.destructive" | "fs.write" | "network.outbound" | string;
  risk: "low" | "medium" | "high";
  reason: string;
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
