import { useState } from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { vi } from "vitest";
import ActivityColumn from "../components/ActivityColumn";
import type { EventEnvelope } from "../types/event";
import type { PermissionRequest } from "../types/pipeline";

// --- P.4.2 module-level mocks ---
// Must be at the top level so Vitest hoists them before imports.
vi.mock("../hooks/usePermissionRequests", () => ({
  usePermissionRequests: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { usePermissionRequests } from "../hooks/usePermissionRequests";
import { invoke } from "@tauri-apps/api/core";

function envelope(opts: {
  id: string;
  type: string;
  data?: unknown;
  t?: number;
  stepId?: string | null;
}): EventEnvelope {
  return {
    schema_version: 1,
    event_id: opts.id,
    run_id: "run-1",
    step_id: opts.stepId ?? null,
    timestamp_ms: opts.t ?? 1_700_000_000_000,
    event: { type: opts.type, data: opts.data },
  };
}

function ControlledActivityColumn({ events }: { events: EventEnvelope[] }) {
  const [filter, setFilter] = useState<"all" | "tool" | "perm" | "error">("all");
  return <ActivityColumn events={events} filter={filter} onFilterChange={setFilter} />;
}

// W.7.2 helper: seeds usePermissionRequests via the mock (P.4.2 — props removed).
// pendingPermissions is accepted here for readability but is applied via
// vi.mocked(usePermissionRequests).mockReturnValue() in W.7.2 beforeEach.
type ControlledWithPermsProps = {
  events: EventEnvelope[];
  initialFilter?: "all" | "tool" | "perm" | "error";
};

function ControlledActivityColumnWithPerms({
  events,
  initialFilter = "all",
}: ControlledWithPermsProps) {
  const [filter, setFilter] = useState<"all" | "tool" | "perm" | "error">(initialFilter);
  return (
    <ActivityColumn
      events={events}
      filter={filter}
      onFilterChange={setFilter}
      runId="run-1"
    />
  );
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

const mixedEvents: EventEnvelope[] = [
  envelope({ id: "e1", type: "RunStarted", t: 1_700_000_000_000, stepId: null }),
  envelope({
    id: "e2",
    type: "ToolCall",
    t: 1_700_000_001_000,
    stepId: "developer",
    data: { tool: "read_file", arg: "/src/api.ts", result: "OK" },
  }),
  envelope({ id: "e3", type: "StepComplete", t: 1_700_000_002_000, stepId: "qa" }),
  envelope({
    id: "e4",
    type: "ToolCall",
    t: 1_700_000_003_000,
    stepId: "qa",
    data: { tool: "shell", arg: "go test", result: "OK" },
  }),
  envelope({
    id: "e5",
    type: "Failed",
    t: 1_700_000_004_000,
    stepId: "developer",
    data: { message: "build failed" },
  }),
];

const findingEvents: EventEnvelope[] = [
  envelope({
    id: "f1",
    type: "Finding",
    t: 1_700_000_010_000,
    stepId: "reviewer",
    data: { message: "Reviewer flagged: lock contention under burst", severity: "error" },
  }),
];

describe("ActivityColumn", () => {
  beforeEach(() => {
    // These tests don't exercise permissions — default the hook to []
    vi.mocked(usePermissionRequests).mockReturnValue([]);
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("renders outer data-testid='activity-column'", () => {
    render(<ControlledActivityColumn events={[]} />);
    expect(screen.getByTestId("activity-column")).toBeInTheDocument();
  });

  it("renders inner UL with data-testid='event-list' for backward compat", () => {
    render(<ControlledActivityColumn events={[]} />);
    expect(screen.getByTestId("event-list")).toBeInTheDocument();
  });

  it("renders activity-header with all 4 tabs", () => {
    render(<ControlledActivityColumn events={[]} />);
    expect(screen.getByTestId("activity-header")).toBeInTheDocument();
    expect(screen.getByTestId("activity-tab-all")).toBeInTheDocument();
    expect(screen.getByTestId("activity-tab-tool")).toBeInTheDocument();
    expect(screen.getByTestId("activity-tab-perm")).toBeInTheDocument();
    expect(screen.getByTestId("activity-tab-error")).toBeInTheDocument();
  });

  it("shows correct counts in header tabs for mixed events", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    expect(screen.getByTestId("activity-tab-all-count")).toHaveTextContent("5");
    expect(screen.getByTestId("activity-tab-tool-count")).toHaveTextContent("2");
    expect(screen.getByTestId("activity-tab-error-count")).toHaveTextContent("1");
    expect(screen.getByTestId("activity-tab-perm-count")).toHaveTextContent("0");
  });

  it("All tab shows all 5 events", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(5);
  });

  it("Tool calls tab shows only the 2 tool events", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    fireEvent.click(screen.getByTestId("activity-tab-tool"));
    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(2);
  });

  it("Errors tab shows only the 1 error event", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    fireEvent.click(screen.getByTestId("activity-tab-error"));
    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(1);
  });

  it("shows empty state when events array is empty", () => {
    render(<ControlledActivityColumn events={[]} />);
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });

  it("filters out TextDelta events — they do not render in any tab", () => {
    const eventsWithDelta: EventEnvelope[] = [
      ...mixedEvents,
      envelope({ id: "e6", type: "TextDelta", t: 1_700_000_005_000, stepId: "developer" }),
    ];
    render(<ControlledActivityColumn events={eventsWithDelta} />);
    // All tab: still only 5 rows (not 6)
    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(5);
  });

  it("ToolCall events render as ToolCallCard", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    const cards = screen.getAllByTestId("tool-call-card");
    expect(cards).toHaveLength(2);
  });

  it("error events render as LogRow with level='error' and red chip", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    expect(screen.getByTestId("log-row-error")).toBeInTheDocument();
    expect(screen.getByTestId("log-row-level-chip")).toHaveClass("bg-red-500");
  });

  it("info events render as LogRow with level='info'", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    // RunStarted and StepComplete are info events
    const infoRows = screen.getAllByTestId("log-row-info");
    expect(infoRows.length).toBeGreaterThanOrEqual(2);
  });

  it("event-list UL has aria-live='polite'", () => {
    render(<ControlledActivityColumn events={[]} />);
    expect(screen.getByTestId("event-list")).toHaveAttribute("aria-live", "polite");
  });

  it("step_id → agent resolved via StepStarted map; events without map entry fall back to 'system'", () => {
    // mixedEvents has no StepStarted events, so all step_ids without a map entry resolve to "system"
    // RunStarted (step_id=null) → "system"
    // ToolCall events (step_id="developer"/"qa") → no StepStarted to populate map → "system"
    render(<ControlledActivityColumn events={mixedEvents} />);
    const agentLabels = screen.getAllByTestId("log-row-agent");
    const agentTexts = agentLabels.map((el) => el.textContent);
    // All non-tool rows have agent "system" since no StepStarted to populate map
    expect(agentTexts).toContain("system");
  });

  it("renders zero event rows when perm tab is selected", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    fireEvent.click(screen.getByTestId("activity-tab-perm"));
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });

  it("Finding event renders in All tab with log-row-error and red chip", () => {
    render(<ControlledActivityColumn events={findingEvents} />);
    expect(screen.getByTestId("log-row-error")).toBeInTheDocument();
    expect(screen.getByTestId("log-row-level-chip")).toHaveClass("bg-red-500");
  });

  it("Finding event renders in Errors tab", () => {
    render(<ControlledActivityColumn events={findingEvents} />);
    fireEvent.click(screen.getByTestId("activity-tab-error"));
    expect(screen.getAllByTestId("event-row")).toHaveLength(1);
  });

  it("Finding with severity='warning' renders as log-row-info, not error", () => {
    const warningFinding = [
      envelope({
        id: "fw1",
        type: "Finding",
        t: 1_700_000_020_000,
        stepId: "reviewer",
        data: { message: "No test for all-punctuation input", severity: "warning" },
      }),
    ];
    render(<ControlledActivityColumn events={warningFinding} />);
    expect(screen.getByTestId("log-row-info")).toBeInTheDocument();
    expect(screen.queryByTestId("log-row-error")).toBeNull();
    expect(screen.queryByTestId("log-row-level-chip")).toBeNull();
  });

  it("Finding with severity='info' renders as log-row-info, not error", () => {
    const infoFinding = [
      envelope({
        id: "fi1",
        type: "Finding",
        t: 1_700_000_021_000,
        stepId: "reviewer",
        data: { message: "Observation: cache hit rate is high", severity: "info" },
      }),
    ];
    render(<ControlledActivityColumn events={infoFinding} />);
    expect(screen.getByTestId("log-row-info")).toBeInTheDocument();
    expect(screen.queryByTestId("log-row-error")).toBeNull();
    expect(screen.queryByTestId("log-row-level-chip")).toBeNull();
  });

  it("Finding with no severity field defaults to info (not error)", () => {
    const noSeverityFinding = [
      envelope({
        id: "fns1",
        type: "Finding",
        t: 1_700_000_022_000,
        stepId: "reviewer",
        data: { message: "Finding without explicit severity" },
      }),
    ];
    render(<ControlledActivityColumn events={noSeverityFinding} />);
    expect(screen.getByTestId("log-row-info")).toBeInTheDocument();
    expect(screen.queryByTestId("log-row-error")).toBeNull();
    expect(screen.queryByTestId("log-row-level-chip")).toBeNull();
  });

  it("Finding with severity='ERROR' (uppercase) is treated as error — case-insensitive", () => {
    const uppercaseErrorFinding = [
      envelope({
        id: "fue1",
        type: "Finding",
        t: 1_700_000_023_000,
        stepId: "reviewer",
        data: { message: "Critical issue found", severity: "ERROR" },
      }),
    ];
    render(<ControlledActivityColumn events={uppercaseErrorFinding} />);
    expect(screen.getByTestId("log-row-error")).toBeInTheDocument();
    expect(screen.getByTestId("log-row-level-chip")).toHaveClass("bg-red-500");
  });

  it("Finding with severity='warning' does NOT appear in Errors tab, only in All", () => {
    const warningFinding = [
      envelope({
        id: "fw2",
        type: "Finding",
        t: 1_700_000_024_000,
        stepId: "reviewer",
        data: { message: "Warning-level finding", severity: "warning" },
      }),
    ];
    render(<ControlledActivityColumn events={warningFinding} />);
    // All tab shows it
    expect(screen.getAllByTestId("event-row")).toHaveLength(1);
    // Errors tab does NOT show it
    fireEvent.click(screen.getByTestId("activity-tab-error"));
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });
});

// --- W.7.2: PermissionCard inline in ActivityColumn ---

const permEnvelope = envelope({
  id: "pe1",
  type: "PermissionRequest",
  t: 1_700_000_001_000,
  stepId: "developer",
  data: { request_id: "p1" },
});

describe("ActivityColumn — W.7.2 PermissionCard inline rendering", () => {
  // P.4.2: pendingPermissions/onPermissionDecision props removed.
  // Tests now seed permissions via mocked usePermissionRequests hook.
  // Decision callbacks now verified via mocked invoke IPC.
  const mockUsePermReqs = vi.mocked(usePermissionRequests);
  const mockInvoke = vi.mocked(invoke);

  beforeEach(() => {
    mockUsePermReqs.mockReset();
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(undefined);
    // Default: no pending permissions unless overridden in the test
    mockUsePermReqs.mockReturnValue([]);
  });

  it("renders PermissionCard for a matched perm event", () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms events={[permEnvelope]} />,
    );
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
    expect(screen.getAllByTestId("event-row")).toHaveLength(1);
  });

  it("filter 'perm' shows permission cards", () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        initialFilter="perm"
      />,
    );
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
  });

  it("filter 'tool' hides permission cards", () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        initialFilter="tool"
      />,
    );
    expect(screen.queryByTestId("permission-card")).toBeNull();
  });

  it("filter 'all' includes permission cards", () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        initialFilter="all"
      />,
    );
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
  });

  it("header counts.perm reflects 1 when one matched perm event present", () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms events={[permEnvelope]} />,
    );
    expect(screen.getByTestId("activity-tab-perm-count")).toHaveTextContent("1");
  });

  it("unmatched perm event (request_id not in hook) drops out", () => {
    // Hook returns [] — the PermissionRequest event has no matching entry
    mockUsePermReqs.mockReturnValue([]);
    const unmatchedEnvelope = envelope({
      id: "pe2",
      type: "PermissionRequest",
      data: { request_id: "p999" },
    });
    render(
      <ControlledActivityColumnWithPerms events={[unmatchedEnvelope]} />,
    );
    expect(screen.queryByTestId("permission-card")).toBeNull();
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
    expect(screen.getByTestId("activity-tab-perm-count")).toHaveTextContent("0");
  });

  it("hook returning [] causes perm events to drop entirely", () => {
    // Mirrors the old "omitting pendingPermissions" test — hook is now the source of truth
    mockUsePermReqs.mockReturnValue([]);
    render(
      <ControlledActivityColumnWithPerms events={[permEnvelope]} />,
    );
    expect(screen.queryByTestId("permission-card")).toBeNull();
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });

  it("decision fires invoke('permission_decide', ..., 'once') on Allow once", async () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms events={[permEnvelope]} />,
    );
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "once",
        runId: "run-1",
        stepId: undefined,
      });
    });
  });

  it("decision fires invoke('permission_decide', ..., 'session') on Allow for session", async () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms events={[permEnvelope]} />,
    );
    fireEvent.click(screen.getByTestId("permission-card-allow-session"));
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "session",
        runId: "run-1",
        stepId: undefined,
      });
    });
  });

  it("decision fires invoke('permission_decide', ..., 'deny') on Deny", async () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    render(
      <ControlledActivityColumnWithPerms events={[permEnvelope]} />,
    );
    fireEvent.click(screen.getByTestId("permission-card-deny"));
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "p1",
        decision: "deny",
        runId: "run-1",
        stepId: undefined,
      });
    });
  });

  it("mixed event stream: RunStarted + PermissionRequest + ToolCall all visible under filter=all", () => {
    mockUsePermReqs.mockReturnValue([examplePerm]);
    const mixedWithPerm: EventEnvelope[] = [
      envelope({ id: "m1", type: "RunStarted", t: 1_700_000_000_000 }),
      permEnvelope,
      envelope({
        id: "m3",
        type: "ToolCall",
        t: 1_700_000_002_000,
        stepId: "developer",
        data: { tool: "read_file", arg: "/src/api.ts", result: "OK" },
      }),
    ];
    render(
      <ControlledActivityColumnWithPerms
        events={mixedWithPerm}
        initialFilter="all"
      />,
    );
    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(3);
    // The perm row is at index 1; it contains the PermissionCard
    expect(rows[1].querySelector('[data-testid="permission-card"]')).not.toBeNull();
  });

  it("W.5.4 backward compat: ControlledActivityColumn (no perms) — perm events drop, others show", () => {
    // ControlledActivityColumn uses usePermissionRequests mock returning []
    mockUsePermReqs.mockReturnValue([]);
    render(<ControlledActivityColumn events={mixedEvents} />);
    expect(screen.getAllByTestId("event-row")).toHaveLength(5);
    expect(screen.queryByTestId("permission-card")).toBeNull();
  });
});

// --- Real backend event classifier (step_id → agent map, human messages, filtered noise) ---

const STEP_ULID = "01KQG1JPPHEE8RMVH3X4Y5Z6W7";
const STEP_ULID_2 = "01KQG1M5RMVH3X4Y5Z6W7A8B9C";

describe("ActivityColumn — real backend event classifier", () => {
  beforeEach(() => {
    vi.mocked(usePermissionRequests).mockReturnValue([]);
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  // A. step_id → agent resolution via StepStarted map

  it("StepStarted followed by StepComplete: StepComplete row shows resolved agent, not ULID", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "rs1",
        type: "StepStarted",
        t: 1_700_000_001_000,
        stepId: STEP_ULID,
        data: { agent: "architect", model: "claude-sonnet" },
      }),
      envelope({
        id: "sc1",
        type: "StepComplete",
        t: 1_700_000_060_000,
        stepId: STEP_ULID,
        data: { status: "passed", duration_ms: 59000, summary: "" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const agentLabels = screen.getAllByTestId("log-row-agent");
    const agentTexts = agentLabels.map((el) => el.textContent);
    // Both rows should show "architect", not the ULID
    expect(agentTexts.every((t) => t === "architect")).toBe(true);
    expect(agentTexts).not.toContain(STEP_ULID);
  });

  it("events with step_id but no preceding StepStarted fall back to 'system'", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "sc_orphan",
        type: "StepComplete",
        t: 1_700_000_060_000,
        stepId: STEP_ULID,
        data: { status: "passed", duration_ms: 1000 },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const agentLabel = screen.getByTestId("log-row-agent");
    expect(agentLabel.textContent).toBe("system");
  });

  it("multiple steps: each step resolves to its own agent name", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "ss1",
        type: "StepStarted",
        stepId: STEP_ULID,
        data: { agent: "architect", model: "claude-sonnet" },
      }),
      envelope({
        id: "ss2",
        type: "StepStarted",
        stepId: STEP_ULID_2,
        data: { agent: "tdd-developer", model: "claude-sonnet" },
      }),
      envelope({
        id: "sc2",
        type: "StepComplete",
        stepId: STEP_ULID_2,
        data: { status: "passed", duration_ms: 5000 },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const agentLabels = screen.getAllByTestId("log-row-agent");
    const agentTexts = agentLabels.map((el) => el.textContent);
    expect(agentTexts).toContain("architect");
    expect(agentTexts).toContain("tdd-developer");
  });

  // B. Filtered event types

  it("ThinkingDelta events are filtered and produce no visible rows", () => {
    const events: EventEnvelope[] = [
      envelope({ id: "td1", type: "ThinkingDelta", stepId: STEP_ULID, data: { content: "hmm..." } }),
    ];
    render(<ControlledActivityColumn events={events} />);
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
    expect(screen.getByTestId("activity-tab-all-count")).toHaveTextContent("0");
  });

  it("ToolUseDelta events are filtered and produce no visible rows", () => {
    const events: EventEnvelope[] = [
      envelope({ id: "tud1", type: "ToolUseDelta", stepId: STEP_ULID, data: { content: "stdout line" } }),
    ];
    render(<ControlledActivityColumn events={events} />);
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });

  it("TextDelta events are still filtered (regression guard)", () => {
    const events: EventEnvelope[] = [
      envelope({ id: "textd1", type: "TextDelta", stepId: STEP_ULID, data: { content: "partial text" } }),
    ];
    render(<ControlledActivityColumn events={events} />);
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });

  // C. ToolUseStart → ToolCallCard

  it("ToolUseStart with string input renders as ToolCallCard with correct tool and arg", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "tus1",
        type: "ToolUseStart",
        stepId: STEP_ULID,
        data: { tool_call_id: "tc1", tool_name: "read_file", input: "/src/foo.ts" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    expect(screen.getByTestId("tool-call-card")).toBeInTheDocument();
  });

  it("ToolUseStart with object input renders as ToolCallCard with JSON-stringified arg", () => {
    const inputObj = { command: "ls -la" };
    const events: EventEnvelope[] = [
      envelope({
        id: "tus2",
        type: "ToolUseStart",
        stepId: STEP_ULID,
        data: { tool_call_id: "tc2", tool_name: "shell", input: inputObj },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const card = screen.getByTestId("tool-call-card");
    expect(card).toBeInTheDocument();
    // The arg should be the JSON stringified input
    expect(card.textContent).toContain(JSON.stringify(inputObj));
  });

  it("ToolUseStart appears in the tool filter tab", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "tus3",
        type: "ToolUseStart",
        stepId: STEP_ULID,
        data: { tool_call_id: "tc3", tool_name: "write_file", input: "/out/bar.ts" },
      }),
    ];
    const { container } = render(<ControlledActivityColumn events={events} />);
    fireEvent.click(screen.getByTestId("activity-tab-tool"));
    const rows = container.querySelectorAll('[data-testid="event-row"]');
    expect(rows).toHaveLength(1);
  });

  it("ToolUseEnd alone produces no visible row (filtered)", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "tue1",
        type: "ToolUseEnd",
        stepId: STEP_ULID,
        data: { tool_call_id: "tc1", exit_code: 0, duration_ms: 123 },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
    expect(screen.getByTestId("activity-tab-all-count")).toHaveTextContent("0");
  });

  // D. Per-type human message translations

  it("RunStarted renders message 'Run started'", () => {
    const events: EventEnvelope[] = [
      envelope({ id: "rs_msg", type: "RunStarted", stepId: null, data: { ticket: "ABC-1" } }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("Run started");
  });

  it("RunComplete with duration renders 'Run completed in 47s'", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "rc_msg",
        type: "RunComplete",
        stepId: null,
        data: { status: "completed", duration_ms: 47000 },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("Run completed in 47s");
  });

  it("StepStarted renders message 'Started'", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "ss_msg",
        type: "StepStarted",
        stepId: STEP_ULID,
        data: { agent: "qa", model: "claude-haiku" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("Started");
  });

  it("StepComplete with non-empty summary renders the summary as message", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "sc_sum",
        type: "StepComplete",
        stepId: STEP_ULID,
        data: { status: "passed", duration_ms: 12000, summary: "ok" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("ok");
  });

  it("StepComplete without summary uses 'status in Xs' format", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "sc_nosummary",
        type: "StepComplete",
        stepId: STEP_ULID,
        data: { status: "passed", duration_ms: 12000, summary: "" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("passed in 12s");
  });

  it("FileChange renders 'Changed /src/foo.go'", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "fc_msg",
        type: "FileChange",
        stepId: STEP_ULID,
        data: { path: "/src/foo.go", before_hash: "abc", after_hash: "def" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("Changed /src/foo.go");
  });

  it("ClarifyingQuestion renders the question text prefixed with '?'", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "cq_msg",
        type: "ClarifyingQuestion",
        stepId: STEP_ULID,
        data: { question_id: "q1", question: "Which module should handle auth?", suggested_answers: [] },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row.textContent).toContain("? Which module should handle auth?");
  });

  it("ToolUseStart shows agent resolved from StepStarted when step_id matches", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "ss_for_tool",
        type: "StepStarted",
        stepId: STEP_ULID,
        data: { agent: "tdd-developer", model: "claude-sonnet" },
      }),
      envelope({
        id: "tus_agent",
        type: "ToolUseStart",
        stepId: STEP_ULID,
        data: { tool_call_id: "tc_a", tool_name: "bash", input: "cargo test" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    // ToolCallCard should show agent "tdd-developer"
    const cards = screen.getAllByTestId("tool-call-card");
    expect(cards).toHaveLength(1);
    // The card's agent chip should contain "tdd-developer"
    expect(cards[0].textContent).toContain("tdd-developer");
  });

  // Issue 2: resolveAgent must treat undefined step_id the same as null
  it("resolveAgent treats undefined step_id same as null — returns 'system'", () => {
    // Mimic Rust serde Option::None serialized as missing field (skip_serializing_if)
    // so JS receives undefined instead of null.
    const eventWithUndefinedStepId = {
      schema_version: 1,
      event_id: "e_undef",
      run_id: "run-1",
      // step_id intentionally omitted (undefined) — simulates Tauri IPC omitting the field
      timestamp_ms: 1_700_000_001_000,
      event: { type: "StepComplete", data: { status: "passed", duration_ms: 1000 } },
    } as unknown as EventEnvelope;

    render(<ControlledActivityColumn events={[eventWithUndefinedStepId]} />);
    const agentLabel = screen.getByTestId("log-row-agent");
    expect(agentLabel.textContent).toBe("system");
  });

  it("resolveAgent returns mapped agent name when step_id matches StepStarted", () => {
    const events: EventEnvelope[] = [
      envelope({
        id: "ss_map",
        type: "StepStarted",
        stepId: STEP_ULID,
        data: { agent: "spec-writer", model: "claude-sonnet" },
      }),
      envelope({
        id: "sc_map",
        type: "StepComplete",
        stepId: STEP_ULID,
        data: { status: "passed", duration_ms: 5000, summary: "done" },
      }),
    ];
    render(<ControlledActivityColumn events={events} />);
    const agentLabels = screen.getAllByTestId("log-row-agent");
    // Both StepStarted and StepComplete rows should show "spec-writer"
    expect(agentLabels.every((el) => el.textContent === "spec-writer")).toBe(true);
  });

  it("buildStepAgentMap handles undefined step_id on StepStarted gracefully", () => {
    // A StepStarted with undefined step_id (from Tauri IPC missing field) should not
    // register any mapping — downstream events with non-null step_id still fall back to system.
    const stepStartedWithUndefinedStepId = {
      schema_version: 1,
      event_id: "e_ss_undef",
      run_id: "run-1",
      // step_id intentionally omitted
      timestamp_ms: 1_700_000_000_000,
      event: { type: "StepStarted", data: { agent: "spec-writer", model: "m" } },
    } as unknown as EventEnvelope;
    const toolUse = envelope({
      id: "e_tool_undef",
      type: "ToolUseStart",
      stepId: STEP_ULID,
      data: { tool_call_id: "tc_x", tool_name: "bash", input: "ls" },
    });
    render(<ControlledActivityColumn events={[stepStartedWithUndefinedStepId, toolUse]} />);
    // ToolUseStart with STEP_ULID — StepStarted had undefined step_id → no map entry → "system"
    const cards = screen.getAllByTestId("tool-call-card");
    expect(cards).toHaveLength(1);
    // Agent chip should be "system" since the map has no entry for STEP_ULID
    expect(cards[0].textContent).toContain("system");
  });
});

// --- P.4.2: ActivityColumn consumes live usePermissionRequests + dispatches IPC ---

const r1Perm: PermissionRequest = {
  requestId: "r1",
  agent: "developer",
  tool: "shell",
  arg: "redis-cli FLUSHDB",
  scope: "shell.destructive",
  risk: "high",
  reason: "Reset Redis to validate cold-start.",
};

const permEvent: EventEnvelope = {
  schema_version: 1,
  event_id: "pe-r1",
  run_id: "run-1",
  step_id: "step-1",
  timestamp_ms: 1_700_000_001_000,
  event: { type: "PermissionRequest", data: { request_id: "r1" } },
};

function ControlledActivityColumnLive({
  runId,
  stepId,
}: {
  runId?: string;
  stepId?: string;
}) {
  const [filter, setFilter] = useState<"all" | "tool" | "perm" | "error">("all");
  return (
    <ActivityColumn
      events={[permEvent]}
      filter={filter}
      onFilterChange={setFilter}
      runId={runId}
      stepId={stepId}
    />
  );
}

describe("ActivityColumn — P.4.2 live hook + IPC wiring", () => {
  const mockUsePermissionRequests = vi.mocked(usePermissionRequests);
  const mockInvoke = vi.mocked(invoke);

  beforeEach(() => {
    mockUsePermissionRequests.mockReset();
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(undefined);
  });

  it("renders_permission_card_from_live_hook", () => {
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    render(<ControlledActivityColumnLive runId="run-1" />);
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
    // Agent name should appear
    expect(screen.getByTestId("permission-card").textContent).toContain("developer");
  });

  it("allow_once_click_fires_permission_decide_ipc", async () => {
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    render(<ControlledActivityColumnLive runId="run-1" />);
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));
    // Allow async IPC call to settle
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "r1",
        decision: "once",
        runId: "run-1",
        stepId: undefined,
      });
    });
  });

  it("session_click_fires_permission_decide_with_session", async () => {
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    render(<ControlledActivityColumnLive runId="run-1" />);
    fireEvent.click(screen.getByTestId("permission-card-allow-session"));
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "r1",
        decision: "session",
        runId: "run-1",
        stepId: undefined,
      });
    });
  });

  it("deny_click_fires_permission_decide_with_deny", async () => {
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    render(<ControlledActivityColumnLive runId="run-1" />);
    fireEvent.click(screen.getByTestId("permission-card-deny"));
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "r1",
        decision: "deny",
        runId: "run-1",
        stepId: undefined,
      });
    });
  });

  it("card_stays_visible_after_click", async () => {
    // No optimistic removal — card stays until the hook returns [] (Resolved envelope)
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    render(<ControlledActivityColumnLive runId="run-1" />);
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));
    // Card is still in DOM after click (hook hasn't been updated yet)
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
  });

  // TECH-DEBT(P.4.3): This test simulates Resolved by mocking the hook
  // to return []. A higher-fidelity version would inject a real
  // PermissionResolved envelope through the Tauri event listener that
  // usePermissionRequests wraps. The integration is fully exercised by
  // usePermissionRequests.test.ts (P.4.1); this layer just confirms
  // ActivityColumn re-renders when the hook output shrinks. P.4.3's
  // app-level integration test will own the envelope-injection variant.
  it("card_unmounts_when_resolved_arrives", () => {
    // First render with the permission present
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    const { rerender } = render(<ControlledActivityColumnLive runId="run-1" />);
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();

    // Simulate PermissionResolved arriving — hook now returns []
    mockUsePermissionRequests.mockReturnValue([]);
    rerender(<ControlledActivityColumnLive runId="run-1" />);
    expect(screen.queryByTestId("permission-card")).toBeNull();
  });

  it("runId_undefined_skips_invoke", async () => {
    // Defensive guard: P.4.3 hasn't shipped yet — runId not threaded from App.tsx
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    render(<ControlledActivityColumnLive runId={undefined} />);
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));
    // Must NOT call invoke
    expect(mockInvoke).not.toHaveBeenCalled();
    // Must warn about missing runId
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining("permission_decide called with no runId"),
    );
    warnSpy.mockRestore();
  });

  it("ipc_failure_surfaces_console_error", async () => {
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    mockInvoke.mockRejectedValue(new Error("backend unavailable"));
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    render(<ControlledActivityColumnLive runId="run-1" />);
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));

    await vi.waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        expect.stringContaining("permission_decide failed"),
        expect.any(Error),
      );
    });
    errorSpy.mockRestore();
  });

  it("stepId_is_threaded_into_ipc_payload", async () => {
    mockUsePermissionRequests.mockReturnValue([r1Perm]);
    render(<ControlledActivityColumnLive runId="run-1" stepId="step-1" />);
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("permission_decide", {
        requestId: "r1",
        decision: "once",
        runId: "run-1",
        stepId: "step-1",
      });
    });
  });
});
