import { describe, it, expect, expectTypeOf } from "vitest";
import {
  agentInstanceFromStep,
  type AgentInstance,
  type PermissionRequest,
  type ActionItem,
  type IssueTicket,
  type RunStateOverall,
  type AgentStatus,
} from "../types/pipeline";

// ---------------------------------------------------------------------------
// Type-level assertions
// ---------------------------------------------------------------------------

describe("AgentInstance type shape", () => {
  it("has id: string", () => {
    expectTypeOf<AgentInstance>().toHaveProperty("id").toEqualTypeOf<string>();
  });

  it("has status: AgentStatus", () => {
    expectTypeOf<AgentInstance>()
      .toHaveProperty("status")
      .toEqualTypeOf<AgentStatus>();
  });

  it("has optional startedAt: number", () => {
    expectTypeOf<AgentInstance>()
      .toHaveProperty("startedAt")
      .toEqualTypeOf<number | undefined>();
  });

  it("has optional endedAt: number", () => {
    expectTypeOf<AgentInstance>()
      .toHaveProperty("endedAt")
      .toEqualTypeOf<number | undefined>();
  });
});

describe("PermissionRequest type shape", () => {
  it("has id: string", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("id")
      .toEqualTypeOf<string>();
  });

  it("has agent: string", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("agent")
      .toEqualTypeOf<string>();
  });

  it("has tool: string", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("tool")
      .toEqualTypeOf<string>();
  });

  it("has arg: string", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("arg")
      .toEqualTypeOf<string>();
  });

  it("has scope: string union", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("scope")
      .toEqualTypeOf<"shell.destructive" | "fs.write" | "network.outbound" | string>();
  });

  it("has risk: low | medium | high", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("risk")
      .toEqualTypeOf<"low" | "medium" | "high">();
  });

  it("has reason: string", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("reason")
      .toEqualTypeOf<string>();
  });

  it("has t: number", () => {
    expectTypeOf<PermissionRequest>()
      .toHaveProperty("t")
      .toEqualTypeOf<number>();
  });
});

describe("ActionItem type shape", () => {
  it("has id: string", () => {
    expectTypeOf<ActionItem>().toHaveProperty("id").toEqualTypeOf<string>();
  });

  it("has kind: issue | warning | followup", () => {
    expectTypeOf<ActionItem>()
      .toHaveProperty("kind")
      .toEqualTypeOf<"issue" | "warning" | "followup">();
  });

  it("has title: string", () => {
    expectTypeOf<ActionItem>()
      .toHaveProperty("title")
      .toEqualTypeOf<string>();
  });

  it("has optional description: string", () => {
    expectTypeOf<ActionItem>()
      .toHaveProperty("description")
      .toEqualTypeOf<string | undefined>();
  });

  it("has fromAgent: string", () => {
    expectTypeOf<ActionItem>()
      .toHaveProperty("fromAgent")
      .toEqualTypeOf<string>();
  });
});

describe("IssueTicket type shape", () => {
  it("has id: string", () => {
    expectTypeOf<IssueTicket>().toHaveProperty("id").toEqualTypeOf<string>();
  });

  it("has title: string", () => {
    expectTypeOf<IssueTicket>()
      .toHaveProperty("title")
      .toEqualTypeOf<string>();
  });

  it("has labels: string[]", () => {
    expectTypeOf<IssueTicket>()
      .toHaveProperty("labels")
      .toEqualTypeOf<string[]>();
  });

  it("has body: string[]", () => {
    expectTypeOf<IssueTicket>()
      .toHaveProperty("body")
      .toEqualTypeOf<string[]>();
  });

  it("has acceptance: string[]", () => {
    expectTypeOf<IssueTicket>()
      .toHaveProperty("acceptance")
      .toEqualTypeOf<string[]>();
  });
});

describe("RunStateOverall type", () => {
  it("is the correct union", () => {
    expectTypeOf<RunStateOverall>().toEqualTypeOf<
      "idle" | "running" | "completed" | "failed"
    >();
  });
});

describe("AgentStatus type", () => {
  it("is the correct union", () => {
    expectTypeOf<AgentStatus>().toEqualTypeOf<
      "queued" | "active" | "done" | "skipped" | "errored" | "failed"
    >();
  });
});

// ---------------------------------------------------------------------------
// Runtime assertions for agentInstanceFromStep
// ---------------------------------------------------------------------------

const baseStep = {
  tokens: 0,
  costUsd: null,
  durationMs: 0,
  summary: null,
} as const;

describe("agentInstanceFromStep", () => {
  it("maps pending → queued", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "architect",
      status: "pending",
    });
    expect(result.status).toBe("queued");
  });

  it("maps running → active", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "tdd-developer",
      status: "running",
    });
    expect(result.status).toBe("active");
  });

  it("maps passed → done", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "qa",
      status: "passed",
    });
    expect(result.status).toBe("done");
  });

  it("maps failed → failed", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "reviewer",
      status: "failed",
    });
    expect(result.status).toBe("failed");
  });

  it("maps skipped → skipped", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "qa",
      status: "skipped",
    });
    expect(result.status).toBe("skipped");
  });

  it("maps needs_triage → errored", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "reviewer",
      status: "needs_triage",
    });
    expect(result.status).toBe("errored");
  });

  it("sets id from step agent field", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "developer",
      status: "pending",
    });
    expect(result.id).toBe("developer");
  });

  it("does not synthesize startedAt when durationMs is 0", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "architect",
      status: "pending",
    });
    expect(result.startedAt).toBeUndefined();
  });

  it("does not synthesize endedAt when durationMs is 0", () => {
    const result = agentInstanceFromStep({
      ...baseStep,
      agent: "architect",
      status: "pending",
    });
    expect(result.endedAt).toBeUndefined();
  });
});
