import { render, screen } from "@testing-library/react";
import EventList from "../components/EventList";
import type { EventEnvelope } from "../types/event";

function makeEnv(id: string, type: string, content: string): EventEnvelope {
  return {
    schema_version: 1,
    event_id: id,
    run_id: "run-1",
    step_id: "step-1",
    timestamp_ms: 1700000000000,
    event: { type, data: { content } },
  };
}

describe("EventList", () => {
  it("renders the empty state when no events", () => {
    render(<EventList events={[]} />);
    expect(screen.getByText(/no events yet/i)).toBeInTheDocument();
  });

  it("renders one row per envelope, in order", () => {
    const events: EventEnvelope[] = [
      makeEnv("e1", "StepStarted", "first"),
      makeEnv("e2", "TextDelta", "second"),
      makeEnv("e3", "StepComplete", "third"),
    ];
    render(<EventList events={events} />);

    const rows = screen.getAllByTestId("event-row");
    expect(rows).toHaveLength(3);

    // First row mentions StepStarted
    expect(rows[0]).toHaveTextContent("StepStarted");
    expect(rows[1]).toHaveTextContent("TextDelta");
    expect(rows[2]).toHaveTextContent("StepComplete");
  });

  it("falls back to short run_id when step_id is null", () => {
    const events: EventEnvelope[] = [
      {
        schema_version: 1,
        event_id: "e1",
        run_id: "01abcdef-test-run",
        step_id: null,
        timestamp_ms: 1700000000000,
        event: { type: "RunStarted", data: {} },
      },
    ];
    render(<EventList events={events} />);
    const row = screen.getByTestId("event-row");
    expect(row).toHaveTextContent("01abcdef"); // first 8 chars of run_id
  });
});
