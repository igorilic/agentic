/**
 * I.7 fix-loop — "cannot add any agent" regression tests.
 *
 * The root cause hypothesis:
 *   1. `get_workspace_id` IPC throws (Tauri 2 capability missing) →
 *      `wsId` stays `null` in App.tsx
 *   2. `usePipelinePersistence(null)` returns a NO-OP setter
 *   3. AgentPicker → onSelect → `setPipelineAgents(next)` → noop → UI never updates
 *
 * These tests:
 *   A. Happy path: `get_workspace_id` succeeds → adding an agent persists to
 *      localStorage and the card appears in the pipeline bar.
 *   B. IPC-failure-loud-error: `get_workspace_id` throws → App.tsx logs a
 *      recognisable console.error (not swallowed silently).
 */
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";

// ---------------------------------------------------------------------------
// Module-level mocks
// ---------------------------------------------------------------------------
vi.mock("../hooks/useTauriEvents", () => ({
  useTauriEvents: vi.fn().mockReturnValue({ events: [], historyError: null }),
}));

vi.mock("../hooks/usePermissionRequests", () => ({
  usePermissionRequests: vi.fn().mockReturnValue([]),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation(async (cmd: string) => {
    if (cmd === "get_workspace_id") return "ws-regression-test";
    if (cmd === "list_runs") return [];
    if (cmd === "list_findings") return [];
    if (cmd === "get_event_history") return [];
    if (cmd === "list_agents") return [];
    return undefined;
  }),
}));

vi.mock("../hooks/useDiscoverableAgents", () => ({
  useDiscoverableAgents: vi.fn().mockReturnValue({
    agents: [
      { name: "architect", description: "Plans things", source: "project" },
      { name: "developer", description: "Writes code", source: "project" },
    ],
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

import { invoke } from "@tauri-apps/api/core";

function stubMatchMedia() {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
describe("addAgentRegression", () => {
  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_workspace_id") return "ws-regression-test";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      if (cmd === "list_agents") return [];
      return undefined;
    });
  });

  it("happy path: get_workspace_id succeeds, adding an agent updates pipeline and localStorage", async () => {
    const user = userEvent.setup();
    render(<App />);

    // Wait for the pipeline bar to render
    await waitFor(() => {
      expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
    });

    // Open agent picker and add an agent
    await user.click(screen.getByTestId("pipeline-add-agent"));

    await waitFor(() => {
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("agent-picker-row-architect"));

    // The agent card must appear in the pipeline bar
    await waitFor(() => {
      expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
    });

    // localStorage must have been written with the new entry
    const stored = localStorage.getItem("agentic.pipeline.ws-regression-test");
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored!) as string[];
    expect(parsed).toContain("architect");
  });

  it("IPC failure: get_workspace_id throws → App.tsx logs a recognisable console.error", async () => {
    // Make get_workspace_id throw
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_workspace_id") throw new Error("IPC denied: missing capability");
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      if (cmd === "list_agents") return [];
      return undefined;
    });

    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    render(<App />);

    // Give the useEffect time to fire and catch the error
    await waitFor(() => {
      // App.tsx must log a console.error with a recognisable prefix when
      // get_workspace_id fails — not swallow it silently.
      expect(errorSpy).toHaveBeenCalledWith(
        expect.stringContaining("[App] get_workspace_id"),
        expect.anything(),
      );
    });

    errorSpy.mockRestore();
  });
});
