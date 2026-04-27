import { renderHook, waitFor } from "@testing-library/react";
import { useFindings } from "../hooks/useFindings";
import type { Finding } from "../types/finding";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function makeFinding(id: string, runId: string): Finding {
  return {
    id,
    run_id: runId,
    step_id: "step1",
    severity: "warning",
    file_path: null,
    line: null,
    message: `msg-${id}`,
    suggestion: null,
    triage: null,
    triaged_at: null,
    created_at: 100,
  };
}

describe("useFindings", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("does not call list_findings when runId is undefined", async () => {
    const { result } = renderHook(() => useFindings(undefined));
    // Give the effect a tick to no-op.
    await new Promise((r) => setTimeout(r, 20));
    expect(invokeMock).not.toHaveBeenCalled();
    expect(result.current.findings).toEqual([]);
  });

  it("invokes list_findings with the runId and stores the result", async () => {
    invokeMock.mockResolvedValueOnce([makeFinding("f1", "run-a"), makeFinding("f2", "run-a")]);

    const { result } = renderHook(() => useFindings("run-a"));

    await waitFor(() => {
      expect(result.current.findings).toHaveLength(2);
    });
    expect(invokeMock).toHaveBeenCalledWith("list_findings", { runId: "run-a" });
    expect(result.current.findings[0].id).toBe("f1");
  });

  it("clears prior findings when runId changes and refetches for the new run", async () => {
    invokeMock.mockImplementation(async (_cmd: string, args?: { runId?: string }) => {
      if (args?.runId === "run-a") return [makeFinding("a1", "run-a")];
      if (args?.runId === "run-b") return [makeFinding("b1", "run-b")];
      return [];
    });

    const { result, rerender } = renderHook(({ runId }) => useFindings(runId), {
      initialProps: { runId: "run-a" as string | undefined },
    });

    await waitFor(() => {
      expect(result.current.findings).toHaveLength(1);
    });
    expect(result.current.findings[0].id).toBe("a1");

    rerender({ runId: "run-b" });

    await waitFor(() => {
      expect(result.current.findings[0]?.id).toBe("b1");
    });
  });

  it("surfaces errors from list_findings on the error field", async () => {
    invokeMock.mockRejectedValueOnce("backend exploded");

    const { result } = renderHook(() => useFindings("run-a"));

    await waitFor(() => {
      expect(result.current.error).toBe("backend exploded");
    });
    expect(result.current.findings).toEqual([]);
  });
});
