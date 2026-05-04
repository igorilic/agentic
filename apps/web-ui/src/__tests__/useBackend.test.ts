import { renderHook, act } from "@testing-library/react";
import { useBackend } from "../hooks/useBackend";

describe("useBackend", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("defaults to 'claude-code' when no value persisted", () => {
    const { result } = renderHook(() => useBackend());
    expect(result.current.backend).toBe("claude-code");
  });

  it("reads persisted value from localStorage on mount", () => {
    localStorage.setItem("agentic.backend", "copilot-cli");
    const { result } = renderHook(() => useBackend());
    expect(result.current.backend).toBe("copilot-cli");
  });

  it("ignores invalid persisted values", () => {
    localStorage.setItem("agentic.backend", "garbage");
    const { result } = renderHook(() => useBackend());
    expect(result.current.backend).toBe("claude-code");
  });

  it("setBackend updates state and persists", () => {
    const { result } = renderHook(() => useBackend());
    act(() => {
      result.current.setBackend("copilot-cli");
    });
    expect(result.current.backend).toBe("copilot-cli");
    expect(localStorage.getItem("agentic.backend")).toBe("copilot-cli");
  });
});
