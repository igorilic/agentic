import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import PipelineBar from "../components/PipelineBar";
import type { AgentStatus } from "../types/pipeline";

const defaultStatuses: Record<string, AgentStatus> = {
  architect: "done",
  developer: "active",
  qa: "queued",
  reviewer: "queued",
};
const defaultAgents = ["architect", "developer", "qa", "reviewer"];

describe("PipelineBar", () => {
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
    it("renders 3 connector testids for 4 agents", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
        />
      );
      const connectors = screen.getAllByTestId("connector");
      expect(connectors).toHaveLength(3);
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
    it("renders 1 connector and 2 cards in order for ['architect','qa']", () => {
      render(
        <PipelineBar
          agents={["architect", "qa"]}
          statuses={{ architect: "done", qa: "queued" }}
          activeIndex={0}
        />
      );
      expect(screen.getAllByTestId("connector")).toHaveLength(1);
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
  });

  describe("insert chips", () => {
    it("renders 3 insert chips for a 4-agent pipeline", () => {
      render(
        <PipelineBar
          agents={defaultAgents}
          statuses={defaultStatuses}
          activeIndex={1}
          onInsert={vi.fn()}
        />
      );
      const chips = screen.queryAllByTestId(/^pipeline-insert-\d+$/);
      expect(chips).toHaveLength(3);
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

    it("clicking pipeline-insert-1 calls onInsert with 1", async () => {
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
      expect(onInsert).toHaveBeenCalledWith(1);
    });

    it("clicking pipeline-insert-2 calls onInsert with 2", async () => {
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
      expect(onInsert).toHaveBeenCalledWith(2);
    });

    it("clicking pipeline-insert-3 calls onInsert with 3", async () => {
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
      expect(onInsert).toHaveBeenCalledWith(3);
    });

    it("clicking pipeline-add-agent calls onInsert with agents.length (4)", async () => {
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
      expect(onInsert).toHaveBeenCalledWith(4);
    });

    it("2-agent pipeline has exactly 1 insert chip at pipeline-insert-1", () => {
      render(
        <PipelineBar
          agents={["architect", "qa"]}
          statuses={{ architect: "done", qa: "queued" }}
          activeIndex={0}
          onInsert={vi.fn()}
        />
      );
      const chips = screen.queryAllByTestId(/^pipeline-insert-\d+$/);
      expect(chips).toHaveLength(1);
      expect(screen.getByTestId("pipeline-insert-1")).toBeInTheDocument();
    });

    it("end cap calls onInsert with 2 for a 2-agent pipeline", async () => {
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
      expect(onInsert).toHaveBeenCalledWith(2);
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

    it("end cap calls onInsert with 0 for empty pipeline", async () => {
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
      expect(onInsert).toHaveBeenCalledWith(0);
    });
  });
});
