/**
 * Integration tests for Step W.9.1 — pipeline mutation (reorder / insert / remove / skip).
 *
 * All mutations are local-only per spec §6.8.3. No backend IPC. Tech-debt #7
 * tracks eventual persistence.
 *
 * Re-seed contract: `pipelineAgents` is seeded from `runState.steps` on
 * `activeRunId` change ONLY — not on every `runState` tick. That way user
 * edits persist between runs; a new run starts fresh.
 *
 * Drag-reorder index contract (from W.2.7 / PipelineBar.tsx):
 *   finalToIndex = dragFromIndex < gapIndex ? gapIndex - 1 : gapIndex
 *   Self-drops (adjusted === from) are no-ops.
 *   drag(0) → gap-3: adjusted = 0 < 3 ? 2 : 3 = 2 → order: [tdd-developer, qa, architect, reviewer]
 */

import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import App from "../App";
import { DEFAULT_AGENTS } from "../types/run";
import { derivePipelineSeed } from "../utils/derivePipelineSeed";
import type { RunState } from "../types/run";

// ---------------------------------------------------------------------------
// Tauri API mocks — mirror app.test.tsx / AppSettingsModal.test.tsx pattern
// ---------------------------------------------------------------------------
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// window.matchMedia stub required by HeaderBar → useTheme
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/**
 * Return testids for the root agent-card elements only (the ones that carry
 * data-status). Filters out nested child testids like agent-card-foo-avatar,
 * agent-card-foo-menu, etc. which also match the prefix pattern.
 */
function getCardTestIds() {
  return screen
    .getAllByTestId(/^agent-card-/)
    .filter((el) => el.hasAttribute("data-status"))
    .map((el) => el.dataset.testid as string);
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------
beforeEach(() => {
  stubMatchMedia();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  vi.clearAllMocks();
});

// ===========================================================================
// 1. derivePipelineSeed pure helper tests (unit)
// ===========================================================================
describe("derivePipelineSeed", () => {
  const emptyRunState: RunState = { steps: [], totalTokens: 0, totalCostUsd: 0 };

  it("returns DEFAULT_AGENTS when runState.steps is empty", () => {
    expect(derivePipelineSeed(emptyRunState)).toEqual([...DEFAULT_AGENTS]);
  });

  it("returns agents from runState.steps when non-empty", () => {
    const runState: RunState = {
      steps: [
        { agent: "architect", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
        { agent: "developer", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
        { agent: "docs",      status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
      ],
      totalTokens: 0,
      totalCostUsd: 0,
    };
    expect(derivePipelineSeed(runState)).toEqual(["architect", "developer", "docs"]);
  });

  it("returns a new array (not the steps array reference)", () => {
    const runState: RunState = {
      steps: [
        { agent: "architect", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
      ],
      totalTokens: 0,
      totalCostUsd: 0,
    };
    const result = derivePipelineSeed(runState);
    expect(result).not.toBe(runState.steps);
  });
});

// ===========================================================================
// 2. App integration — initial render uses DEFAULT_AGENTS
// ===========================================================================
describe("App pipeline mutation — W.9.1", () => {
  it("initial render shows DEFAULT_AGENTS in order", () => {
    render(<App />);
    const ids = getCardTestIds();
    expect(ids).toEqual(DEFAULT_AGENTS.map((a) => `agent-card-${a}`));
  });

  // =========================================================================
  // 3. Reorder via drag (scenario 1)
  // =========================================================================
  it("reorder: drag architect(0) to gap-3 moves architect to index 2", () => {
    // drag(0) → gap-3: adjusted = 0 < 3 ? 2 : 3 = 2
    // Result: [tdd-developer, qa, architect, reviewer]
    render(<App />);

    const architectCard = screen.getByTestId("agent-card-architect");
    const gap3 = screen.getByTestId("pipeline-gap-3");

    fireEvent.dragStart(architectCard);
    fireEvent.dragOver(gap3);
    fireEvent.drop(gap3);

    const ids = getCardTestIds();
    expect(ids).toEqual([
      "agent-card-tdd-developer",
      "agent-card-qa",
      "agent-card-architect",
      "agent-card-reviewer",
    ]);
  });

  // =========================================================================
  // 4. Append insert via "+ Add agent" (scenario 2)
  // =========================================================================
  it("append insert: click pipeline-add-agent, pick researcher → appended at end", async () => {
    render(<App />);

    await userEvent.click(screen.getByTestId("pipeline-add-agent"));
    expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
    await userEvent.click(screen.getByTestId("agent-picker-row-researcher"));

    expect(screen.getByTestId("agent-card-researcher")).toBeInTheDocument();

    // Researcher should be last
    const ids = getCardTestIds();
    expect(ids[ids.length - 1]).toBe("agent-card-researcher");
    expect(ids).toHaveLength(5);
  });

  // =========================================================================
  // 5. Mid-pipeline insert via insert chip (scenario 3)
  // =========================================================================
  it("mid-insert: click pipeline-insert-2, pick security → security at index 2", async () => {
    render(<App />);

    await userEvent.click(screen.getByTestId("pipeline-insert-2"));
    expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
    await userEvent.click(screen.getByTestId("agent-picker-row-security"));

    const ids = getCardTestIds();
    // Expected: [architect, tdd-developer, security, qa, reviewer]
    expect(ids).toEqual([
      "agent-card-architect",
      "agent-card-tdd-developer",
      "agent-card-security",
      "agent-card-qa",
      "agent-card-reviewer",
    ]);
  });

  // =========================================================================
  // 6. Remove via kebab (scenario 4)
  // =========================================================================
  it("remove: open qa kebab → click Remove → qa card gone, rest in order", () => {
    render(<App />);

    fireEvent.click(screen.getByTestId("agent-card-qa-menu"));
    fireEvent.click(screen.getByTestId("agent-card-qa-menu-remove"));

    expect(screen.queryByTestId("agent-card-qa")).not.toBeInTheDocument();

    const ids = getCardTestIds();
    expect(ids).toEqual([
      "agent-card-architect",
      "agent-card-tdd-developer",
      "agent-card-reviewer",
    ]);
  });

  // =========================================================================
  // 7. Skip via kebab (scenario 5)
  // =========================================================================
  it("skip: open reviewer kebab → click Skip this run → card gets data-skipped='true' and opacity-50", () => {
    render(<App />);

    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu"));
    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu-skip"));

    const reviewerCard = screen.getByTestId("agent-card-reviewer");
    expect(reviewerCard).toHaveAttribute("data-skipped", "true");
    expect(reviewerCard.className).toContain("opacity-50");
  });

  it("skip: card name has line-through class when skipped", () => {
    render(<App />);

    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu"));
    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu-skip"));

    // The agent name span should have line-through class
    const reviewerCard = screen.getByTestId("agent-card-reviewer");
    const nameSpan = reviewerCard.querySelector("span.line-through");
    expect(nameSpan).not.toBeNull();
  });

  it("non-skipped card has data-skipped='false' by default", () => {
    render(<App />);
    const architectCard = screen.getByTestId("agent-card-architect");
    expect(architectCard).toHaveAttribute("data-skipped", "false");
  });

  // =========================================================================
  // 8. Re-seed via derivePipelineSeed — tested as a pure helper above.
  //    Here we verify the integration: removing qa, then confirming a
  //    re-render with new run id resets the list.
  //    (Full IPC mock for activeRunId change is in tech-debt #7's scope;
  //     the pure-helper tests above cover the seed logic directly.)
  // =========================================================================
  it("re-seed smoke: after remove, fresh App render restores DEFAULT_AGENTS (no run active)", () => {
    // Simulate a re-mount (no run id active, so seed = DEFAULT_AGENTS)
    const { unmount } = render(<App />);

    // Remove qa
    fireEvent.click(screen.getByTestId("agent-card-qa-menu"));
    fireEvent.click(screen.getByTestId("agent-card-qa-menu-remove"));
    expect(screen.queryByTestId("agent-card-qa")).not.toBeInTheDocument();

    unmount();

    // Fresh mount — state resets because it's a new React tree
    render(<App />);
    expect(screen.getByTestId("agent-card-qa")).toBeInTheDocument();
    const ids = getCardTestIds();
    expect(ids).toEqual(DEFAULT_AGENTS.map((a) => `agent-card-${a}`));
  });
});
