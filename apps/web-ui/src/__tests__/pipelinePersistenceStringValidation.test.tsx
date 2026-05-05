/**
 * I.7 fix-loop — TD2: Validate parsed entries are strings.
 *
 * After Array.isArray(parsed) check, also verify parsed.every(x => typeof x === "string").
 * If any element fails, treat as corrupt: clear the key, return [], console.warn.
 *
 * Test: clears storage and returns [] when parsed array contains non-string entries.
 */
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePipelinePersistence } from "../hooks/usePipelinePersistence";

const KEY = (wsId: string) => `agentic.pipeline.${wsId}`;

describe("TD2 — usePipelinePersistence: non-string element validation", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("clears storage and returns [] when parsed array contains non-string entries", () => {
    // Seed with an array containing non-string elements
    localStorage.setItem(KEY("ws-td2"), JSON.stringify([1, null, {}]));
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    const { result } = renderHook(() => usePipelinePersistence("ws-td2"));

    // Should return empty and clear the key
    expect(result.current.pipelineAgents).toEqual([]);
    expect(localStorage.getItem(KEY("ws-td2"))).toBeNull();
    // Should emit a console.warn
    expect(warnSpy).toHaveBeenCalled();

    warnSpy.mockRestore();
  });

  it("clears storage and returns [] when array has mixed valid and invalid entries", () => {
    // Seed with mix: one valid string, one invalid number
    localStorage.setItem(KEY("ws-td2-mixed"), JSON.stringify(["architect", 42]));
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    const { result } = renderHook(() => usePipelinePersistence("ws-td2-mixed"));

    expect(result.current.pipelineAgents).toEqual([]);
    expect(localStorage.getItem(KEY("ws-td2-mixed"))).toBeNull();
    expect(warnSpy).toHaveBeenCalled();

    warnSpy.mockRestore();
  });

  it("accepts a valid all-string array and returns it", () => {
    localStorage.setItem(KEY("ws-td2-valid"), JSON.stringify(["architect", "reviewer"]));

    const { result } = renderHook(() => usePipelinePersistence("ws-td2-valid"));

    // Valid strings — must be returned as-is
    expect(result.current.pipelineAgents).toEqual(["architect", "reviewer"]);
    // Key must NOT be cleared
    expect(localStorage.getItem(KEY("ws-td2-valid"))).not.toBeNull();
  });
});
