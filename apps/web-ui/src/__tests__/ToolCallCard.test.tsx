import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import ToolCallCard from "../components/ToolCallCard";

describe("ToolCallCard", () => {
  describe("header row rendering", () => {
    it("renders outer container with data-testid tool-call-card", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      expect(screen.getByTestId("tool-call-card")).toBeInTheDocument();
    });

    it("renders agent name text", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      expect(screen.getByText("developer")).toBeInTheDocument();
    });

    it("renders tool and arg as read_file(/src/api.ts)", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      // tool and parens may be split across text nodes; match combined textContent
      expect(screen.getByText(/read_file/)).toBeInTheDocument();
      expect(screen.getByText("/src/api.ts")).toBeInTheDocument();
    });
  });

  describe("result chip — OK", () => {
    it("renders result-chip-ok testid", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      expect(screen.getByTestId("result-chip-ok")).toBeInTheDocument();
    });

    it("result-chip-ok has bg-green-100 and text-green-700", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      const chip = screen.getByTestId("result-chip-ok");
      expect(chip.className).toContain("bg-green-100");
      expect(chip.className).toContain("text-green-700");
    });
  });

  describe("result chip — error", () => {
    it("renders result-chip-error testid", () => {
      render(
        <ToolCallCard agent="developer" tool="run_cmd" arg="npm test" result="error" />
      );
      expect(screen.getByTestId("result-chip-error")).toBeInTheDocument();
    });

    it("result-chip-error has bg-red-100 and text-red-700", () => {
      render(
        <ToolCallCard agent="developer" tool="run_cmd" arg="npm test" result="error" />
      );
      const chip = screen.getByTestId("result-chip-error");
      expect(chip.className).toContain("bg-red-100");
      expect(chip.className).toContain("text-red-700");
    });
  });

  describe("result chip — neutral fallback", () => {
    it("renders result-chip-neutral for non-OK/non-error result", () => {
      render(
        <ToolCallCard agent="developer" tool="run_cmd" arg="npm test" result="pending" />
      );
      expect(screen.getByTestId("result-chip-neutral")).toBeInTheDocument();
      expect(screen.queryByTestId("result-chip-ok")).toBeNull();
      expect(screen.queryByTestId("result-chip-error")).toBeNull();
    });

    it("result-chip-neutral has neutral styling", () => {
      render(
        <ToolCallCard agent="developer" tool="run_cmd" arg="npm test" result="pending" />
      );
      const chip = screen.getByTestId("result-chip-neutral");
      expect(chip.className).toContain("bg-bg-surface-2");
      expect(chip.className).toContain("text-fg-muted");
    });
  });

  describe("per-agent color on agent name", () => {
    it("architect agent name has text-agent-architect", () => {
      render(
        <ToolCallCard agent="architect" tool="plan" arg="step1" result="OK" />
      );
      const agentEl = screen.getByTestId("tool-call-card-agent");
      expect(agentEl.className).toContain("text-agent-architect");
    });

    it("developer agent name has text-agent-developer", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      const agentEl = screen.getByTestId("tool-call-card-agent");
      expect(agentEl.className).toContain("text-agent-developer");
    });

    it("qa agent name has text-agent-qa", () => {
      render(
        <ToolCallCard agent="qa" tool="run_tests" arg="all" result="OK" />
      );
      const agentEl = screen.getByTestId("tool-call-card-agent");
      expect(agentEl.className).toContain("text-agent-qa");
    });

    it("reviewer agent name has text-agent-reviewer", () => {
      render(
        <ToolCallCard agent="reviewer" tool="review_diff" arg="HEAD" result="OK" />
      );
      const agentEl = screen.getByTestId("tool-call-card-agent");
      expect(agentEl.className).toContain("text-agent-reviewer");
    });

    it("unknown agent (researcher) falls back to text-fg", () => {
      render(
        <ToolCallCard agent="researcher" tool="search" arg="query" result="OK" />
      );
      const agentEl = screen.getByTestId("tool-call-card-agent");
      expect(agentEl.className).toContain("text-fg");
      expect(agentEl.className).not.toMatch(/text-agent-[a-z]+/);
    });
  });

  describe("card border + radius", () => {
    it("outer container has border and rounded-lg", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      const card = screen.getByTestId("tool-call-card");
      expect(card.className).toContain("border");
      expect(card.className).toContain("rounded-lg");
    });
  });

  describe("no collapsible body when details is undefined", () => {
    it("toggle button is not rendered", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      expect(screen.queryByTestId("tool-call-card-toggle")).toBeNull();
    });

    it("body is not rendered", () => {
      render(
        <ToolCallCard agent="developer" tool="read_file" arg="/src/api.ts" result="OK" />
      );
      expect(screen.queryByTestId("tool-call-card-body")).toBeNull();
    });
  });

  describe("collapsible body when details is provided", () => {
    it("renders the toggle button", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details="stdout: ok\nexit code 0"
        />
      );
      expect(screen.getByTestId("tool-call-card-toggle")).toBeInTheDocument();
    });

    it("body is initially collapsed (not rendered)", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details="stdout: ok\nexit code 0"
        />
      );
      expect(screen.queryByTestId("tool-call-card-body")).toBeNull();
    });

    it("click toggle expands and shows body content", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details={"stdout: ok\nexit code 0"}
        />
      );
      fireEvent.click(screen.getByTestId("tool-call-card-toggle"));
      const body = screen.getByTestId("tool-call-card-body");
      expect(body).toBeInTheDocument();
      expect(body.textContent).toContain("stdout: ok");
      expect(body.textContent).toContain("exit code 0");
    });

    it("click toggle again collapses body", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details={"stdout: ok\nexit code 0"}
        />
      );
      const toggle = screen.getByTestId("tool-call-card-toggle");
      fireEvent.click(toggle);
      expect(screen.getByTestId("tool-call-card-body")).toBeInTheDocument();
      fireEvent.click(toggle);
      expect(screen.queryByTestId("tool-call-card-body")).toBeNull();
    });

    it("expanded body has max-h-[200px] and overflow-y-auto and font-mono", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details="stdout: ok"
        />
      );
      fireEvent.click(screen.getByTestId("tool-call-card-toggle"));
      const body = screen.getByTestId("tool-call-card-body");
      expect(body.className).toContain("max-h-[200px]");
      expect(body.className).toContain("overflow-y-auto");
      expect(body.className).toContain("font-mono");
    });
  });

  describe("toggle button accessibility", () => {
    it("toggle has aria-expanded=false initially", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details="stdout: ok"
        />
      );
      const toggle = screen.getByTestId("tool-call-card-toggle");
      expect(toggle).toHaveAttribute("aria-expanded", "false");
    });

    it("toggle has aria-expanded=true after click", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details="stdout: ok"
        />
      );
      const toggle = screen.getByTestId("tool-call-card-toggle");
      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute("aria-expanded", "true");
    });

    it("toggle has aria-label Toggle details", () => {
      render(
        <ToolCallCard
          agent="developer"
          tool="read_file"
          arg="/src/api.ts"
          result="OK"
          details="stdout: ok"
        />
      );
      const toggle = screen.getByTestId("tool-call-card-toggle");
      expect(toggle).toHaveAttribute("aria-label", "Toggle details");
    });
  });
});
