import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import LogRow from "../components/LogRow";

describe("LogRow", () => {
  describe("info row — architect", () => {
    it("renders with data-testid log-row-info", () => {
      render(<LogRow level="info" t="14:02:08" agent="architect" message="Plan: 4 phases." />);
      expect(screen.getByTestId("log-row-info")).toBeInTheDocument();
    });

    it("renders the bracketed timestamp [14:02:08]", () => {
      render(<LogRow level="info" t="14:02:08" agent="architect" message="Plan: 4 phases." />);
      expect(screen.getByText("[14:02:08]")).toBeInTheDocument();
    });

    it("renders the agent name text", () => {
      render(<LogRow level="info" t="14:02:08" agent="architect" message="Plan: 4 phases." />);
      expect(screen.getByText("architect")).toBeInTheDocument();
    });

    it("renders the message text", () => {
      render(<LogRow level="info" t="14:02:08" agent="architect" message="Plan: 4 phases." />);
      expect(screen.getByText("Plan: 4 phases.")).toBeInTheDocument();
    });

    it("agent name element has data-testid log-row-agent with className containing text-agent-architect", () => {
      render(<LogRow level="info" t="14:02:08" agent="architect" message="Plan: 4 phases." />);
      const agentEl = screen.getByTestId("log-row-agent");
      expect(agentEl.className).toContain("text-agent-architect");
    });

    it("does NOT have the level chip for info", () => {
      render(<LogRow level="info" t="14:02:08" agent="architect" message="Plan: 4 phases." />);
      expect(screen.queryByTestId("log-row-level-chip")).toBeNull();
    });
  });

  describe("status row", () => {
    it("renders with data-testid log-row-status", () => {
      render(<LogRow level="status" t="14:03:00" agent="developer" message="Running tests." />);
      expect(screen.getByTestId("log-row-status")).toBeInTheDocument();
    });

    it("renders the bracketed timestamp", () => {
      render(<LogRow level="status" t="14:03:00" agent="developer" message="Running tests." />);
      expect(screen.getByText("[14:03:00]")).toBeInTheDocument();
    });

    it("renders the agent name text", () => {
      render(<LogRow level="status" t="14:03:00" agent="developer" message="Running tests." />);
      expect(screen.getByText("developer")).toBeInTheDocument();
    });

    it("renders the message text", () => {
      render(<LogRow level="status" t="14:03:00" agent="developer" message="Running tests." />);
      expect(screen.getByText("Running tests.")).toBeInTheDocument();
    });

    it("agent name element has data-testid log-row-agent with className containing text-agent-developer", () => {
      render(<LogRow level="status" t="14:03:00" agent="developer" message="Running tests." />);
      const agentEl = screen.getByTestId("log-row-agent");
      expect(agentEl.className).toContain("text-agent-developer");
    });

    it("does NOT have the level chip for status", () => {
      render(<LogRow level="status" t="14:03:00" agent="developer" message="Running tests." />);
      expect(screen.queryByTestId("log-row-level-chip")).toBeNull();
    });
  });

  describe("error row", () => {
    it("renders with data-testid log-row-error", () => {
      render(
        <LogRow level="error" agent="developer" t="14:06:02" message="redis-cli FLUSHDB blocked" />
      );
      expect(screen.getByTestId("log-row-error")).toBeInTheDocument();
    });

    it("has the level chip with data-testid log-row-level-chip and bg-red-500 className", () => {
      render(
        <LogRow level="error" agent="developer" t="14:06:02" message="redis-cli FLUSHDB blocked" />
      );
      const chip = screen.getByTestId("log-row-level-chip");
      expect(chip).toBeInTheDocument();
      expect(chip.className).toContain("bg-red-500");
    });
  });

  describe("per-agent color routing", () => {
    it("architect → text-agent-architect", () => {
      render(<LogRow level="info" t="14:00:00" agent="architect" message="msg" />);
      expect(screen.getByTestId("log-row-agent").className).toContain("text-agent-architect");
    });

    it("developer → text-agent-developer", () => {
      render(<LogRow level="info" t="14:00:00" agent="developer" message="msg" />);
      expect(screen.getByTestId("log-row-agent").className).toContain("text-agent-developer");
    });

    it("qa → text-agent-qa", () => {
      render(<LogRow level="info" t="14:00:00" agent="qa" message="msg" />);
      expect(screen.getByTestId("log-row-agent").className).toContain("text-agent-qa");
    });

    it("reviewer → text-agent-reviewer", () => {
      render(<LogRow level="info" t="14:00:00" agent="reviewer" message="msg" />);
      expect(screen.getByTestId("log-row-agent").className).toContain("text-agent-reviewer");
    });

    it("researcher (unknown) → no text-agent-* class, fallback to text-fg", () => {
      render(<LogRow level="info" t="14:00:00" agent="researcher" message="msg" />);
      const agentEl = screen.getByTestId("log-row-agent");
      expect(agentEl.className).toContain("text-fg");
      expect(agentEl.className).not.toMatch(/text-agent-[a-z]+/);
    });
  });

  describe("outer container styling", () => {
    it("outer container has className containing font-mono", () => {
      render(<LogRow level="info" t="14:00:00" agent="architect" message="msg" />);
      const el = screen.getByTestId("log-row-info");
      expect(el.className).toContain("font-mono");
    });

    it("outer container has className containing text-[12px]", () => {
      render(<LogRow level="info" t="14:00:00" agent="architect" message="msg" />);
      const el = screen.getByTestId("log-row-info");
      expect(el.className).toContain("text-[12px]");
    });

    it("outer container testid is log-row-{level}", () => {
      render(<LogRow level="error" t="14:00:00" agent="developer" message="boom" />);
      expect(screen.getByTestId("log-row-error")).toBeInTheDocument();
    });
  });

  describe("multiple rows in same render", () => {
    it("an info row and an error row both accessible by their respective testids", () => {
      render(
        <>
          <LogRow level="info" t="14:01:00" agent="architect" message="started" />
          <LogRow level="error" t="14:06:02" message="redis-cli FLUSHDB blocked" agent="developer" />
        </>
      );
      expect(screen.getByTestId("log-row-info")).toBeInTheDocument();
      expect(screen.getByTestId("log-row-error")).toBeInTheDocument();
    });
  });
});
