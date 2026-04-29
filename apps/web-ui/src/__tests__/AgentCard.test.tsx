import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import AgentCard from "../components/AgentCard";
import type { AgentStatus } from "../types/pipeline";

describe("AgentCard", () => {
  describe("basic rendering for queued / active / done variants", () => {
    const statuses: AgentStatus[] = ["queued", "active", "done"];

    for (const status of statuses) {
      it(`renders card with data-testid and data-status="${status}"`, () => {
        render(<AgentCard agent="architect" status={status} />);
        const card = screen.getByTestId("agent-card-architect");
        expect(card).toBeInTheDocument();
        expect(card).toHaveAttribute("data-status", status);
      });

      it(`renders kebab menu button for status="${status}"`, () => {
        render(<AgentCard agent="architect" status={status} />);
        const menu = screen.getByTestId("agent-card-architect-menu");
        expect(menu).toBeInTheDocument();
      });
    }
  });

  describe("active variant specifics", () => {
    it("card className contains border-status-active when status is active", () => {
      render(<AgentCard agent="architect" status="active" />);
      const card = screen.getByTestId("agent-card-architect");
      expect(card.className).toContain("border-status-active");
    });

    it("renders animate-pulse indicator when status is active", () => {
      render(<AgentCard agent="architect" status="active" />);
      const pulse = screen.getByTestId("agent-card-architect-pulse");
      expect(pulse).toBeInTheDocument();
      expect(pulse.className).toContain("animate-pulse");
    });

    it("renders soft accent tint overlay when status is active", () => {
      render(<AgentCard agent="architect" status="active" />);
      const tint = screen.getByTestId("agent-card-architect-tint");
      expect(tint).toBeInTheDocument();
    });
  });

  describe("queued variant — no pulse, no tint", () => {
    it("does NOT render animate-pulse indicator when status is queued", () => {
      render(<AgentCard agent="architect" status="queued" />);
      expect(screen.queryByTestId("agent-card-architect-pulse")).toBeNull();
    });
  });

  describe("done variant — no pulse, no tint", () => {
    it("does NOT render animate-pulse indicator when status is done", () => {
      render(<AgentCard agent="architect" status="done" />);
      expect(screen.queryByTestId("agent-card-architect-pulse")).toBeNull();
    });
  });

  describe("agent name drives testids", () => {
    it("uses agent name in testid — developer", () => {
      render(<AgentCard agent="developer" status="queued" />);
      expect(screen.getByTestId("agent-card-developer")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-developer-menu")).toBeInTheDocument();
    });
  });

  describe("kebab button interaction", () => {
    it("calls onMenuClick when kebab button is clicked", () => {
      const handler = vi.fn();
      render(<AgentCard agent="architect" status="queued" onMenuClick={handler} />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(handler).toHaveBeenCalledTimes(1);
    });

    it("does not throw when kebab is clicked and no onMenuClick is provided", () => {
      render(<AgentCard agent="architect" status="queued" />);
      expect(() => {
        fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      }).not.toThrow();
    });
  });

  describe("avatar tile per-agent accent bg class", () => {
    const agentColors: Array<[string, string]> = [
      ["architect", "bg-agent-architect"],
      ["developer", "bg-agent-developer"],
      ["qa", "bg-agent-qa"],
      ["reviewer", "bg-agent-reviewer"],
    ];

    for (const [agent, expectedClass] of agentColors) {
      it(`avatar tile for "${agent}" has class ${expectedClass}`, () => {
        render(<AgentCard agent={agent} status="queued" />);
        const avatar = screen.getByTestId(`agent-card-${agent}-avatar`);
        expect(avatar.className).toContain(expectedClass);
      });
    }

    it("avatar tile for unknown agent has fallback class bg-bg-surface-2 (not bg-agent-*)", () => {
      render(<AgentCard agent="researcher" status="queued" />);
      const avatar = screen.getByTestId("agent-card-researcher-avatar");
      expect(avatar.className).toContain("bg-bg-surface-2");
      expect(avatar.className).not.toMatch(/bg-agent-/);
    });
  });
});
