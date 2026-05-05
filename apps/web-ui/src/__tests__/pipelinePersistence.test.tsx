/**
 * I.7 — Per-project pipeline persistence integration tests.
 *
 * Tests the full App component with localStorage persistence:
 * - saving on change keyed by workspace id
 * - hydrating on mount
 * - graceful corrupt-JSON handling
 * - per-workspace isolation
 * - first-run empty state (no DEFAULT_AGENTS fallback)
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
    if (cmd === "get_workspace_id") return "ws-abc123";
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
      { name: "my-architect", description: "Plans things", source: "project" },
      { name: "my-developer", description: "Writes code", source: "project" },
    ],
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

describe("pipelinePersistence — App integration", () => {
  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_workspace_id") return "ws-abc123";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      if (cmd === "list_agents") return [];
      return undefined;
    });
  });

  // ── first-run empty state ─────────────────────────────────────────────────

  it("first-run with no localStorage entry shows an empty pipeline (no DEFAULT_AGENTS)", async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
    });

    // No agent cards should be rendered (empty pipeline)
    expect(screen.queryAllByTestId(/^agent-card-/)).toHaveLength(0);
    // Empty state prompt should be visible
    expect(screen.getByTestId("pipeline-empty-state")).toBeInTheDocument();
  });

  // ── hydration from localStorage ───────────────────────────────────────────

  it("hydrates pipelineAgents from localStorage on mount", async () => {
    // Pre-seed localStorage with a custom agent
    localStorage.setItem(
      "agentic.pipeline.ws-abc123",
      JSON.stringify(["my-architect"]),
    );

    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
    });

    // The agent card for my-architect should be rendered
    await waitFor(() => {
      expect(screen.getByTestId("agent-card-my-architect")).toBeInTheDocument();
    });
  });

  // ── corrupt JSON graceful handling ────────────────────────────────────────

  it("ignores corrupt JSON gracefully and starts with an empty pipeline", async () => {
    localStorage.setItem("agentic.pipeline.ws-abc123", "{not json}");
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
    });

    // Pipeline should be empty
    expect(screen.queryAllByTestId(/^agent-card-/)).toHaveLength(0);
    // Empty state prompt should be visible
    expect(screen.getByTestId("pipeline-empty-state")).toBeInTheDocument();
    // Console.warn should have been called
    expect(warnSpy).toHaveBeenCalled();

    warnSpy.mockRestore();
  });

  // ── save on change ────────────────────────────────────────────────────────

  it("saves pipelineAgents to localStorage on change, keyed by workspace id", async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
    });

    // Click "+ Add agent" and pick "my-architect"
    const user = userEvent.setup();
    await user.click(screen.getByTestId("pipeline-add-agent"));

    await waitFor(() => {
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("agent-picker-row-my-architect"));

    await waitFor(() => {
      expect(screen.getByTestId("agent-card-my-architect")).toBeInTheDocument();
    });

    // Check localStorage was written with the correct key and value
    const stored = localStorage.getItem("agentic.pipeline.ws-abc123");
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored!) as string[];
    expect(parsed).toContain("my-architect");
  });

  // ── per-workspace isolation ───────────────────────────────────────────────

  it("isolates per-workspace: different wsId shows a different list", async () => {
    // Pre-seed two workspace keys
    localStorage.setItem(
      "agentic.pipeline.ws-workspace-a",
      JSON.stringify(["my-architect"]),
    );
    localStorage.setItem(
      "agentic.pipeline.ws-workspace-b",
      JSON.stringify(["my-developer"]),
    );

    // First render with workspace A
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_workspace_id") return "ws-workspace-a";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      if (cmd === "list_agents") return [];
      return undefined;
    });

    const { unmount } = render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId("agent-card-my-architect")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("agent-card-my-developer")).toBeNull();
    unmount();

    // Second render with workspace B
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_workspace_id") return "ws-workspace-b";
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      if (cmd === "list_agents") return [];
      return undefined;
    });

    render(<App />);
    await waitFor(() => {
      expect(screen.getByTestId("agent-card-my-developer")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("agent-card-my-architect")).toBeNull();
  });
});
