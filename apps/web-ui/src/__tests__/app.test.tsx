import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "../App";
import type { EventEnvelope } from "../types/event";
import type { PermissionRequest } from "../types/pipeline";

// ---------------------------------------------------------------------------
// Module-level mocks — hoisted before imports
// ---------------------------------------------------------------------------

// Mock useTauriEvents so P.4.3 tests can inject a controlled event stream
// without needing a real Tauri listener. Default: empty stream.
vi.mock("../hooks/useTauriEvents", () => ({
  useTauriEvents: vi.fn().mockReturnValue({ events: [], historyError: null }),
}));

// Mock usePermissionRequests so P.4.3 tests can inject pending permissions.
// Default: no pending permissions.
vi.mock("../hooks/usePermissionRequests", () => ({
  usePermissionRequests: vi.fn().mockReturnValue([]),
}));

// Mock the Tauri APIs since they're not available in jsdom.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
// Default invoke mock returns [] for all IPC commands that fetch lists.
// Commands like list_runs, list_auth_accounts, and list_findings all return
// Vec<T> on the backend which serialises to [] (never null/undefined).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// ---------------------------------------------------------------------------
// Named imports of mocks (after vi.mock hoisting)
// ---------------------------------------------------------------------------
import { useTauriEvents } from "../hooks/useTauriEvents";
import { usePermissionRequests } from "../hooks/usePermissionRequests";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// HeaderBar uses useTheme which calls window.matchMedia — stub it for jsdom.
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
    run_id: opts.runId ?? "r1",
    step_id: opts.stepId ?? null,
    timestamp_ms: 1_700_000_000_000,
    event: { type: opts.type, data: opts.data },
  };
}

const examplePerm: PermissionRequest = {
  requestId: "p1",
  agent: "developer",
  tool: "shell",
  arg: "redis-cli FLUSHDB",
  scope: "shell.destructive",
  risk: "high",
  reason: "Reset Redis to validate cold-start.",
};

// ---------------------------------------------------------------------------
// Baseline App shell tests (unchanged from pre-P.4.3)
// ---------------------------------------------------------------------------

describe("App", () => {
  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    // Reset mocks to defaults for baseline tests
    vi.mocked(useTauriEvents).mockReturnValue({ events: [], historyError: null });
    vi.mocked(usePermissionRequests).mockReturnValue([]);
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it("renders the app shell header", () => {
    render(<App />);
    expect(screen.getByTestId("app-shell-header")).toBeInTheDocument();
  });

  it("renders the app shell pipeline", () => {
    render(<App />);
    expect(screen.getByTestId("app-shell-pipeline")).toBeInTheDocument();
  });

  it("renders the chat pane", () => {
    render(<App />);
    expect(screen.getByTestId("chat-pane")).toBeInTheDocument();
  });

  it("renders the event-list (inside ActivityColumn)", () => {
    render(<App />);
    expect(screen.getByTestId("event-list")).toBeInTheDocument();
  });

  it("renders the issue column", () => {
    render(<App />);
    expect(screen.getByTestId("issue-column")).toBeInTheDocument();
  });

  it("does NOT render the standalone cockpit stepper", () => {
    render(<App />);
    expect(screen.queryByTestId("cockpit-stepper")).toBeNull();
  });

  it("does NOT render a top-level findings-table", () => {
    render(<App />);
    expect(screen.queryByTestId("findings-table")).toBeNull();
  });

  it("renders the Agentic brand", () => {
    render(<App />);
    expect(screen.getByText("Agentic")).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// P.4.3: runId + stepId threading into permission_decide
// ---------------------------------------------------------------------------

describe("App — P.4.3 runId/stepId threading into permission_decide", () => {
  const mockUseTauriEvents = vi.mocked(useTauriEvents);
  const mockUsePermissionRequests = vi.mocked(usePermissionRequests);
  const mockInvoke = vi.mocked(invoke);

  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");

    mockUseTauriEvents.mockReset();
    mockUsePermissionRequests.mockReset();
    mockInvoke.mockReset();

    // Default: empty events, no permissions
    mockUseTauriEvents.mockReturnValue({ events: [], historyError: null });
    mockUsePermissionRequests.mockReturnValue([]);
    // Default: most IPC commands return []
    mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });
  });

  /**
   * Helper: start a run (sets activeRunId in App state) by invoking /plan.
   * Returns the run id that was returned by start_ticket_run.
   */
  async function startRun(runId: string) {
    const user = userEvent.setup();
    mockInvoke.mockImplementation(async (cmd: string): Promise<unknown> => {
      if (cmd === "start_ticket_run") return runId;
      if (cmd === "list_runs") return [];
      if (cmd === "list_findings") return [];
      if (cmd === "get_event_history") return [];
      return undefined;
    });
    render(<App />);
    await user.type(screen.getByTestId("chat-input"), `/plan Fix auth bug`);
    await user.click(screen.getByTestId("chat-send"));
    // Wait for invoke("start_ticket_run") to have been called, indicating
    // handleTicketRunStarted has fired and activeRunId has been set.
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "start_ticket_run",
        expect.anything(),
      );
    });
  }

  it("permission_decide_includes_runId_from_active_run", async () => {
    // Arrange: inject a PermissionRequest event in the event stream.
    const permEnv = makeEnvelope({
      id: "pe1",
      type: "PermissionRequest",
      data: { request_id: "p1" },
      runId: "r1",
      stepId: null,
    });
    mockUseTauriEvents.mockReturnValue({
      events: [permEnv],
      historyError: null,
    });
    mockUsePermissionRequests.mockReturnValue([examplePerm]);

    await startRun("r1");

    // Click "Allow once" on the rendered PermissionCard
    const allowOnce = await vi.waitFor(() =>
      screen.getByTestId("permission-card-allow-once"),
    );
    allowOnce.click();

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "once",
        runId: "r1",
        stepId: undefined,
      });
    });
  });

  it("permission_decide_includes_latest_stepId", async () => {
    // Arrange: event stream has StepStarted{step_id:"s1"} then PermissionRequest.
    const stepEnv = makeEnvelope({
      id: "ss1",
      type: "StepStarted",
      data: { agent: "developer", model: "claude-sonnet" },
      stepId: "s1",
      runId: "r1",
    });
    const permEnv = makeEnvelope({
      id: "pe1",
      type: "PermissionRequest",
      data: { request_id: "p1" },
      runId: "r1",
      stepId: null,
    });
    mockUseTauriEvents.mockReturnValue({
      events: [stepEnv, permEnv],
      historyError: null,
    });
    mockUsePermissionRequests.mockReturnValue([examplePerm]);

    await startRun("r1");

    const allowOnce = await vi.waitFor(() =>
      screen.getByTestId("permission-card-allow-once"),
    );
    allowOnce.click();

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "once",
        runId: "r1",
        stepId: "s1",
      });
    });
  });

  it("permission_decide_omits_stepId_when_no_step_started", async () => {
    // Arrange: event stream has only a PermissionRequest, no StepStarted.
    const permEnv = makeEnvelope({
      id: "pe1",
      type: "PermissionRequest",
      data: { request_id: "p1" },
      runId: "r1",
      stepId: null,
    });
    mockUseTauriEvents.mockReturnValue({
      events: [permEnv],
      historyError: null,
    });
    mockUsePermissionRequests.mockReturnValue([examplePerm]);

    await startRun("r1");

    const allowOnce = await vi.waitFor(() =>
      screen.getByTestId("permission-card-allow-once"),
    );
    allowOnce.click();

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "once",
        runId: "r1",
        stepId: undefined,
      });
    });
  });

  it("latest_stepId_uses_most_recent_step_started", async () => {
    // Arrange: two StepStarted events — s1 then s2. s2 should win.
    const step1Env = makeEnvelope({
      id: "ss1",
      type: "StepStarted",
      data: { agent: "architect", model: "claude-opus" },
      stepId: "s1",
      runId: "r1",
    });
    const step2Env = makeEnvelope({
      id: "ss2",
      type: "StepStarted",
      data: { agent: "developer", model: "claude-sonnet" },
      stepId: "s2",
      runId: "r1",
    });
    const permEnv = makeEnvelope({
      id: "pe1",
      type: "PermissionRequest",
      data: { request_id: "p1" },
      runId: "r1",
      stepId: null,
    });
    mockUseTauriEvents.mockReturnValue({
      events: [step1Env, step2Env, permEnv],
      historyError: null,
    });
    mockUsePermissionRequests.mockReturnValue([examplePerm]);

    await startRun("r1");

    const allowOnce = await vi.waitFor(() =>
      screen.getByTestId("permission-card-allow-once"),
    );
    allowOnce.click();

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "once",
        runId: "r1",
        stepId: "s2",
      });
    });
  });
});
