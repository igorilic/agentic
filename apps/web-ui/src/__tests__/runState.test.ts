import { describe, expect, it } from "vitest";
import {
  DEFAULT_AGENTS,
  deriveRunState,
  emptyRunState,
} from "../types/run";
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

describe("deriveRunState", () => {
  it("returns all-pending state for empty events", () => {
    const state = deriveRunState([]);
    expect(state.steps).toHaveLength(4);
    expect(state.steps.every((s) => s.status === "pending")).toBe(true);
    expect(state.totalTokens).toBe(0);
    expect(state.totalCostUsd).toBe(0);
  });

  it("transitions step to running on StepStarted", () => {
    const state = deriveRunState([
      envelope({
        step_id: "s1",
        event: {
          type: "StepStarted",
          data: { agent: "architect", model: { id: "m" } },
        },
      }),
    ]);
    expect(state.steps[0].status).toBe("running");
    expect(state.steps[1].status).toBe("pending");
  });

  it("transitions step to passed on StepComplete with status=passed", () => {
    const state = deriveRunState([
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
    ]);
    expect(state.steps[2].status).toBe("passed");
    expect(state.steps[2].tokens).toBe(30);
    expect(state.steps[2].costUsd).toBe(0.001);
    expect(state.steps[2].durationMs).toBe(50);
  });

  it("sums totalTokens and totalCostUsd across steps", () => {
    const state = deriveRunState([
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
    ]);
    expect(state.totalTokens).toBe(150);
    expect(state.totalCostUsd).toBe(0.015);
  });

  it("ignores events for unknown agents", () => {
    const state = deriveRunState([
      envelope({
        step_id: "s1",
        event: {
          type: "StepStarted",
          data: { agent: "non-existent-agent", model: {} },
        },
      }),
    ]);
    // All four still pending.
    expect(state.steps.every((s) => s.status === "pending")).toBe(true);
  });

  it("DEFAULT_AGENTS has the expected 4 agents in order", () => {
    expect(DEFAULT_AGENTS).toEqual([
      "architect",
      "tdd-developer",
      "qa",
      "reviewer",
    ]);
  });

  it("emptyRunState honors a custom agent list", () => {
    const state = emptyRunState(["a", "b"]);
    expect(state.steps).toHaveLength(2);
    expect(state.steps[0].agent).toBe("a");
  });
});
