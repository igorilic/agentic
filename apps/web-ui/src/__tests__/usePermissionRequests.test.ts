import { renderHook } from "@testing-library/react";
import { vi } from "vitest";
import { usePermissionRequests } from "../hooks/usePermissionRequests";

vi.mock("../hooks/useTauriEvents", () => ({
  useTauriEvents: vi.fn(),
}));

import { useTauriEvents } from "../hooks/useTauriEvents";

const mockUseTauriEvents = useTauriEvents as ReturnType<typeof vi.fn>;

function makeRequestEnvelope(
  requestId: string,
  overrides: Partial<{
    agent: string;
    tool: string;
    arg: string;
    scope: string;
    risk: "low" | "medium" | "high";
    reason: string;
    run_id: string;
  }> = {},
) {
  return {
    schema_version: 1,
    event_id: `evt-${requestId}`,
    run_id: overrides.run_id ?? "run-1",
    step_id: null,
    timestamp_ms: Date.now(),
    event: {
      type: "PermissionRequest",
      data: {
        request_id: requestId,
        agent: overrides.agent ?? "developer",
        tool: overrides.tool ?? "Bash",
        arg: overrides.arg ?? "test command",
        scope: overrides.scope ?? "shell",
        risk: overrides.risk ?? "low",
        reason: overrides.reason ?? "test reason",
      },
    },
  };
}

function makeResolvedEnvelope(requestId: string, run_id = "run-1") {
  return {
    schema_version: 1,
    event_id: `evt-resolved-${requestId}`,
    run_id,
    step_id: null,
    timestamp_ms: Date.now(),
    event: {
      type: "PermissionResolved",
      data: {
        request_id: requestId,
        decision: "allow_once" as const,
        source: "user" as const,
      },
    },
  };
}

function makeTextDeltaEnvelope(id: string) {
  return {
    schema_version: 1,
    event_id: `evt-td-${id}`,
    run_id: "run-1",
    step_id: null,
    timestamp_ms: Date.now(),
    event: {
      type: "TextDelta",
      data: { content: "some text" },
    },
  };
}

describe("usePermissionRequests", () => {
  beforeEach(() => {
    mockUseTauriEvents.mockReset();
  });

  it("tracks_a_pending_request", () => {
    mockUseTauriEvents.mockReturnValue({
      events: [makeRequestEnvelope("r1")],
      historyError: null,
    });

    const { result } = renderHook(() => usePermissionRequests());

    expect(result.current).toHaveLength(1);
    expect(result.current[0]).toMatchObject({
      requestId: "r1",
      agent: "developer",
      tool: "Bash",
      arg: "test command",
      scope: "shell",
      risk: "low",
      reason: "test reason",
    });
  });

  it("removes_request_on_resolved", () => {
    mockUseTauriEvents.mockReturnValue({
      events: [makeRequestEnvelope("r1"), makeResolvedEnvelope("r1")],
      historyError: null,
    });

    const { result } = renderHook(() => usePermissionRequests());

    expect(result.current).toHaveLength(0);
  });

  it("dedups_duplicate_request_envelopes", () => {
    mockUseTauriEvents.mockReturnValue({
      events: [makeRequestEnvelope("r1"), makeRequestEnvelope("r1")],
      historyError: null,
    });

    const { result } = renderHook(() => usePermissionRequests());

    expect(result.current).toHaveLength(1);
    expect(result.current[0].requestId).toBe("r1");
  });

  it("preserves_order_by_arrival", () => {
    mockUseTauriEvents.mockReturnValue({
      events: [makeRequestEnvelope("r1"), makeRequestEnvelope("r2")],
      historyError: null,
    });

    const { result } = renderHook(() => usePermissionRequests());

    expect(result.current).toHaveLength(2);
    expect(result.current[0].requestId).toBe("r1");
    expect(result.current[1].requestId).toBe("r2");
  });

  it("clears_on_run_change", () => {
    // First render: upstream has run-1's request
    mockUseTauriEvents.mockReturnValue({
      events: [makeRequestEnvelope("r1", { run_id: "run-1" })],
      historyError: null,
    });

    const { result, rerender } = renderHook(() => usePermissionRequests());

    expect(result.current).toHaveLength(1);
    expect(result.current[0].requestId).toBe("r1");

    // useTauriEvents clears on run change — simulate by returning empty events
    mockUseTauriEvents.mockReturnValue({
      events: [],
      historyError: null,
    });

    rerender();

    // Hook should reflect the empty upstream — no special code needed
    expect(result.current).toHaveLength(0);
  });

  it("unrelated_events_are_ignored", () => {
    mockUseTauriEvents.mockReturnValue({
      events: [
        makeRequestEnvelope("r1"),
        makeTextDeltaEnvelope("td-1"),
        makeRequestEnvelope("r2"),
      ],
      historyError: null,
    });

    const { result } = renderHook(() => usePermissionRequests());

    expect(result.current).toHaveLength(2);
    expect(result.current[0].requestId).toBe("r1");
    expect(result.current[1].requestId).toBe("r2");
  });
});
