/**
 * Integration tests for Step W.9.1 — pipeline mutation (reorder / insert / remove / skip).
 *
 * All mutations are local-only per spec §6.8.3. No backend IPC. Tech-debt #7
 * tracks eventual persistence.
 *
 * I.7 change: DEFAULT_AGENTS no longer exported from run.ts.
 * derivePipelineSeed returns [] when steps is empty (no fallback).
 * App integration tests now pre-seed localStorage so the pipeline has agents.
 *
 * Drag-reorder index contract (from W.2.7 / PipelineBar.tsx):
 *   finalToIndex = dragFromIndex < gapIndex ? gapIndex - 1 : gapIndex
 *   Self-drops (adjusted === from) are no-ops.
 *   drag(0) → gap-3: adjusted = 0 < 3 ? 2 : 3 = 2 → order: [tdd-developer, qa, architect, reviewer]
 */

import { render, screen, fireEvent, act } from "@testing-library/react";
import { renderHook } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import App from "../App";
import { derivePipelineSeed } from "../utils/derivePipelineSeed";
import { usePipelineMutation } from "../hooks/usePipelineMutation";
import type { RunState } from "../types/run";
import type { AgentInfoDto } from "../types/agents";

// The canonical 4-agent pipeline used by these tests.
// No longer imported from types/run (DEFAULT_AGENTS dropped in I.7).
const DEFAULT_AGENTS = ["architect", "tdd-developer", "qa", "reviewer"];

// AgentPicker now calls useDiscoverableAgents. Mock it here so App tests
// do not require Tauri IPC infrastructure beyond the basic invoke mock.
vi.mock("../hooks/useDiscoverableAgents", () => ({
  useDiscoverableAgents: vi.fn(),
}));
import { useDiscoverableAgents } from "../hooks/useDiscoverableAgents";

const APP_AGENTS: AgentInfoDto[] = [
  { name: "architect",  description: "Designs system & breaks down work",  source: "project" },
  { name: "developer",  description: "Writes code & tests",                source: "project" },
  { name: "qa",         description: "Runs tests, checks edge cases",      source: "project" },
  { name: "reviewer",   description: "Code review & feedback",             source: "project" },
  { name: "researcher", description: "Gathers context, reads docs",        source: "project" },
  { name: "security",   description: "Audits for vulnerabilities",         source: "project" },
  { name: "perf",       description: "Profiles & optimises hot paths",     source: "project" },
  { name: "docs",       description: "Updates README, API docs",           source: "project" },
  { name: "designer",   description: "UX & visual review",                 source: "project" },
  { name: "db",         description: "Schema migrations & data",           source: "project" },
  { name: "devops",     description: "CI/CD & deploy config",              source: "project" },
  { name: "a11y",       description: "WCAG compliance pass",               source: "project" },
];

// ---------------------------------------------------------------------------
// Tauri API mocks — mirror app.test.tsx / AppSettingsModal.test.tsx pattern
// ---------------------------------------------------------------------------
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation(async (cmd: string) => {
    if (cmd === "get_workspace_id") return "ws-test-mutation";
    if (cmd === "list_runs") return [];
    if (cmd === "list_findings") return [];
    if (cmd === "get_event_history") return [];
    return [];
  }),
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
  // Pre-seed localStorage with the 4-agent pipeline so tests that need
  // an initialized pipeline work as expected. I.7: no DEFAULT_AGENTS fallback.
  localStorage.setItem("agentic.pipeline.ws-test-mutation", JSON.stringify(DEFAULT_AGENTS));
});

// ===========================================================================
// 1. derivePipelineSeed pure helper tests (unit)
// ===========================================================================
describe("derivePipelineSeed", () => {
  const emptyRunState: RunState = { steps: [], totalTokens: 0, totalCostUsd: 0 };

  it("returns [] when runState.steps is empty (no DEFAULT_AGENTS fallback — I.7)", () => {
    expect(derivePipelineSeed(emptyRunState)).toEqual([]);
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
// 2. App integration — initial render uses persisted localStorage pipeline
// ===========================================================================
describe("App pipeline mutation — W.9.1", () => {
  beforeEach(() => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: APP_AGENTS,
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

  it("initial render shows persisted DEFAULT_AGENTS from localStorage in order", async () => {
    render(<App />);
    // Wait for wsId to be fetched and pipeline to hydrate from localStorage
    await vi.waitFor(() => {
      const ids = getCardTestIds();
      expect(ids).toEqual(DEFAULT_AGENTS.map((a) => `agent-card-${a}`));
    });
  });

  // =========================================================================
  // 3. Reorder via drag (scenario 1)
  // =========================================================================
  it("reorder: drag architect(0) to gap-3 moves architect to index 2", async () => {
    // drag(0) → gap-3: adjusted = 0 < 3 ? 2 : 3 = 2
    // Result: [tdd-developer, qa, architect, reviewer]
    render(<App />);
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
    });

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
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
    });

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
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
    });

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
  it("remove: open qa kebab → click Remove → qa card gone, rest in order", async () => {
    render(<App />);
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-qa")).toBeInTheDocument();
    });

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
  it("skip: open reviewer kebab → click Skip this run → card gets data-skipped='true' and opacity-50", async () => {
    render(<App />);
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-reviewer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu"));
    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu-skip"));

    const reviewerCard = screen.getByTestId("agent-card-reviewer");
    expect(reviewerCard).toHaveAttribute("data-skipped", "true");
    expect(reviewerCard.className).toContain("opacity-50");
  });

  it("skip: card name has line-through class when skipped", async () => {
    render(<App />);
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-reviewer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu"));
    fireEvent.click(screen.getByTestId("agent-card-reviewer-menu-skip"));

    // The agent name span should have line-through class
    const reviewerCard = screen.getByTestId("agent-card-reviewer");
    const nameSpan = reviewerCard.querySelector("span.line-through");
    expect(nameSpan).not.toBeNull();
  });

  it("non-skipped card has data-skipped='false' by default", async () => {
    render(<App />);
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
    });
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
  it("on initial mount with localStorage pipeline, seeds pipelineAgents from it", async () => {
    const { unmount } = render(<App />);
    await vi.waitFor(() => {
      expect(screen.getByTestId("agent-card-qa")).toBeInTheDocument();
    });

    // Remove qa
    fireEvent.click(screen.getByTestId("agent-card-qa-menu"));
    fireEvent.click(screen.getByTestId("agent-card-qa-menu-remove"));
    expect(screen.queryByTestId("agent-card-qa")).not.toBeInTheDocument();

    unmount();

    // Fresh mount — localStorage now has the updated 3-agent list (qa removed was persisted)
    // so we get 3 agents
    render(<App />);
    await vi.waitFor(() => {
      const ids = getCardTestIds();
      expect(ids).toHaveLength(3);
    });
  });

  // =========================================================================
  // 9. Guard: re-seed does NOT fire when activeRunId transitions to undefined
  //    (spec §6.8.3: re-seed only on undefined → string, not string → undefined)
  // =========================================================================
  it("does not re-seed when activeRunId becomes undefined (run cancelled)", () => {
    // Set up a run state with steps (so seed != DEFAULT_AGENTS)
    const runStateWithSteps: RunState = {
      steps: [
        { agent: "architect", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
        { agent: "tdd-developer", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
        { agent: "qa", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
        { agent: "reviewer", status: "pending", tokens: 0, costUsd: null, durationMs: 0, summary: null },
      ],
      totalTokens: 0,
      totalCostUsd: 0,
    };

    // Mount with an active run id — seeds from runState
    type HookProps = { runState: RunState; activeRunId: string | undefined };
    const initialProps: HookProps = { runState: runStateWithSteps, activeRunId: "run-1" };
    const { result, rerender } = renderHook(
      ({ runState, activeRunId }: HookProps) =>
        usePipelineMutation(runState, activeRunId),
      { initialProps },
    );

    // Simulate user edit: remove qa (index 2)
    act(() => {
      result.current.onRemove(2);
    });

    // User now has 3 agents (qa removed)
    expect(result.current.pipelineAgents).toEqual(["architect", "tdd-developer", "reviewer"]);

    // Simulate run cancellation: activeRunId → undefined
    rerender({ runState: runStateWithSteps, activeRunId: undefined });

    // Guard should have blocked the re-seed — user edits are preserved
    expect(result.current.pipelineAgents).toEqual(["architect", "tdd-developer", "reviewer"]);
  });
});
