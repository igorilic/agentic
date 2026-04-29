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

  describe("F3 — card role, tabIndex, aria-label", () => {
    const cases: Array<{ agent: string; status: AgentStatus }> = [
      { agent: "architect", status: "queued" },
      { agent: "architect", status: "active" },
      { agent: "developer", status: "queued" },
      { agent: "developer", status: "active" },
    ];

    for (const { agent, status } of cases) {
      it(`card has role="button", tabIndex=0, and aria-label for agent="${agent}" status="${status}"`, () => {
        render(<AgentCard agent={agent} status={status} />);
        const card = screen.getByTestId(`agent-card-${agent}`);
        expect(card).toHaveAttribute("role", "button");
        // tabIndex=0 on a non-interactive element: check the property directly
        expect(card.tabIndex).toBe(0);
        expect(card).toHaveAttribute("aria-label", `${agent} — ${status}`);
      });
    }
  });

  describe("F2 — decorative elements have aria-hidden", () => {
    it("active tint overlay has aria-hidden=true", () => {
      render(<AgentCard agent="architect" status="active" />);
      const tint = screen.getByTestId("agent-card-architect-tint");
      expect(tint).toHaveAttribute("aria-hidden", "true");
    });

    it("pulse indicator has aria-hidden=true", () => {
      render(<AgentCard agent="architect" status="active" />);
      const pulse = screen.getByTestId("agent-card-architect-pulse");
      expect(pulse).toHaveAttribute("aria-hidden", "true");
    });
  });

  describe("F4 — per-agent tint color in active state", () => {
    it("architect active tint uses blue rgba (59 130 246)", () => {
      render(<AgentCard agent="architect" status="active" />);
      const tint = screen.getByTestId("agent-card-architect-tint") as HTMLElement;
      const bg = tint.style.backgroundColor;
      expect(bg).toContain("59");
      expect(bg).toContain("130");
      expect(bg).toContain("246");
    });

    it("developer active tint uses green rgba (16 185 129)", () => {
      render(<AgentCard agent="developer" status="active" />);
      const tint = screen.getByTestId("agent-card-developer-tint") as HTMLElement;
      const bg = tint.style.backgroundColor;
      expect(bg).toContain("16");
      expect(bg).toContain("185");
      expect(bg).toContain("129");
    });

    it("qa active tint uses purple rgba (139 92 246)", () => {
      render(<AgentCard agent="qa" status="active" />);
      const tint = screen.getByTestId("agent-card-qa-tint") as HTMLElement;
      const bg = tint.style.backgroundColor;
      expect(bg).toContain("139");
      expect(bg).toContain("92");
      expect(bg).toContain("246");
    });

    it("reviewer active tint uses amber rgba (245 158 11)", () => {
      render(<AgentCard agent="reviewer" status="active" />);
      const tint = screen.getByTestId("agent-card-reviewer-tint") as HTMLElement;
      const bg = tint.style.backgroundColor;
      expect(bg).toContain("245");
      expect(bg).toContain("158");
      expect(bg).toContain("11");
    });

    it("unknown agent falls back to amber rgba (245 158 11)", () => {
      render(<AgentCard agent="researcher" status="active" />);
      const tint = screen.getByTestId("agent-card-researcher-tint") as HTMLElement;
      const bg = tint.style.backgroundColor;
      expect(bg).toContain("245");
      expect(bg).toContain("158");
      expect(bg).toContain("11");
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
