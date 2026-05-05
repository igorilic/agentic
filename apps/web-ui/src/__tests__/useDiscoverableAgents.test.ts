import { renderHook, act, waitFor } from "@testing-library/react";
import { useDiscoverableAgents } from "../hooks/useDiscoverableAgents";
import type { AgentInfoDto } from "../types/agents";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// Mock useBackend so we can control the backend value in tests
const mockUseBackend = vi.fn();
vi.mock("../hooks/useBackend", () => ({
  useBackend: () => mockUseBackend(),
}));

describe("useDiscoverableAgents", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    mockUseBackend.mockReturnValue({ backend: "claude-code", setBackend: vi.fn() });
  });

  // ---------------------------------------------------------------------------
  // Test 1: returns agents after invoke resolves
  // ---------------------------------------------------------------------------
  it("returns_agents_after_invoke_resolves", async () => {
    const fixtures: AgentInfoDto[] = [
      { name: "architect", description: "Plans the work", source: "project" },
    ];
    invokeMock.mockResolvedValue(fixtures);

    const { result } = renderHook(() => useDiscoverableAgents());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invokeMock).toHaveBeenCalledWith("list_agents", { backend: "claude-code" });
    expect(result.current.agents).toEqual(fixtures);
    expect(result.current.error).toBe(null);
  });

  // ---------------------------------------------------------------------------
  // Test 2: flips isLoading during fetch
  // ---------------------------------------------------------------------------
  it("flips_isLoading_during_fetch", async () => {
    let resolveInvoke!: (value: AgentInfoDto[]) => void;
    const deferred = new Promise<AgentInfoDto[]>((res) => {
      resolveInvoke = res;
    });
    invokeMock.mockReturnValue(deferred);

    const { result } = renderHook(() => useDiscoverableAgents());

    // isLoading should be true while in flight
    expect(result.current.isLoading).toBe(true);

    // Resolve the deferred promise
    await act(async () => {
      resolveInvoke([]);
      await deferred;
    });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
  });

  // ---------------------------------------------------------------------------
  // Test 3: surfaces error string
  // ---------------------------------------------------------------------------
  it("surfaces_error_string", async () => {
    invokeMock.mockRejectedValue("unknown backend: frobnicate");

    const { result } = renderHook(() => useDiscoverableAgents());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.error).toBe("unknown backend: frobnicate");
    expect(result.current.agents).toEqual([]);
  });

  // ---------------------------------------------------------------------------
  // Test 4: refetches when backend changes
  // ---------------------------------------------------------------------------
  it("refetches_when_backend_changes", async () => {
    invokeMock.mockResolvedValue([]);

    // Start with claude-code
    const { result, rerender } = renderHook(() => useDiscoverableAgents());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenLastCalledWith("list_agents", { backend: "claude-code" });

    // Switch to copilot-cli
    mockUseBackend.mockReturnValue({ backend: "copilot-cli", setBackend: vi.fn() });
    rerender();

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });

    expect(invokeMock).toHaveBeenLastCalledWith("list_agents", { backend: "copilot-cli" });
  });

  // ---------------------------------------------------------------------------
  // Test 5: refetch() manually re-invokes
  // ---------------------------------------------------------------------------
  it("refetch_manually_triggers_new_invoke", async () => {
    invokeMock.mockResolvedValue([]);

    const { result } = renderHook(() => useDiscoverableAgents());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.refetch();
    });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });
  });
});
