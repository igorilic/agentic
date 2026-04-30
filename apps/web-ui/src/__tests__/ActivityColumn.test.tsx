import { useState } from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import ActivityColumn from "../components/ActivityColumn";
import type { EventEnvelope } from "../types/event";
import type { PermissionRequest } from "../types/pipeline";

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

type ControlledWithPermsProps = {
  events: EventEnvelope[];
  pendingPermissions?: PermissionRequest[];
  onPermissionDecision?: (permId: string, decision: "once" | "session" | "deny") => void;
  initialFilter?: "all" | "tool" | "perm" | "error";
};

function ControlledActivityColumnWithPerms({
  events,
  pendingPermissions,
  onPermissionDecision,
  initialFilter = "all",
}: ControlledWithPermsProps) {
  const [filter, setFilter] = useState<"all" | "tool" | "perm" | "error">(initialFilter);
  return (
    <ActivityColumn
      events={events}
      filter={filter}
      onFilterChange={setFilter}
      pendingPermissions={pendingPermissions}
      onPermissionDecision={onPermissionDecision}
    />
  );
}

const examplePerm: PermissionRequest = {
  id: "p1",
  agent: "developer",
  tool: "shell",
  arg: "redis-cli FLUSHDB",
  scope: "shell.destructive",
  risk: "high",
  reason: "Reset Redis to validate cold-start.",
  t: 1_700_000_001_000,
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

  it("uses step_id as agent name, falls back to 'system' when null", () => {
    render(<ControlledActivityColumn events={mixedEvents} />);
    // e1 has stepId: null → agent should be "system"
    // e2 has stepId: "developer" → agent should be "developer"
    const agentLabels = screen.getAllByTestId("log-row-agent");
    const agentTexts = agentLabels.map((el) => el.textContent);
    expect(agentTexts).toContain("system");
    expect(agentTexts).toContain("developer");
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
  data: { permId: "p1" },
});

describe("ActivityColumn — W.7.2 PermissionCard inline rendering", () => {
  it("renders PermissionCard for a matched perm event", () => {
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
      />,
    );
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
    expect(screen.getAllByTestId("event-row")).toHaveLength(1);
  });

  it("filter 'perm' shows permission cards", () => {
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
        initialFilter="perm"
      />,
    );
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
  });

  it("filter 'tool' hides permission cards", () => {
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
        initialFilter="tool"
      />,
    );
    expect(screen.queryByTestId("permission-card")).toBeNull();
  });

  it("filter 'all' includes permission cards", () => {
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
        initialFilter="all"
      />,
    );
    expect(screen.getByTestId("permission-card")).toBeInTheDocument();
  });

  it("header counts.perm reflects 1 when one matched perm event present", () => {
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
      />,
    );
    expect(screen.getByTestId("activity-tab-perm-count")).toHaveTextContent("1");
  });

  it("unmatched perm event (permId not in pendingPermissions) drops out", () => {
    const unmatchedEnvelope = envelope({
      id: "pe2",
      type: "PermissionRequest",
      data: { permId: "p999" },
    });
    render(
      <ControlledActivityColumnWithPerms
        events={[unmatchedEnvelope]}
        pendingPermissions={[]}
      />,
    );
    expect(screen.queryByTestId("permission-card")).toBeNull();
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
    expect(screen.getByTestId("activity-tab-perm-count")).toHaveTextContent("0");
  });

  it("omitting pendingPermissions prop causes perm events to drop entirely", () => {
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        // pendingPermissions omitted
      />,
    );
    expect(screen.queryByTestId("permission-card")).toBeNull();
    expect(screen.queryAllByTestId("event-row")).toHaveLength(0);
  });

  it("decision callback fires onPermissionDecision('p1', 'once') on Allow once", () => {
    const onDecision = vi.fn();
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
        onPermissionDecision={onDecision}
      />,
    );
    fireEvent.click(screen.getByTestId("permission-card-allow-once"));
    expect(onDecision).toHaveBeenCalledWith("p1", "once");
  });

  it("decision callback fires onPermissionDecision('p1', 'session') on Allow for session", () => {
    const onDecision = vi.fn();
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
        onPermissionDecision={onDecision}
      />,
    );
    fireEvent.click(screen.getByTestId("permission-card-allow-session"));
    expect(onDecision).toHaveBeenCalledWith("p1", "session");
  });

  it("decision callback fires onPermissionDecision('p1', 'deny') on Deny", () => {
    const onDecision = vi.fn();
    render(
      <ControlledActivityColumnWithPerms
        events={[permEnvelope]}
        pendingPermissions={[examplePerm]}
        onPermissionDecision={onDecision}
      />,
    );
    fireEvent.click(screen.getByTestId("permission-card-deny"));
    expect(onDecision).toHaveBeenCalledWith("p1", "deny");
  });

  it("mixed event stream: RunStarted + PermissionRequest + ToolCall all visible under filter=all", () => {
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
        pendingPermissions={[examplePerm]}
        initialFilter="all"
      />,
    );
    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(3);
    // The perm row is at index 1; it contains the PermissionCard
    expect(rows[1].querySelector('[data-testid="permission-card"]')).not.toBeNull();
  });

  it("W.5.4 backward compat: existing tests still pass without pendingPermissions (perm events drop, others show)", () => {
    // existing ControlledActivityColumn never passes pendingPermissions — verify it still works
    render(<ControlledActivityColumn events={mixedEvents} />);
    expect(screen.getAllByTestId("event-row")).toHaveLength(5);
    expect(screen.queryByTestId("permission-card")).toBeNull();
  });
});
