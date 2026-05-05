/**
 * I.7 fix-loop — F2: HeaderBar "Run pipeline" empty-pipeline guard.
 *
 * The "Run pipeline" button must be disabled when pipelineAgents is empty,
 * with a tooltip "Pick agents in the pipeline rail first".
 *
 * Tests:
 *   - Button is disabled when hasAgents=false
 *   - Button has the correct title attribute when hasAgents=false
 *   - Button is enabled when hasAgents=true
 *   - Clicking a disabled button does NOT call onRunPipeline
 */
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import HeaderBar from "../components/HeaderBar";

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

const baseProps = {
  brand: "Agentic",
  ticketSlug: null as string | null,
  runState: "idle" as const,
  elapsedMs: null as number | null,
  onOpenSettings: vi.fn(),
  onRunPipeline: vi.fn(),
  onStopRun: vi.fn(),
  onRerun: vi.fn(),
};

describe("HeaderBar — F2 empty-pipeline guard", () => {
  beforeEach(() => {
    stubMatchMedia();
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    vi.clearAllMocks();
  });

  it("Run pipeline button is disabled when hasAgents is false", () => {
    render(<HeaderBar {...baseProps} hasAgents={false} />);
    const btn = screen.getByTestId("header-run");
    expect(btn).toBeDisabled();
  });

  it("Run pipeline button has a tooltip when disabled", () => {
    render(<HeaderBar {...baseProps} hasAgents={false} />);
    const btn = screen.getByTestId("header-run");
    expect(btn).toHaveAttribute("title", "Pick agents in the pipeline rail first");
  });

  it("Run pipeline button is enabled when hasAgents is true", () => {
    render(<HeaderBar {...baseProps} hasAgents={true} />);
    const btn = screen.getByTestId("header-run");
    expect(btn).not.toBeDisabled();
  });

  it("clicking a disabled Run pipeline button does NOT call onRunPipeline", async () => {
    const user = userEvent.setup();
    const onRunPipeline = vi.fn();
    render(<HeaderBar {...baseProps} hasAgents={false} onRunPipeline={onRunPipeline} />);
    await user.click(screen.getByTestId("header-run"));
    expect(onRunPipeline).not.toHaveBeenCalled();
  });

  it("clicking an enabled Run pipeline button calls onRunPipeline", async () => {
    const user = userEvent.setup();
    const onRunPipeline = vi.fn();
    render(<HeaderBar {...baseProps} hasAgents={true} onRunPipeline={onRunPipeline} />);
    await user.click(screen.getByTestId("header-run"));
    expect(onRunPipeline).toHaveBeenCalledTimes(1);
  });

  it("Run pipeline button is enabled by default when hasAgents is not provided (backward compat)", () => {
    // Existing tests pass no hasAgents prop — button must remain enabled
    render(<HeaderBar {...baseProps} />);
    const btn = screen.getByTestId("header-run");
    expect(btn).not.toBeDisabled();
  });
});
