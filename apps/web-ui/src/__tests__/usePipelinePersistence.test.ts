/**
 * I.7 — usePipelinePersistence hook unit tests
 *
 * Tests the hook in isolation: wsId=null behavior, parse-error recovery,
 * write-on-mutation.
 */
import { renderHook, act } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Import after any mocks are set up
import { usePipelinePersistence } from "../hooks/usePipelinePersistence";

const STORAGE_KEY = (wsId: string) => `agentic.pipeline.${wsId}`;

describe("usePipelinePersistence", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  // ── wsId = null ────────────────────────────────────────────────────────────

  it("returns empty pipelineAgents when wsId is null", () => {
    const { result } = renderHook(() => usePipelinePersistence(null));
    expect(result.current.pipelineAgents).toEqual([]);
  });

  it("setPipelineAgents is a no-op when wsId is null", () => {
    const { result } = renderHook(() => usePipelinePersistence(null));
    act(() => {
      result.current.setPipelineAgents(["architect"]);
    });
    // Still empty — noop, nothing written to storage
    expect(result.current.pipelineAgents).toEqual([]);
    expect(localStorage.getItem("agentic.pipeline.null")).toBeNull();
  });

  // ── happy path ─────────────────────────────────────────────────────────────

  it("returns [] when no key exists in localStorage", () => {
    const { result } = renderHook(() => usePipelinePersistence("ws-abc123"));
    expect(result.current.pipelineAgents).toEqual([]);
  });

  it("hydrates from localStorage on mount when key exists", () => {
    const stored = ["my-architect", "my-developer"];
    localStorage.setItem(STORAGE_KEY("ws-abc123"), JSON.stringify(stored));

    const { result } = renderHook(() => usePipelinePersistence("ws-abc123"));
    expect(result.current.pipelineAgents).toEqual(stored);
  });

  it("writes to localStorage when setPipelineAgents is called", () => {
    const { result } = renderHook(() => usePipelinePersistence("ws-abc123"));
    act(() => {
      result.current.setPipelineAgents(["architect", "qa"]);
    });
    expect(result.current.pipelineAgents).toEqual(["architect", "qa"]);
    const stored = JSON.parse(
      localStorage.getItem(STORAGE_KEY("ws-abc123"))!,
    ) as string[];
    expect(stored).toEqual(["architect", "qa"]);
  });

  it("overwrites localStorage on subsequent mutations", () => {
    const { result } = renderHook(() => usePipelinePersistence("ws-abc123"));
    act(() => {
      result.current.setPipelineAgents(["architect", "qa"]);
    });
    act(() => {
      result.current.setPipelineAgents(["reviewer"]);
    });
    const stored = JSON.parse(
      localStorage.getItem(STORAGE_KEY("ws-abc123"))!,
    ) as string[];
    expect(stored).toEqual(["reviewer"]);
  });

  // ── parse-error recovery ───────────────────────────────────────────────────

  it("returns [] and clears key on corrupt JSON", () => {
    localStorage.setItem(STORAGE_KEY("ws-abc123"), "{not json}");
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    const { result } = renderHook(() => usePipelinePersistence("ws-abc123"));
    expect(result.current.pipelineAgents).toEqual([]);
    expect(localStorage.getItem(STORAGE_KEY("ws-abc123"))).toBeNull();
    expect(warnSpy).toHaveBeenCalled();

    warnSpy.mockRestore();
  });

  it("returns [] and clears key when JSON is not an array", () => {
    localStorage.setItem(STORAGE_KEY("ws-abc123"), JSON.stringify({ oops: true }));
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    const { result } = renderHook(() => usePipelinePersistence("ws-abc123"));
    expect(result.current.pipelineAgents).toEqual([]);
    expect(localStorage.getItem(STORAGE_KEY("ws-abc123"))).toBeNull();
    expect(warnSpy).toHaveBeenCalled();

    warnSpy.mockRestore();
  });

  // ── wsId change re-reads storage ───────────────────────────────────────────

  it("reloads from different localStorage key when wsId changes", () => {
    localStorage.setItem(STORAGE_KEY("ws-111"), JSON.stringify(["architect"]));
    localStorage.setItem(STORAGE_KEY("ws-222"), JSON.stringify(["reviewer", "qa"]));

    const { result, rerender } = renderHook(
      ({ wsId }) => usePipelinePersistence(wsId),
      { initialProps: { wsId: "ws-111" as string | null } },
    );
    expect(result.current.pipelineAgents).toEqual(["architect"]);

    rerender({ wsId: "ws-222" });
    expect(result.current.pipelineAgents).toEqual(["reviewer", "qa"]);
  });

  it("returns [] when switching to a wsId with no storage entry", () => {
    localStorage.setItem(STORAGE_KEY("ws-111"), JSON.stringify(["architect"]));

    const { result, rerender } = renderHook(
      ({ wsId }) => usePipelinePersistence(wsId),
      { initialProps: { wsId: "ws-111" as string | null } },
    );
    expect(result.current.pipelineAgents).toEqual(["architect"]);

    rerender({ wsId: "ws-222" });
    expect(result.current.pipelineAgents).toEqual([]);
  });
});
