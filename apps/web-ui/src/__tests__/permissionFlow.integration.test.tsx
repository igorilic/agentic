/**
 * P.6.1 — Permission flow end-to-end integration test (Vitest / jsdom).
 *
 * Exercises the full UI loop without a real Tauri backend:
 *   PermissionRequest envelope → card renders →
 *   click "Allow once" → invoke('permission_decide', …) →
 *   PermissionResolved envelope → card unmounts.
 *
 * Approach:
 * - Mock `useTauriEvents` with a ref-backed implementation so we can push new
 *   envelopes and force a re-render via `rerender()`.
 * - Do NOT mock `usePermissionRequests` — let it call through to the mocked
 *   `useTauriEvents`. This is the integration: events → hook → card lifecycle.
 * - Mock `invoke` and `@tauri-apps/api/event.listen` at the Tauri boundary.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, type Mock } from "vitest";
import App from "../App";
import type { EventEnvelope } from "../types/event";

// ---------------------------------------------------------------------------
// Module-level mocks — hoisted before imports
// ---------------------------------------------------------------------------

// Mock useTauriEvents with a mutable ref-backed implementation.
// usePermissionRequests is NOT mocked here — it must call through to this mock.
vi.mock("../hooks/useTauriEvents", () => ({
  useTauriEvents: vi.fn().mockReturnValue({ events: [], historyError: null }),
}));

// Tauri event listener — not used by the mock but imported by useTauriEvents source.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Tauri invoke — the IPC call under test.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// ---------------------------------------------------------------------------
// Named imports of mocks (resolved after vi.mock hoisting)
// ---------------------------------------------------------------------------
import { useTauriEvents } from "../hooks/useTauriEvents";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Stub window.matchMedia — required by HeaderBar → useTheme in jsdom. */
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
    run_id: opts.runId ?? "run-1",
    step_id: opts.stepId ?? null,
    timestamp_ms: 1_700_000_000_000,
    event: { type: opts.type, data: opts.data },
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("permission flow integration", () => {
  const mockUseTauriEvents = vi.mocked(useTauriEvents);
  const mockInvoke = vi.mocked(invoke) as Mock;

  /** Mutable events ref — push new envelopes here, then call rerender. */
  let eventsRef: { current: EventEnvelope[] };

  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");

    eventsRef = { current: [] };

    mockUseTauriEvents.mockReset();
    mockInvoke.mockReset();

    // useTauriEvents reads eventsRef.current on every call.
    // usePermissionRequests calls useTauriEvents() (no args) — it will pick up
    // the same ref, so pushing envelopes and re-rendering is enough to drive
    // the full integration chain.
    mockUseTauriEvents.mockImplementation(() => ({
      events: eventsRef.current,
      historyError: null,
    }));

    // Default: all IPC commands return [] (safe fallback).
    mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });
  });

  /**
   * Helper: start a run so App sets activeRunId.
   * Returns the rerender function so callers can push more events.
   */
  async function startRunAndMount(runId: string) {
    const user = userEvent.setup();

    mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "start_ticket_run") return runId;
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });

    const utils = render(<App />);

    // Start a run through the chat input so App wires up activeRunId.
    await user.type(screen.getByTestId("chat-input"), `/plan Fix auth bug`);
    await user.click(screen.getByTestId("chat-send"));

    // Wait until start_ticket_run has been invoked (activeRunId is now set).
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "start_ticket_run",
        expect.anything(),
      );
    });

    // Reset invoke mock so subsequent assertions are clean.
    mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "permission_decide") return undefined;
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });

    return { user, rerender: utils.rerender };
  }

  // ---------------------------------------------------------------------------
  // Test 1: full loop — request → allow once → resolved → unmount
  // ---------------------------------------------------------------------------

  it("user clicks Allow once: invoke fires + Resolved unmounts card", async () => {
    const { rerender } = await startRunAndMount("run-1");

    // Step 2: push a PermissionRequest envelope for request_id "r1".
    // The full permission data is needed so PermissionCard can render
    // (agent, tool, arg, scope, risk, reason).
    const permRequest = makeEnvelope({
      id: "perm-req-1",
      type: "PermissionRequest",
      data: {
        request_id: "r1",
        agent: "developer",
        tool: "shell",
        arg: "redis-cli FLUSHDB",
        scope: "shell.destructive",
        risk: "high",
        reason: "Reset Redis to validate cold-start.",
      },
      runId: "run-1",
      stepId: null,
    });

    eventsRef.current = [permRequest];
    rerender(<App />);

    // Step 3: wait for PermissionCard to render.
    const card = await waitFor(() => screen.getByTestId("permission-card"));
    expect(card).toBeInTheDocument();

    // Step 4: click "Allow once".
    await userEvent.click(screen.getByTestId("permission-card-allow-once"));

    // Step 5: assert invoke was called with the correct payload.
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "r1",
        decision: "once",
        runId: "run-1",
        stepId: undefined,
      });
    });

    // Step 6: push a PermissionResolved envelope for "r1".
    const permResolved = makeEnvelope({
      id: "perm-res-1",
      type: "PermissionResolved",
      data: {
        request_id: "r1",
        decision: "allow_once",
        source: "user",
      },
      runId: "run-1",
      stepId: null,
    });

    eventsRef.current = [permRequest, permResolved];
    rerender(<App />);

    // Step 7: wait for the card to disappear.
    await waitFor(() => {
      expect(screen.queryByTestId("permission-card")).toBeNull();
    });
  });

  // ---------------------------------------------------------------------------
  // Test 2: stepId is threaded when a StepStarted precedes the request
  // ---------------------------------------------------------------------------

  it("stepId from StepStarted is threaded into permission_decide payload", async () => {
    const { rerender } = await startRunAndMount("run-1");

    const stepStarted = makeEnvelope({
      id: "step-1",
      type: "StepStarted",
      data: { agent: "developer", model: "claude-sonnet" },
      stepId: "step-abc",
      runId: "run-1",
    });
    const permRequest = makeEnvelope({
      id: "perm-req-2",
      type: "PermissionRequest",
      data: {
        request_id: "r2",
        agent: "developer",
        tool: "shell",
        arg: "npm run build",
        scope: "shell.build",
        risk: "medium",
        reason: "Build the project.",
      },
      runId: "run-1",
      stepId: null,
    });

    eventsRef.current = [stepStarted, permRequest];
    rerender(<App />);

    await waitFor(() => screen.getByTestId("permission-card"));
    await userEvent.click(screen.getByTestId("permission-card-allow-once"));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "r2",
        decision: "once",
        runId: "run-1",
        stepId: "step-abc",
      });
    });
  });

  // ---------------------------------------------------------------------------
  // Test 3: PermissionResolved for an unknown request_id does not crash the app
  // ---------------------------------------------------------------------------

  it("PermissionResolved for unknown request does not crash the app", async () => {
    const { rerender } = await startRunAndMount("run-1");

    // Push a Resolved envelope for a request_id that was never requested.
    // This simulates a history replay race where Resolved arrives without a
    // prior Request in the current buffer window.
    const staleResolved = makeEnvelope({
      id: "perm-res-stale",
      type: "PermissionResolved",
      data: {
        request_id: "r-unknown",
        decision: "timed_out",
        source: "timeout",
      },
      runId: "run-1",
      stepId: null,
    });

    eventsRef.current = [staleResolved];
    rerender(<App />);

    // App shell is still present — no crash.
    await waitFor(() => {
      expect(screen.getByTestId("app-shell-header")).toBeInTheDocument();
    });

    // No permission card rendered (nothing to resolve).
    expect(screen.queryByTestId("permission-card")).toBeNull();
  });
});
