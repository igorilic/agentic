import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import PipelineBar from "../components/PipelineBar";
import type { AgentStatus } from "../types/pipeline";
import type { AgentInfoDto } from "../types/agents";

// AgentPicker now calls useDiscoverableAgents. Mock it here so PipelineBar
// tests do not require Tauri IPC infrastructure.
vi.mock("../hooks/useDiscoverableAgents", () => ({
  useDiscoverableAgents: vi.fn(),
}));
import { useDiscoverableAgents } from "../hooks/useDiscoverableAgents";

const PIPELINE_BAR_AGENTS: AgentInfoDto[] = [
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

const defaultStatuses: Record<string, AgentStatus> = {
  architect: "done",
  developer: "active",
  qa: "queued",
  reviewer: "queued",
};
const defaultAgents = ["architect", "developer", "qa", "reviewer"];

describe("PipelineBar", () => {
  beforeEach(() => {
    vi.mocked(useDiscoverableAgents).mockReturnValue({
      agents: PIPELINE_BAR_AGENTS,
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

  describe("outer container", () => {
    it("renders pipeline-bar testid", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      expect(screen.getByTestId("pipeline-bar")).toBeInTheDocument();
    });

    it("pipeline-bar has class h-[84px]", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      const bar = screen.getByTestId("pipeline-bar");
      expect(bar.className).toContain("h-[84px]");
    });
  });

  describe("agent cards", () => {
    it("renders four agent-card testids for default agents", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      expect(screen.getByTestId("agent-card-architect")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-developer")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-qa")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-reviewer")).toBeInTheDocument();
    });

    it("renders agent cards in pipeline order", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      const cards = Array.from(
        screen.getAllByTestId(/^agent-card-(architect|developer|qa|reviewer)$/)
      ).map((el) => el.dataset.testid);
      expect(cards).toEqual([
        "agent-card-architect",
        "agent-card-developer",
        "agent-card-qa",
        "agent-card-reviewer",
      ]);
    });
  });

  describe("connectors", () => {
    it("renders 4 connector testids for 4 agents", () => {
      // gap-0 (before first card) + gaps 1, 2, 3 (between cards) each include
      // a Connector. The trailing end-gap (gap-4) does not include a Connector.
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      const connectors = screen.getAllByTestId("connector");
      expect(connectors).toHaveLength(4);
    });

    it("each connector has data-active='false'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      const connectors = screen.getAllByTestId("connector");
      for (const connector of connectors) {
        expect(connector).toHaveAttribute("data-active", "false");
      }
    });
  });

  describe("+ Add agent end cap", () => {
    it("renders pipeline-add-agent testid", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      expect(screen.getByTestId("pipeline-add-agent")).toBeInTheDocument();
    });

    it("end cap contains text '+ Add agent'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      const btn = screen.getByTestId("pipeline-add-agent");
      expect(btn).toHaveTextContent("+ Add agent");
    });
  });

  describe("2-agent variant", () => {
    it("renders 2 connectors and 2 cards in order for ['architect','qa']", () => {
      // gap-0 (before first card) + gap-1 (between the two cards) each include
      // a Connector; the trailing end-gap does not.
      render(
        <PipelineBar
          agents={["architect", "qa"]}
          statuses={{ architect: "done", qa: "queued" }}
          activeIndex={0}
        />
      );
      expect(screen.getAllByTestId("connector")).toHaveLength(2);
      const cards = Array.from(
        screen.getAllByTestId(/^agent-card-(architect|qa)$/)
      ).map((el) => el.dataset.testid);
      expect(cards).toEqual(["agent-card-architect", "agent-card-qa"]);
    });
  });

  describe("empty agents prop", () => {
    it("renders no agent-card testids and no connectors when agents is empty", () => {
      render(
        <PipelineBar
          agents={[]}
          statuses={{}}
          activeIndex={-1}
        />
      );
      expect(screen.queryAllByTestId(/^agent-card-/)).toHaveLength(0);
      expect(screen.queryAllByTestId("connector")).toHaveLength(0);
    });

    it("still renders pipeline-add-agent end cap when agents is empty", () => {
      render(
        <PipelineBar
          agents={[]}
          statuses={{}}
          activeIndex={-1}
        />
      );
      expect(screen.getByTestId("pipeline-add-agent")).toBeInTheDocument();
    });

    // I.7 — empty-state UX
    it("renders pipeline-empty-state with 'Add an agent to get started' when agents is empty", () => {
      render(
        <PipelineBar
          agents={[]}
          statuses={{}}
          activeIndex={-1}
        />
      );
      const emptyState = screen.getByTestId("pipeline-empty-state");
      expect(emptyState).toBeInTheDocument();
      expect(emptyState).toHaveTextContent("Add an agent to get started");
    });

    it("does NOT render pipeline-empty-state when agents is non-empty", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      expect(screen.queryByTestId("pipeline-empty-state")).toBeNull();
    });
  });

  describe("insert chips", () => {
    it("renders 4 insert chips for a 4-agent pipeline", () => {
      // gap-0 (before first card) adds pipeline-insert-0; gaps 1-3 (between
      // cards) add pipeline-insert-1 through pipeline-insert-3. The trailing
      // end-gap (gap-4) has no insert button — that role is filled by
      // the '+ Add agent' end cap.
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={vi.fn()}
        />
      );
      const chips = screen.queryAllByTestId(/^pipeline-insert-\d+$/);
      expect(chips).toHaveLength(4);
    });

    it("each chip has aria-label 'Insert agent at position {atIndex}'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={vi.fn()}
        />
      );
      expect(screen.getByTestId("pipeline-insert-1")).toHaveAttribute(
        "aria-label",
        "Insert agent at position 1"
      );
      expect(screen.getByTestId("pipeline-insert-2")).toHaveAttribute(
        "aria-label",
        "Insert agent at position 2"
      );
      expect(screen.getByTestId("pipeline-insert-3")).toHaveAttribute(
        "aria-label",
        "Insert agent at position 3"
      );
    });

    it("each chip contains a + character", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={vi.fn()}
        />
      );
      expect(screen.getByTestId("pipeline-insert-1")).toHaveTextContent("+");
      expect(screen.getByTestId("pipeline-insert-2")).toHaveTextContent("+");
      expect(screen.getByTestId("pipeline-insert-3")).toHaveTextContent("+");
    });

    // W.2.6: clicking a chip opens AgentPicker (no direct onInsert call yet)
    it("click pipeline-insert-1 opens AgentPicker", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-1"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("click pipeline-insert-2 opens AgentPicker", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-2"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("click pipeline-insert-3 opens AgentPicker", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-3"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("click pipeline-add-agent opens AgentPicker", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-add-agent"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("2-agent pipeline has exactly 2 insert chips at pipeline-insert-0 and pipeline-insert-1", () => {
      // gap-0 (before first card) + gap-1 (between the two cards).
      render(
        <PipelineBar
          agents={["architect", "qa"]}
          statuses={{ architect: "done", qa: "queued" }}
          activeIndex={0}
          onInsert={vi.fn()}
        />
      );
      const chips = screen.queryAllByTestId(/^pipeline-insert-\d+$/);
      expect(chips).toHaveLength(2);
      expect(screen.getByTestId("pipeline-insert-0")).toBeInTheDocument();
      expect(screen.getByTestId("pipeline-insert-1")).toBeInTheDocument();
    });

    it("click pipeline-insert-1 opens AgentPicker for 2-agent pipeline", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={["architect", "qa"]}
          statuses={{ architect: "done", qa: "queued" }}
          activeIndex={0}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-1"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("click pipeline-add-agent opens AgentPicker for 2-agent pipeline", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={["architect", "qa"]}
          statuses={{ architect: "done", qa: "queued" }}
          activeIndex={0}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-add-agent"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("empty pipeline renders no insert chips", () => {
      render(
        <PipelineBar
          agents={[]}
          statuses={{}}
          activeIndex={-1}
          onInsert={vi.fn()}
        />
      );
      const chips = screen.queryAllByTestId(/^pipeline-insert-\d+$/);
      expect(chips).toHaveLength(0);
    });

    it("click pipeline-add-agent opens AgentPicker for empty pipeline", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={[]}
          statuses={{}}
          activeIndex={-1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-add-agent"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });
  });

  describe("AgentPicker insert flow", () => {
    it("click pipeline-insert-2 then pick QA fires onInsert(2, 'qa')", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-2"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      await userEvent.click(screen.getByTestId("agent-picker-row-researcher"));
      expect(onInsert).toHaveBeenCalledWith(2, "researcher");
      expect(screen.queryByTestId("agent-picker")).not.toBeInTheDocument();
    });

    it("click pipeline-add-agent then pick researcher fires onInsert(4, 'researcher')", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-add-agent"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      await userEvent.click(screen.getByTestId("agent-picker-row-researcher"));
      expect(onInsert).toHaveBeenCalledWith(4, "researcher");
      expect(screen.queryByTestId("agent-picker")).not.toBeInTheDocument();
    });

    it("press Escape closes picker without calling onInsert", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-2"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      await userEvent.keyboard("{Escape}");
      expect(screen.queryByTestId("agent-picker")).not.toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("click outside the picker closes it without calling onInsert", async () => {
      const onInsert = vi.fn();
      render(
        <div>
          <div data-testid="outside">Outside</div>
          <PipelineBar
            agents={defaultAgents}
            statuses={defaultStatuses}
            activeIndex={1}
            onInsert={onInsert}
          />
        </div>
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-2"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      await userEvent.click(screen.getByTestId("outside"));
      expect(screen.queryByTestId("agent-picker")).not.toBeInTheDocument();
      expect(onInsert).not.toHaveBeenCalled();
    });

    it("clicking a different chip switches the open picker to new index", async () => {
      const onInsert = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={onInsert}
        />
      );
      // Open at index 1
      await userEvent.click(screen.getByTestId("pipeline-insert-1"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      // Switch to index 3
      await userEvent.click(screen.getByTestId("pipeline-insert-3"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      // Pick agent — must fire with index 3, not 1
      await userEvent.click(screen.getByTestId("agent-picker-row-researcher"));
      expect(onInsert).toHaveBeenCalledWith(3, "researcher");
      expect(onInsert).not.toHaveBeenCalledWith(1, expect.anything());
    });

    it("AgentPicker excludes already-pipeline agents", async () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={vi.fn()}
        />
      );
      await userEvent.click(screen.getByTestId("pipeline-insert-1"));
      expect(screen.getByTestId("agent-picker")).toBeInTheDocument();
      // Agents already in the pipeline should be excluded
      expect(screen.queryByTestId("agent-picker-row-architect")).not.toBeInTheDocument();
      expect(screen.queryByTestId("agent-picker-row-developer")).not.toBeInTheDocument();
      expect(screen.queryByTestId("agent-picker-row-qa")).not.toBeInTheDocument();
      expect(screen.queryByTestId("agent-picker-row-reviewer")).not.toBeInTheDocument();
      // Agents not in the pipeline should be visible
      expect(screen.getByTestId("agent-picker-row-researcher")).toBeInTheDocument();
    });
  });

  describe("drag-reorder", () => {
    // Gap index N = "before card at position N".
    // Gap 0 is before the first card; gaps 1-3 are between cards;
    // gap 4 is after the last card (before + Add agent).
    // onReorder(fromIndex, finalToIndex) — consumer does a plain splice without adjustment.
    // Adjusted-index contract: finalToIndex = (fromIndex < gapN) ? gapN - 1 : gapN.
    // Self-drop (adjusted === fromIndex) is a no-op; onReorder must NOT be called.

    // --- Bug B: gap-0 must exist as a drop target before the first card ---

    it("gap-0 exists as a drop target before the first card", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      expect(screen.getByTestId("pipeline-gap-0")).toBeInTheDocument();
    });

    it("gap-0 initially has data-drop-active='false'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      expect(screen.getByTestId("pipeline-gap-0")).toHaveAttribute(
        "data-drop-active",
        "false"
      );
    });

    it("drag reviewer(3) to gap-0 calls onReorder(3, 0)", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-reviewer");
      const gap0 = screen.getByTestId("pipeline-gap-0");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap0);
      fireEvent.drop(gap0);
      expect(onReorder).toHaveBeenCalledTimes(1);
      expect(onReorder).toHaveBeenCalledWith(3, 0);
    });

    it("drag developer(1) to gap-0 calls onReorder(1, 0)", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-developer");
      const gap0 = screen.getByTestId("pipeline-gap-0");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap0);
      fireEvent.drop(gap0);
      expect(onReorder).toHaveBeenCalledTimes(1);
      expect(onReorder).toHaveBeenCalledWith(1, 0);
    });

    it("drag architect(0) to gap-0 (self-adjacent) does NOT call onReorder", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap0 = screen.getByTestId("pipeline-gap-0");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap0);
      fireEvent.drop(gap0);
      expect(onReorder).not.toHaveBeenCalled();
    });

    // --- Bug A: dataTransfer.setData must be called in onDragStart ---
    // WKWebView (macOS Tauri) and Webview2 (Windows) require at least one
    // dataTransfer entry to treat a dragstart as valid. jsdom does not
    // enforce this requirement, so existing tests pass regardless.

    it("onDragStart calls dataTransfer.setData('text/plain', stringified index)", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const mockDataTransfer = {
        setData: vi.fn(),
        effectAllowed: "" as DataTransfer["effectAllowed"],
        dropEffect: "none" as DataTransfer["dropEffect"],
      };
      fireEvent.dragStart(card, { dataTransfer: mockDataTransfer });
      expect(mockDataTransfer.setData).toHaveBeenCalledWith("text/plain", "0");
    });

    it("onDragStart sets dataTransfer.effectAllowed to 'move'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-developer"); // index 1
      const mockDataTransfer = {
        setData: vi.fn(),
        effectAllowed: "" as DataTransfer["effectAllowed"],
        dropEffect: "none" as DataTransfer["dropEffect"],
      };
      fireEvent.dragStart(card, { dataTransfer: mockDataTransfer });
      expect(mockDataTransfer.effectAllowed).toBe("move");
    });

    it("renders gap drop targets pipeline-gap-1 through pipeline-gap-4", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      expect(screen.getByTestId("pipeline-gap-1")).toBeInTheDocument();
      expect(screen.getByTestId("pipeline-gap-2")).toBeInTheDocument();
      expect(screen.getByTestId("pipeline-gap-3")).toBeInTheDocument();
      expect(screen.getByTestId("pipeline-gap-4")).toBeInTheDocument();
    });

    it("gaps initially have data-drop-active='false'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      for (let n = 1; n <= 4; n++) {
        expect(screen.getByTestId(`pipeline-gap-${n}`)).toHaveAttribute(
          "data-drop-active",
          "false"
        );
      }
    });

    it("drag architect(0) to gap-2 sets data-drop-active='true' on gap-2", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap2 = screen.getByTestId("pipeline-gap-2");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap2);
      expect(gap2).toHaveAttribute("data-drop-active", "true");
    });

    it("drag architect(0) to gap-2 and drop calls onReorder(0, 1)", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap2 = screen.getByTestId("pipeline-gap-2");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap2);
      fireEvent.drop(gap2);
      expect(onReorder).toHaveBeenCalledTimes(1);
      expect(onReorder).toHaveBeenCalledWith(0, 1);
    });

    it("gap data-drop-active resets to 'false' after drop", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap2 = screen.getByTestId("pipeline-gap-2");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap2);
      expect(gap2).toHaveAttribute("data-drop-active", "true");
      fireEvent.drop(gap2);
      expect(gap2).toHaveAttribute("data-drop-active", "false");
    });

    it("drag reviewer(3) to gap-1 calls onReorder(3, 1)", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-reviewer");
      const gap1 = screen.getByTestId("pipeline-gap-1");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap1);
      fireEvent.drop(gap1);
      expect(onReorder).toHaveBeenCalledTimes(1);
      expect(onReorder).toHaveBeenCalledWith(3, 1);
    });

    it("drag architect(0) to gap-4 (end) calls onReorder(0, 3)", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap4 = screen.getByTestId("pipeline-gap-4");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap4);
      fireEvent.drop(gap4);
      expect(onReorder).toHaveBeenCalledTimes(1);
      expect(onReorder).toHaveBeenCalledWith(0, 3);
    });

    it("drag architect(0) to gap-1 (self-adjacent) does NOT call onReorder", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap1 = screen.getByTestId("pipeline-gap-1");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap1);
      fireEvent.drop(gap1);
      expect(onReorder).not.toHaveBeenCalled();
    });

    it("dragLeave on a gap clears data-drop-active to 'false'", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      const gap2 = screen.getByTestId("pipeline-gap-2");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap2);
      expect(gap2).toHaveAttribute("data-drop-active", "true");
      fireEvent.dragLeave(gap2);
      expect(gap2).toHaveAttribute("data-drop-active", "false");
    });

    it("dragged card gets data-dragging='true' on dragStart", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      fireEvent.dragStart(card);
      expect(card).toHaveAttribute("data-dragging", "true");
    });

    it("dragged card loses data-dragging='true' after dragEnd", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const card = screen.getByTestId("agent-card-architect");
      fireEvent.dragStart(card);
      expect(card).toHaveAttribute("data-dragging", "true");
      fireEvent.dragEnd(card);
      expect(card).toHaveAttribute("data-dragging", "false");
    });

    it("non-dragged cards do not get data-dragging='true' during drag", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={vi.fn()}
        />
      );
      const architectCard = screen.getByTestId("agent-card-architect");
      const developerCard = screen.getByTestId("agent-card-developer");
      fireEvent.dragStart(architectCard);
      expect(developerCard).toHaveAttribute("data-dragging", "false");
    });

    // Right-side self-drop: fromIndex >= gapIndex branch.
    // drag reviewer(3) → gap-3: adjusted = 3 < 3 ? 2 : 3 = 3 → self-drop → no-op.
    it("drag reviewer(3) to gap-3 (self-adjacent left) does NOT call onReorder", () => {
      const onReorder = vi.fn();
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onReorder={onReorder}
        />
      );
      const card = screen.getByTestId("agent-card-reviewer");
      const gap3 = screen.getByTestId("pipeline-gap-3");
      fireEvent.dragStart(card);
      fireEvent.dragOver(gap3);
      fireEvent.drop(gap3);
      expect(onReorder).not.toHaveBeenCalled();
    });
  });
});
