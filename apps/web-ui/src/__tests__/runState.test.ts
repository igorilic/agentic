import { describe, expect, it } from "vitest";
import {
  deriveRunState,
  emptyRunState,
} from "../types/run";
import type { StepInfo } from "../types/run";
import type { EventEnvelope } from "../types/event";

function envelope(overrides: Partial<EventEnvelope>): EventEnvelope {
  return {
    schema_version: 1,
    event_id: "e",
    run_id: "r",
    step_id: null,
    timestamp_ms: 0,
    event: { type: "TextDelta", data: { content: "" } },
    ...overrides,
  };
}

// Local constant — DEFAULT_AGENTS no longer exported from run.ts (I.7)
const DEFAULT_AGENTS = ["architect", "tdd-developer", "qa", "reviewer"];

describe("deriveRunState", () => {
  it("returns all-pending state for empty events (with explicit agents)", () => {
    const state = deriveRunState([], undefined, DEFAULT_AGENTS);
    expect(state.steps).toHaveLength(4);
    expect(state.steps.every((s) => s.status === "pending")).toBe(true);
    expect(state.totalTokens).toBe(0);
    expect(state.totalCostUsd).toBe(0);
  });

  it("transitions step to running on StepStarted", () => {
    const state = deriveRunState(
      [
        envelope({
          step_id: "s1",
          event: {
            type: "StepStarted",
            data: { agent: "architect", model: { id: "m" } },
          },
        }),
      ],
      undefined,
      DEFAULT_AGENTS,
    );
    expect(state.steps[0].status).toBe("running");
    expect(state.steps[1].status).toBe("pending");
  });

  it("transitions step to passed on StepComplete with status=passed", () => {
    const state = deriveRunState(
      [
        envelope({
          step_id: "s1",
          event: {
            type: "StepStarted",
            data: { agent: "qa", model: { id: "m" } },
          },
        }),
        envelope({
          step_id: "s1",
          event: {
            type: "StepComplete",
            data: {
              status: "passed",
              summary: "ok",
              token_usage: { input_tokens: 10, output_tokens: 20 },
              cost_usd: 0.001,
              duration_ms: 50,
            },
          },
        }),
      ],
      undefined,
      DEFAULT_AGENTS,
    );
    expect(state.steps[2].status).toBe("passed");
    expect(state.steps[2].tokens).toBe(30);
    expect(state.steps[2].costUsd).toBe(0.001);
    expect(state.steps[2].durationMs).toBe(50);
  });

  it("sums totalTokens and totalCostUsd across steps", () => {
    const state = deriveRunState(
      [
        envelope({
          step_id: "s1",
          event: {
            type: "StepStarted",
            data: { agent: "architect", model: {} },
          },
        }),
        envelope({
          step_id: "s1",
          event: {
            type: "StepComplete",
            data: {
              status: "passed",
              summary: "",
              token_usage: { input_tokens: 100, output_tokens: 0 },
              cost_usd: 0.01,
              duration_ms: 1,
            },
          },
        }),
        envelope({
          step_id: "s2",
          event: {
            type: "StepStarted",
            data: { agent: "qa", model: {} },
          },
        }),
        envelope({
          step_id: "s2",
          event: {
            type: "StepComplete",
            data: {
              status: "failed",
              summary: "",
              token_usage: { input_tokens: 0, output_tokens: 50 },
              cost_usd: 0.005,
              duration_ms: 1,
            },
          },
        }),
      ],
      undefined,
      DEFAULT_AGENTS,
    );
    expect(state.totalTokens).toBe(150);
    expect(state.totalCostUsd).toBe(0.015);
  });

  it("ignores events for unknown agents", () => {
    const state = deriveRunState(
      [
        envelope({
          step_id: "s1",
          event: {
            type: "StepStarted",
            data: { agent: "non-existent-agent", model: {} },
          },
        }),
      ],
      undefined,
      DEFAULT_AGENTS,
    );
    // All four still pending.
    expect(state.steps.every((s) => s.status === "pending")).toBe(true);
  });

  it("captures needs_triage status from StepComplete", () => {
    const state = deriveRunState(
      [
        envelope({
          step_id: "s1",
          event: {
            type: "StepStarted",
            data: { agent: "qa", model: { id: "m" } },
          },
        }),
        envelope({
          step_id: "s1",
          event: {
            type: "StepComplete",
            data: {
              status: "needs_triage",
              summary: "Reviewer flagged 3 issues",
              token_usage: { input_tokens: 0, output_tokens: 0 },
              cost_usd: null,
              duration_ms: 100,
            },
          },
        }),
      ],
      undefined,
      DEFAULT_AGENTS,
    );
    expect(state.steps[2].status).toBe("needs_triage");
  });

  it("includes cache tokens in the per-step total", () => {
    const state = deriveRunState(
      [
        envelope({
          step_id: "s1",
          event: {
            type: "StepStarted",
            data: { agent: "architect", model: {} },
          },
        }),
        envelope({
          step_id: "s1",
          event: {
            type: "StepComplete",
            data: {
              status: "passed",
              summary: "",
              token_usage: {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_input_tokens: 200,
                cache_creation_input_tokens: 300,
              },
              cost_usd: null,
              duration_ms: 1,
            },
          },
        }),
      ],
      undefined,
      DEFAULT_AGENTS,
    );
    // 100 + 50 + 200 + 300 = 650
    expect(state.steps[0].tokens).toBe(650);
    expect(state.totalTokens).toBe(650);
  });

  it("emptyRunState requires an explicit agent list", () => {
    const state = emptyRunState(DEFAULT_AGENTS);
    expect(state.steps).toHaveLength(4);
    expect(state.steps.map((s) => s.agent)).toEqual(DEFAULT_AGENTS);
  });

  it("emptyRunState honors a custom agent list", () => {
    const state = emptyRunState(["a", "b"]);
    expect(state.steps).toHaveLength(2);
    expect(state.steps[0].agent).toBe("a");
  });

  // I.7 — emptyRunState with empty list produces zero steps
  it("emptyRunState with empty list produces zero steps", () => {
    const state = emptyRunState([]);
    expect(state.steps).toHaveLength(0);
    expect(state.totalTokens).toBe(0);
    expect(state.totalCostUsd).toBe(0);
  });

  // Issue 1: deriveRunState must seed steps from RunStarted.data.agents when present
  it("seeds steps from RunStarted.data.agents when present", () => {
    const events: EventEnvelope[] = [
      envelope({
        step_id: null,
        event: {
          type: "RunStarted",
          data: { agents: ["a", "b", "c"] },
        },
      }),
      envelope({
        step_id: "s1",
        event: { type: "StepStarted", data: { agent: "a", model: "m" } },
      }),
    ];
    const state = deriveRunState(events);
    expect(state.steps).toHaveLength(3);
    expect(state.steps[0].agent).toBe("a");
    expect(state.steps[0].status).toBe("running");
    expect(state.steps[1].agent).toBe("b");
    expect(state.steps[1].status).toBe("pending");
    expect(state.steps[2].agent).toBe("c");
    expect(state.steps[2].status).toBe("pending");
  });

  it("falls back to passed agents when RunStarted lacks agents (legacy event)", () => {
    const events: EventEnvelope[] = [
      envelope({
        step_id: null,
        // Legacy RunStarted with no agents field
        event: { type: "RunStarted", data: { ticket: "ABC-1" } },
      }),
    ];
    const state = deriveRunState(events, undefined, ["a", "b"]);
    expect(state.steps).toHaveLength(2);
    expect(state.steps[0].agent).toBe("a");
    expect(state.steps[1].agent).toBe("b");
  });

  it("returns empty steps when no RunStarted and no agents arg", () => {
    const events: EventEnvelope[] = [
      envelope({ step_id: null, event: { type: "TextDelta", data: { content: "hi" } } }),
    ];
    const state = deriveRunState(events);
    expect(state.steps).toHaveLength(0);
  });

  it("RunStarted agents with empty array produces zero steps (no fallback to arg)", () => {
    // Explicit [] in RunStarted means the run has no steps — don't override with agents param
    const events: EventEnvelope[] = [
      envelope({
        step_id: null,
        event: { type: "RunStarted", data: { agents: [] } },
      }),
    ];
    const state = deriveRunState(events, undefined, ["a", "b"]);
    // An explicit empty agents array in the event should NOT use the fallback.
    // This tests the distinction between "no agents field" (fallback applies)
    // and "agents: []" (run genuinely has no agents configured).
    expect(state.steps).toHaveLength(0);
  });

  // I.9 — StepInfo contract: agent is the identity field, role must not exist
  it("StepInfo has no role field", () => {
    const step: StepInfo = {
      agent: "x",
      status: "pending",
      tokens: 0,
      costUsd: null,
      durationMs: 0,
      summary: null,
    };
    // @ts-expect-error — StepInfo must not have a `role` field
    const _check = step.role;
    expect(_check).toBeUndefined();
  });

  // GH #66 — multi-run safety: deriveRunState should filter by activeRunId

  // M1 — multi-run isolation: only r1 events affect r1 state
  // Both runs fire StepStarted + StepComplete for the SAME agent ("architect")
  // so that removing the activeRunId filter would cause r2's "failed" result
  // to overwrite r1's "passed" on the shared slot — a real correctness failure.
  it("M1: filters to activeRunId — only matching run events applied", () => {
    const events: EventEnvelope[] = [
      // r1: architect started and completed passed
      envelope({ run_id: "r1", step_id: "s-r1-arch", event: { type: "StepStarted", data: { agent: "architect", model: {} } } }),
      envelope({ run_id: "r1", step_id: "s-r1-arch", event: { type: "StepComplete", data: { status: "passed", summary: "", token_usage: { input_tokens: 0, output_tokens: 0 }, cost_usd: null, duration_ms: 1 } } }),
      // r2: architect started and completed failed (SAME agent, different step_id)
      // Without the filter, this would overwrite r1's "passed" with "failed".
      envelope({ run_id: "r2", step_id: "s-r2-arch", event: { type: "StepStarted", data: { agent: "architect", model: {} } } }),
      envelope({ run_id: "r2", step_id: "s-r2-arch", event: { type: "StepComplete", data: { status: "failed", summary: "", token_usage: { input_tokens: 0, output_tokens: 0 }, cost_usd: null, duration_ms: 1 } } }),
    ];
    const state = deriveRunState(events, "r1", ["architect", "tdd-developer"]);
    expect(state.steps[0].status).toBe("passed");   // architect — only r1 effect applied
    expect(state.steps[1].status).toBe("pending");  // tdd-developer — neither run touched
  });

  // M2 — activeRunId undefined preserves legacy behavior: all events processed
  it("M2: activeRunId undefined applies all events (back-compat)", () => {
    const events: EventEnvelope[] = [
      envelope({ run_id: "r1", step_id: "s1", event: { type: "StepStarted", data: { agent: "architect", model: {} } } }),
      envelope({ run_id: "r1", step_id: "s1", event: { type: "StepComplete", data: { status: "passed", summary: "", token_usage: { input_tokens: 0, output_tokens: 0 }, cost_usd: null, duration_ms: 1 } } }),
      envelope({ run_id: "r2", step_id: "s2", event: { type: "StepStarted", data: { agent: "tdd-developer", model: {} } } }),
      envelope({ run_id: "r2", step_id: "s2", event: { type: "StepComplete", data: { status: "failed", summary: "", token_usage: { input_tokens: 0, output_tokens: 0 }, cost_usd: null, duration_ms: 1 } } }),
    ];
    const state = deriveRunState(events, undefined, ["architect", "tdd-developer"]);
    const architect = state.steps.find((s) => s.agent === "architect");
    const tddDev = state.steps.find((s) => s.agent === "tdd-developer");
    expect(architect?.status).toBe("passed");    // r1 effect visible
    expect(tddDev?.status).toBe("failed");       // r2 effect also visible — no filter
  });

  // M3 — agents resolved from RunStarted respect the activeRunId filter
  it("M3: RunStarted from another run must not contribute agents when activeRunId is set", () => {
    const events: EventEnvelope[] = [
      // r1 RunStarted declares agents ["a", "b"]
      envelope({ run_id: "r1", step_id: null, event: { type: "RunStarted", data: { agents: ["a", "b"] } } }),
      // r2 RunStarted declares agents ["x", "y"] — must be ignored when filtering to r1
      envelope({ run_id: "r2", step_id: null, event: { type: "RunStarted", data: { agents: ["x", "y"] } } }),
    ];
    const state = deriveRunState(events, "r1");
    expect(state.steps).toHaveLength(2);
    expect(state.steps.map((s) => s.agent)).toEqual(["a", "b"]);
  });
});
