import { useState } from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import ActivityColumn from "../components/ActivityColumn";
import type { EventEnvelope } from "../types/event";

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
});
