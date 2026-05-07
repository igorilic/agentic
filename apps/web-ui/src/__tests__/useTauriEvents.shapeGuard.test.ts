/**
 * Shape-guard tests for isEventEnvelope / isEventEnvelopeArray (GH #62).
 *
 * Tests G1-G6 exercise the predicates in isolation.
 * Test H1 exercises the hook wiring: a malformed invoke response must surface
 * via historyError rather than crashing downstream code.
 */
import { renderHook, waitFor } from "@testing-library/react";
import { isEventEnvelope, isEventEnvelopeArray } from "../types/event";
import { useTauriEvents } from "../hooks/useTauriEvents";

// ─── hook mock infra (mirrors useTauriEvents.test.ts) ────────────────────────

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_channel: string, _handler: unknown) => () => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// ─── helpers ─────────────────────────────────────────────────────────────────

const wellFormed = {
  schema_version: 1,
  event_id: "e1",
  run_id: "r1",
  step_id: null,
  timestamp_ms: 1000,
  event: { type: "TextDelta", data: { content: "hello" } },
};

// ─── G1: predicate accepts well-formed envelope ──────────────────────────────

describe("isEventEnvelope", () => {
  it("G1 — accepts a well-formed envelope", () => {
    expect(isEventEnvelope(wellFormed)).toBe(true);
  });

  it("G1 — step_id may be a string", () => {
    expect(isEventEnvelope({ ...wellFormed, step_id: "step-1" })).toBe(true);
  });

  // G2 — non-objects
  it("G2 — rejects null", () => {
    expect(isEventEnvelope(null)).toBe(false);
  });

  it("G2 — rejects a string", () => {
    expect(isEventEnvelope("foo")).toBe(false);
  });

  it("G2 — rejects a number", () => {
    expect(isEventEnvelope(1)).toBe(false);
  });

  // G3 — missing event_id
  it("G3 — rejects envelope missing event_id", () => {
    const { event_id: _omit, ...rest } = wellFormed;
    expect(isEventEnvelope(rest)).toBe(false);
  });

  // G4 — wrong-typed fields
  it("G4 — rejects event_id set to a number", () => {
    expect(isEventEnvelope({ ...wellFormed, event_id: 42 })).toBe(false);
  });

  it("G4 — rejects event_id set to empty string", () => {
    expect(isEventEnvelope({ ...wellFormed, event_id: "" })).toBe(false);
  });

  it("G4 — rejects run_id set to null", () => {
    expect(isEventEnvelope({ ...wellFormed, run_id: null })).toBe(false);
  });

  it("G4 — rejects run_id set to a number", () => {
    expect(isEventEnvelope({ ...wellFormed, run_id: 42 })).toBe(false);
  });

  it("G4 — rejects schema_version set to a string", () => {
    expect(isEventEnvelope({ ...wellFormed, schema_version: "1" })).toBe(false);
  });

  it("G4 — rejects timestamp_ms set to a string", () => {
    expect(isEventEnvelope({ ...wellFormed, timestamp_ms: "1000" })).toBe(false);
  });

  it("G4 — rejects step_id set to undefined (must be string | null)", () => {
    // undefined is not acceptable per the contract
    const env = { ...wellFormed } as Record<string, unknown>;
    delete env["step_id"];
    expect(isEventEnvelope(env)).toBe(false);
  });

  // G5 — missing event.type
  it("G5 — rejects when event has no type field", () => {
    expect(isEventEnvelope({ ...wellFormed, event: { data: { content: "x" } } })).toBe(false);
  });

  it("G5 — rejects when event is not an object", () => {
    expect(isEventEnvelope({ ...wellFormed, event: "TextDelta" })).toBe(false);
  });
});

// ─── G6 + array tests ────────────────────────────────────────────────────────

describe("isEventEnvelopeArray", () => {
  it("G1 — accepts array of well-formed envelopes", () => {
    expect(isEventEnvelopeArray([wellFormed])).toBe(true);
  });

  it("G1 — accepts empty array", () => {
    expect(isEventEnvelopeArray([])).toBe(true);
  });

  it("G2 — rejects array containing null", () => {
    expect(isEventEnvelopeArray([null])).toBe(false);
  });

  it("G2 — rejects array containing a string", () => {
    expect(isEventEnvelopeArray(["foo"])).toBe(false);
  });

  it("G2 — rejects array containing a number", () => {
    expect(isEventEnvelopeArray([1])).toBe(false);
  });

  it("G6 — rejects a JSON string at the top level", () => {
    expect(isEventEnvelopeArray("[]")).toBe(false);
  });

  it("G6 — rejects a plain object at the top level", () => {
    expect(isEventEnvelopeArray({})).toBe(false);
  });

  it("G6 — rejects null at the top level", () => {
    expect(isEventEnvelopeArray(null)).toBe(false);
  });
});

// ─── H1: historyError set when invoke returns a non-array ─────────────────────

describe("useTauriEvents — malformed invoke response", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("H1 — historyError is set when get_event_history returns a non-array", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_event_history") return "not an array";
      if (cmd === "subscribe_events") return undefined;
      throw new Error(`unexpected invoke: ${cmd}`);
    });

    const { result } = renderHook(() => useTauriEvents("run-1"));

    await waitFor(() => {
      expect(result.current.historyError).not.toBeNull();
    });

    expect(result.current.historyError).toMatch(/malformed/i);
  });
});
