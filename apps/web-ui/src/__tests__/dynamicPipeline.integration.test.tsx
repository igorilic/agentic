/**
 * Step I.10 — End-to-end integration smoke: 2-agent dynamic pipeline.
 *
 * Exercises every Phase I contract together in a single test:
 *   I.1/I.6 — list discoverable agents
 *   I.7     — per-workspace persistence keyed by get_workspace_id
 *   I.8     — AgentPicker allows selecting agents
 *   I.4/I.5 — start_ticket_run receives the exact agents list
 *   Phase I — 2× StepStarted + StepComplete envelopes advance pipeline cards
 *
 * Approach:
 * - Mock @tauri-apps/api/core::invoke to capture all IPC calls (precise assertion
 *   for step 7: start_ticket_run was called with agents: ["architect", "reviewer"]).
 * - Mock useTauriEvents with a ref-backed implementation so we can push event
 *   envelopes and force a re-render (same idiom as permissionFlow.integration.test.tsx).
 * - Mock useDiscoverableAgents to return 3 discoverable agents.
 * - No new production code — this is the canary that I.1–I.9 land coherently.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, type Mock } from "vitest";
import App from "../App";
import type { EventEnvelope } from "../types/event";

// ---------------------------------------------------------------------------
// Module-level mocks — hoisted before imports
// ---------------------------------------------------------------------------

// useTauriEvents: ref-backed so we can inject envelopes without wiring Tauri.
vi.mock("../hooks/useTauriEvents", () => ({
  useTauriEvents: vi.fn().mockReturnValue({ events: [], historyError: null }),
}));

// usePermissionRequests: not under test — keep out of the way.
vi.mock("../hooks/usePermissionRequests", () => ({
  usePermissionRequests: vi.fn().mockReturnValue([]),
}));

// Tauri event listener — not used directly; useTauriEvents is mocked above.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// @tauri-apps/api/core::invoke — THE precise IPC boundary under test.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// useDiscoverableAgents — supplies 3 agents for the AgentPicker list.
vi.mock("../hooks/useDiscoverableAgents", () => ({
  useDiscoverableAgents: vi.fn().mockReturnValue({
    agents: [
      { name: "architect", description: "Designs system & breaks down work", source: "project" },
      { name: "reviewer",  description: "Code review & feedback",            source: "project" },
      { name: "qa",        description: "Runs tests, checks edge cases",     source: "project" },
    ],
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

// ---------------------------------------------------------------------------
// Named imports after hoisting
// ---------------------------------------------------------------------------
import { useTauriEvents } from "../hooks/useTauriEvents";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** window.matchMedia stub — required by HeaderBar → useTheme in jsdom. */
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

function makeEnvelope(opts: {
  id: string;
  type: string;
  data?: unknown;
  stepId?: string | null;
  runId?: string;
}): EventEnvelope {
  return {
    schema_version: 1,
    event_id: opts.id,
    run_id: opts.runId ?? "run-smoke-1",
    step_id: opts.stepId ?? null,
    timestamp_ms: 1_700_000_000_000,
    event: { type: opts.type, data: opts.data },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("I.10 — end-to-end dynamic pipeline integration smoke", () => {
  const mockUseTauriEvents = vi.mocked(useTauriEvents);
  const mockInvoke = vi.mocked(invoke) as Mock;

  /** Mutable event buffer — push envelopes here, then call rerender(). */
  let eventsRef: { current: EventEnvelope[] };

  const WS_ID = "ws-test123";
  const RUN_ID = "run-smoke-1";

  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");

    eventsRef = { current: [] };

    mockUseTauriEvents.mockReset();
    mockInvoke.mockReset();

    // useTauriEvents reads eventsRef.current on every call.
    mockUseTauriEvents.mockImplementation(() => ({
      events: eventsRef.current,
      historyError: null,
    }));

    // Default invoke handler — returns sensible defaults for every IPC command
    // that App.tsx calls on mount/run.
    mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "get_workspace_id") return WS_ID;
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      if (cmd === "list_agents") return [];
      if (cmd === "subscribe_events") return undefined;
      return undefined;
    });
  });

  it(
    "end-to-end: list → pick 2 → persist → run → 2 StepStarted events advance cards",
    async () => {
      const user = userEvent.setup();

      // ── 1. localStorage starts empty (cleared in beforeEach) ──────────────

      // ── 2. Render App ─────────────────────────────────────────────────────
      const { rerender } = render(<App />);

      // Wait for the pipeline bar and workspace id to be resolved.
      await waitFor(() => {
        expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
      });

      // Pipeline starts empty (no agents in localStorage).
      expect(screen.queryAllByTestId(/^agent-card-/)).toHaveLength(0);
      expect(screen.getByTestId("pipeline-empty-state")).toBeInTheDocument();

      // ── 3. Open AgentPicker via "+ Add agent" — pick "architect" ──────────
      await user.click(screen.getByTestId("pipeline-add-agent"));

      await waitFor(() => {
        expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      });

      await user.click(screen.getByTestId("agent-picker-row-architect"));

      await waitFor(() => {
        expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
      });

      // ── 4. Open AgentPicker again — pick "reviewer" ───────────────────────
      await user.click(screen.getByTestId("pipeline-add-agent"));

      await waitFor(() => {
        expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      });

      await user.click(screen.getByTestId("agent-picker-row-reviewer"));

      await waitFor(() => {
        expect(screen.getByTestId("agent-card-reviewer")).toBeInTheDocument();
      });

      // ── 5. Assert pipeline shows architect + reviewer in order ─────────────
      const cards = screen
        .getAllByTestId(/^agent-card-/)
        .filter((el) => el.hasAttribute("data-status"))
        .map((el) => el.dataset.testid as string);

      expect(cards).toEqual(["agent-card-architect", "agent-card-reviewer"]);

      // Assert localStorage persisted with the correct workspace key.
      const stored = localStorage.getItem(`agentic.pipeline.${WS_ID}`);
      expect(stored).not.toBeNull();
      expect(JSON.parse(stored!)).toEqual(["architect", "reviewer"]);

      // ── 6. Wire invoke mock to capture start_ticket_run and return RUN_ID ──
      mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
        if (cmd === "get_workspace_id") return WS_ID;
        if (cmd === "list_runs") return [];
        if (cmd === "list_findings") return [];
        if (cmd === "get_event_history") return [];
        if (cmd === "list_agents") return [];
        if (cmd === "subscribe_events") return undefined;
        if (cmd === "start_ticket_run") return RUN_ID;
        return undefined;
      });

      // ── 7. Click "Run pipeline" ─────────────────────────────────────────────
      const runBtn = await waitFor(() => screen.getByTestId("header-run"));
      await user.click(runBtn);

      // ── 8. Assert start_ticket_run was called with exactly the two agents ───
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("start_ticket_run", {
          ticket: "Untitled run",
          backend: expect.any(String),
          model: null,
          agents: ["architect", "reviewer"],
        });
      });

      // ── 9. Drive the event stream: RunStarted → 2× StepStarted+StepComplete ─

      // RunStarted — initialises the pipeline with the user-selected agents list.
      eventsRef.current = [
        makeEnvelope({
          id: "ev-0",
          type: "RunStarted",
          data: { agents: ["architect", "reviewer"] },
          runId: RUN_ID,
        }),
      ];
      rerender(<App />);

      // Both cards should be in the pipeline at "queued" (pending → AgentStatus
      // "queued") after RunStarted is processed.
      await waitFor(() => {
        expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
        expect(screen.getByTestId("agent-card-reviewer")).toBeInTheDocument();
      });

      // StepStarted for architect (step_id = "s1")
      eventsRef.current = [
        ...eventsRef.current,
        makeEnvelope({
          id: "ev-1",
          type: "StepStarted",
          data: { agent: "architect", model: "claude-sonnet" },
          stepId: "s1",
          runId: RUN_ID,
        }),
      ];
      rerender(<App />);

      await waitFor(() => {
        const architectCard = screen.getByTestId("agent-card-architect");
        expect(architectCard).toHaveAttribute("data-status", "active");
      });

      // StepComplete for architect (status = "passed")
      eventsRef.current = [
        ...eventsRef.current,
        makeEnvelope({
          id: "ev-2",
          type: "StepComplete",
          data: { status: "passed", duration_ms: 1200 },
          stepId: "s1",
          runId: RUN_ID,
        }),
      ];
      rerender(<App />);

      await waitFor(() => {
        const architectCard = screen.getByTestId("agent-card-architect");
        expect(architectCard).toHaveAttribute("data-status", "done");
      });

      // StepStarted for reviewer (step_id = "s2")
      eventsRef.current = [
        ...eventsRef.current,
        makeEnvelope({
          id: "ev-3",
          type: "StepStarted",
          data: { agent: "reviewer", model: "claude-sonnet" },
          stepId: "s2",
          runId: RUN_ID,
        }),
      ];
      rerender(<App />);

      await waitFor(() => {
        const reviewerCard = screen.getByTestId("agent-card-reviewer");
        expect(reviewerCard).toHaveAttribute("data-status", "active");
      });

      // StepComplete for reviewer (status = "passed")
      eventsRef.current = [
        ...eventsRef.current,
        makeEnvelope({
          id: "ev-4",
          type: "StepComplete",
          data: { status: "passed", duration_ms: 980 },
          stepId: "s2",
          runId: RUN_ID,
        }),
      ];
      rerender(<App />);

      await waitFor(() => {
        const reviewerCard = screen.getByTestId("agent-card-reviewer");
        expect(reviewerCard).toHaveAttribute("data-status", "done");
      });

      // Architect card must remain "done" throughout reviewer's step.
      expect(screen.getByTestId("agent-card-architect")).toHaveAttribute(
        "data-status",
        "done",
      );

      // RunComplete — wrap up the run.
      eventsRef.current = [
        ...eventsRef.current,
        makeEnvelope({
          id: "ev-5",
          type: "RunComplete",
          data: { status: "completed" },
          runId: RUN_ID,
        }),
      ];
      rerender(<App />);

      // Both pipeline cards end in "done" state.
      await waitFor(() => {
        expect(screen.getByTestId("agent-card-architect")).toHaveAttribute(
          "data-status",
          "done",
        );
        expect(screen.getByTestId("agent-card-reviewer")).toHaveAttribute(
          "data-status",
          "done",
        );
      });
    },
  );
});
