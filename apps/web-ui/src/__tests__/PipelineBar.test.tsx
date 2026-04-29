import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
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
});
