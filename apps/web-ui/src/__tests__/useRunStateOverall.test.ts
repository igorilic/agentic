import { renderHook } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { useRunStateOverall } from "../hooks/useRunStateOverall";
import type { EventEnvelope } from "../types/event";

function envelope(opts: {
  id: string;
  type: string;
  data?: unknown;
  t?: number;
  runId?: string;
}): EventEnvelope {
  return {
    schema_version: 1,
    event_id: opts.id,
    run_id: opts.runId ?? "run-1",
    step_id: null,
    timestamp_ms: opts.t ?? 1_700_000_000_000,
    event: { type: opts.type, data: opts.data },
  };
}

describe("useRunStateOverall", () => {
  it("idle when no events and no activeRunId", () => {
    const { result } = renderHook(() => useRunStateOverall([], undefined));
    expect(result.current.overallRunState).toBe("idle");
  });

  it("running when an event for the active run exists", () => {
    const events = [envelope({ id: "e1", type: "RunStarted" })];
    const { result } = renderHook(() => useRunStateOverall(events, "run-1"));
    expect(result.current.overallRunState).toBe("running");
  });

  it("completed on RunComplete with status completed", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: { status: "completed" } }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, "run-1"));
    expect(result.current.overallRunState).toBe("completed");
  });

  it("failed on RunComplete with status failed", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: { status: "failed" } }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, "run-1"));
    expect(result.current.overallRunState).toBe("failed");
  });

  it("failed on RunComplete with status cancelled", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: { status: "cancelled" } }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, "run-1"));
    expect(result.current.overallRunState).toBe("failed");
  });

  it("failed on RunComplete with status crashed", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: { status: "crashed" } }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, "run-1"));
    expect(result.current.overallRunState).toBe("failed");
  });

  it("completed when status field is missing on RunComplete", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: {} }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, "run-1"));
    expect(result.current.overallRunState).toBe("completed");
  });

  it("completed when activeRunId is undefined but a RunComplete (status: completed) is in history", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: { status: "completed" } }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, undefined));
    expect(result.current.overallRunState).toBe("completed");
  });

  it("failed when activeRunId is undefined but a RunComplete (status: failed) is in history", () => {
    const events = [
      envelope({ id: "e1", type: "RunStarted" }),
      envelope({ id: "e2", type: "RunComplete", data: { status: "failed" } }),
    ];
    const { result } = renderHook(() => useRunStateOverall(events, undefined));
    expect(result.current.overallRunState).toBe("failed");
  });
});
