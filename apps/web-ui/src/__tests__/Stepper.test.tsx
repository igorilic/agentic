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

  it("renders the warning icon for needs_triage status", () => {
    const state = makeRunState([
      { status: "passed" },
      { status: "passed" },
      { status: "needs_triage" },
      { status: "pending" },
    ]);
    render(<Stepper state={state} />);
    const qaStep = screen.getByTestId("stepper-step-qa");
    expect(qaStep).toHaveAttribute("data-status", "needs_triage");
    const qaIcon = screen.getByTestId("stepper-icon-qa");
    expect(qaIcon).toHaveTextContent("⚠");
  });

  it("renders the skipped icon for skipped status", () => {
    const state = makeRunState([
      { status: "passed" },
      { status: "skipped" },
      { status: "passed" },
      { status: "passed" },
    ]);
    render(<Stepper state={state} />);
    const tddStep = screen.getByTestId("stepper-step-tdd-developer");
    expect(tddStep).toHaveAttribute("data-status", "skipped");
    const tddIcon = screen.getByTestId("stepper-icon-tdd-developer");
    expect(tddIcon).toHaveTextContent("⊘");
  });

  // Responsive layout assertions: class strings lock in the mobile-first intent.
  it("step list has flex-col base layout for vertical stacking at narrow widths", () => {
    render(<Stepper state={makeRunState()} />);
    const ol = screen.getByRole("list");
    expect(ol.className).toMatch(/flex-col/);
  });

  it("step list has sm:flex-row class to restore horizontal layout at sm breakpoint", () => {
    render(<Stepper state={makeRunState()} />);
    const ol = screen.getByRole("list");
    expect(ol.className).toMatch(/sm:flex-row/);
  });

  it("arrow separator has hidden class so it is invisible in the vertical stack", () => {
    const state = makeRunState();
    render(<Stepper state={state} />);
    // There are 3 separators (between 4 steps). Each must carry `hidden`.
    // Scoped to the stepper <ol> to avoid matching aria-hidden elements from
    // other components that may render in the same document.
    const list = screen.getByRole("list");
    const separators = list.querySelectorAll('[aria-hidden="true"]');
    expect(separators.length).toBeGreaterThan(0);
    separators.forEach((sep) => {
      expect(sep.className).toMatch(/hidden/);
    });
  });

  it("arrow separator has sm:inline class to reappear in the horizontal layout", () => {
    const state = makeRunState();
    render(<Stepper state={state} />);
    // Scoped to the stepper <ol> to avoid false positives from sibling elements.
    const list = screen.getByRole("list");
    const separators = list.querySelectorAll('[aria-hidden="true"]');
    separators.forEach((sep) => {
      expect(sep.className).toMatch(/sm:inline/);
    });
  });
});
