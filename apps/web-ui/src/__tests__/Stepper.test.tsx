import { render, screen } from "@testing-library/react";
import Stepper from "../components/Stepper";
import type { RunState } from "../types/run";

function makeRunState(
  overrides: Partial<RunState["steps"][number]>[] = [],
): RunState {
  const defaultSteps = [
    {
      agent: "architect",
      status: "pending" as const,
      tokens: 0,
      costUsd: null,
      durationMs: 0,
      summary: null,
    },
    {
      agent: "tdd-developer",
      status: "pending" as const,
      tokens: 0,
      costUsd: null,
      durationMs: 0,
      summary: null,
    },
    {
      agent: "qa",
      status: "pending" as const,
      tokens: 0,
      costUsd: null,
      durationMs: 0,
      summary: null,
    },
    {
      agent: "reviewer",
      status: "pending" as const,
      tokens: 0,
      costUsd: null,
      durationMs: 0,
      summary: null,
    },
  ];
  const steps = defaultSteps.map((s, i) => ({ ...s, ...(overrides[i] ?? {}) }));
  return {
    steps,
    totalTokens: steps.reduce((sum, s) => sum + s.tokens, 0),
    totalCostUsd: steps.reduce((sum, s) => sum + (s.costUsd ?? 0), 0),
  };
}

describe("Stepper", () => {
  it("renders all four pipeline steps in order", () => {
    render(<Stepper state={makeRunState()} />);
    const steps = screen.getAllByRole("listitem");
    expect(steps).toHaveLength(4);
    expect(steps[0]).toHaveTextContent("architect");
    expect(steps[1]).toHaveTextContent("tdd-developer");
    expect(steps[2]).toHaveTextContent("qa");
    expect(steps[3]).toHaveTextContent("reviewer");
  });

  it("renders the failure icon when a step status is failed", () => {
    const state = makeRunState([
      { status: "passed" },
      { status: "passed" },
      { status: "failed" },
      { status: "pending" },
    ]);
    render(<Stepper state={state} />);
    const qaStep = screen.getByTestId("stepper-step-qa");
    expect(qaStep).toHaveAttribute("data-status", "failed");
    const qaIcon = screen.getByTestId("stepper-icon-qa");
    expect(qaIcon).toHaveTextContent("✗");
    // The architect step shows passed icon.
    const archIcon = screen.getByTestId("stepper-icon-architect");
    expect(archIcon).toHaveTextContent("✓");
  });

  it("displays total token count summed across steps", () => {
    const state = makeRunState([
      { tokens: 100 },
      { tokens: 250 },
      { tokens: 50 },
      { tokens: 0 },
    ]);
    render(<Stepper state={state} />);
    const totals = screen.getByTestId("stepper-totals");
    expect(totals).toHaveTextContent("Total tokens: 400");
  });

  it("hides cost when total is zero", () => {
    render(<Stepper state={makeRunState()} />);
    const totals = screen.getByTestId("stepper-totals");
    expect(totals).not.toHaveTextContent(/Cost/);
  });

  it("shows cost when at least one step has cost_usd", () => {
    const state = makeRunState([
      { costUsd: 0.0042 },
      { costUsd: null },
      { costUsd: null },
      { costUsd: null },
    ]);
    render(<Stepper state={state} />);
    const totals = screen.getByTestId("stepper-totals");
    expect(totals).toHaveTextContent("$0.0042");
  });
});
