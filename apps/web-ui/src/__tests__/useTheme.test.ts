import { renderHook, act } from "@testing-library/react";
import { useTheme } from "../hooks/useTheme";

describe("useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("defaults to light when localStorage is unset", () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe("light");
  });

  it("flips theme to dark after toggle()", () => {
    const { result } = renderHook(() => useTheme());
    act(() => {
      result.current.toggle();
    });
    expect(result.current.theme).toBe("dark");
  });

  it("writes localStorage after toggle()", () => {
    const { result } = renderHook(() => useTheme());
    act(() => {
      result.current.toggle();
    });
    expect(localStorage.getItem("agentic.theme")).toBe("dark");
  });

  it("sets data-theme attribute to dark after toggle()", () => {
    const { result } = renderHook(() => useTheme());
    act(() => {
      result.current.toggle();
    });
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("fresh hook instance reads dark from localStorage", () => {
    const { result: first } = renderHook(() => useTheme());
    act(() => {
      first.current.toggle();
    });
    const { result: second } = renderHook(() => useTheme());
    expect(second.current.theme).toBe("dark");
  });

  it("setTheme('light') updates theme to light and removes data-theme attribute", () => {
    const { result } = renderHook(() => useTheme());
    // Start by going dark first so there is something to revert
    act(() => {
      result.current.setTheme("dark");
    });
    expect(result.current.theme).toBe("dark");
    act(() => {
      result.current.setTheme("light");
    });
    expect(result.current.theme).toBe("light");
    expect(document.documentElement.getAttribute("data-theme")).toBeNull();
  });
});
