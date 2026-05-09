import { renderHook, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { usePipelinePresets } from "../hooks/usePipelinePresets";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function makeWirePreset(id: string, name: string, agents: string[]) {
  return {
    id,
    name,
    agents,
    created_at: 1000,
    updated_at: 2000,
  };
}

describe("usePipelinePresets", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("list returns presets from IPC, snake_case -> camelCase converted", async () => {
    invokeMock.mockResolvedValueOnce([
      makeWirePreset("p1", "My Preset", ["architect", "qa"]),
    ]);

    const { result } = renderHook(() => usePipelinePresets());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(invokeMock).toHaveBeenCalledWith("list_pipeline_presets");
    expect(result.current.presets).toHaveLength(1);
    const preset = result.current.presets[0];
    expect(preset.id).toBe("p1");
    expect(preset.name).toBe("My Preset");
    expect(preset.agents).toEqual(["architect", "qa"]);
    expect(preset.createdAt).toBe(1000);
    expect(preset.updatedAt).toBe(2000);
    // snake_case keys must NOT appear
    expect((preset as Record<string, unknown>)["created_at"]).toBeUndefined();
    expect((preset as Record<string, unknown>)["updated_at"]).toBeUndefined();
  });

  it("save calls invoke with name+agents and refetches the list", async () => {
    // Initial list fetch
    invokeMock.mockResolvedValueOnce([]);
    // save call
    invokeMock.mockResolvedValueOnce(makeWirePreset("p-new", "Draft", ["architect"]));
    // refetch after save
    invokeMock.mockResolvedValueOnce([makeWirePreset("p-new", "Draft", ["architect"])]);

    const { result } = renderHook(() => usePipelinePresets());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let saved: unknown;
    await act(async () => {
      saved = await result.current.save("Draft", ["architect"]);
    });

    expect(invokeMock).toHaveBeenCalledWith("save_pipeline_preset", {
      name: "Draft",
      agents: ["architect"],
    });
    // returned preset is camelCase-converted
    expect((saved as { createdAt: number }).createdAt).toBe(1000);
    // list was refetched
    expect(result.current.presets).toHaveLength(1);
  });

  it("update calls invoke with id+name+agents and refetches the list", async () => {
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "Old", ["qa"])]);
    invokeMock.mockResolvedValueOnce(makeWirePreset("p1", "New", ["qa", "reviewer"]));
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "New", ["qa", "reviewer"])]);

    const { result } = renderHook(() => usePipelinePresets());
    await waitFor(() => expect(result.current.loading).toBe(false));

    let updated: unknown;
    await act(async () => {
      updated = await result.current.update("p1", "New", ["qa", "reviewer"]);
    });

    expect(invokeMock).toHaveBeenCalledWith("update_pipeline_preset", {
      id: "p1",
      name: "New",
      agents: ["qa", "reviewer"],
    });
    expect((updated as { name: string }).name).toBe("New");
    expect(result.current.presets[0].agents).toEqual(["qa", "reviewer"]);
  });

  it("remove calls invoke with id and refetches the list", async () => {
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "ToDelete", ["qa"])]);
    invokeMock.mockResolvedValueOnce(undefined); // delete returns void
    invokeMock.mockResolvedValueOnce([]); // refetch returns empty

    const { result } = renderHook(() => usePipelinePresets());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.presets).toHaveLength(1);

    await act(async () => {
      await result.current.remove("p1");
    });

    expect(invokeMock).toHaveBeenCalledWith("delete_pipeline_preset", { id: "p1" });
    expect(result.current.presets).toHaveLength(0);
  });

  it("error from invoke surfaces on .error and clears on next successful refresh", async () => {
    invokeMock.mockRejectedValueOnce("IPC exploded");

    const { result } = renderHook(() => usePipelinePresets());

    await waitFor(() => {
      expect(result.current.error).toBe("IPC exploded");
    });
    expect(result.current.presets).toEqual([]);

    // Second call succeeds — error should clear
    invokeMock.mockResolvedValueOnce([makeWirePreset("p1", "OK", ["qa"])]);
    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.presets).toHaveLength(1);
  });
});
