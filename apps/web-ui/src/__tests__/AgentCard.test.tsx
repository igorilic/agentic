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

  describe("kebab menu interaction", () => {
    it("clicking kebab opens menu with Remove, Skip this run, Configure… buttons", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(screen.getByTestId("agent-card-architect-menu-remove")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-architect-menu-skip")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-architect-menu-configure")).toBeInTheDocument();
    });

    it("menu items have correct visible text labels", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(screen.getByTestId("agent-card-architect-menu-remove").textContent).toBe("Remove");
      expect(screen.getByTestId("agent-card-architect-menu-skip").textContent).toBe("Skip this run");
      // U+2026 ellipsis character — single char, not three dots
      expect(screen.getByTestId("agent-card-architect-menu-configure").textContent).toBe("Configure…");
    });

    it("menu uses correct agent name in testids for developer", () => {
      render(<AgentCard agent="developer" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-developer-menu"));
      expect(screen.getByTestId("agent-card-developer-menu-remove")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-developer-menu-skip")).toBeInTheDocument();
      expect(screen.getByTestId("agent-card-developer-menu-configure")).toBeInTheDocument();
    });

    it("menu is not visible before kebab is clicked", () => {
      render(<AgentCard agent="architect" status="queued" />);
      expect(screen.queryByTestId("agent-card-architect-menu-remove")).toBeNull();
    });

    it("clicking Remove fires onRemove once and closes the menu", () => {
      const onRemove = vi.fn();
      render(<AgentCard agent="architect" status="queued" onRemove={onRemove} />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-remove"));
      expect(onRemove).toHaveBeenCalledTimes(1);
      expect(screen.queryByTestId("agent-card-architect-menu-remove")).toBeNull();
    });

    it("clicking Skip fires onSkip once and closes the menu", () => {
      const onSkip = vi.fn();
      render(<AgentCard agent="architect" status="queued" onSkip={onSkip} />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-skip"));
      expect(onSkip).toHaveBeenCalledTimes(1);
      expect(screen.queryByTestId("agent-card-architect-menu-skip")).toBeNull();
    });

    it("clicking Remove when no onRemove provided does not throw", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(() => {
        fireEvent.click(screen.getByTestId("agent-card-architect-menu-remove"));
      }).not.toThrow();
    });

    it("clicking Skip when no onSkip provided does not throw", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(() => {
        fireEvent.click(screen.getByTestId("agent-card-architect-menu-skip"));
      }).not.toThrow();
    });

    it("clicking Configure… opens the placeholder modal and closes the menu", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-configure"));
      expect(screen.getByTestId("agent-configure-modal")).toBeInTheDocument();
      expect(screen.queryByTestId("agent-card-architect-menu-remove")).toBeNull();
    });

    it("modal header text is exactly 'Configure agent — not yet implemented'", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-configure"));
      const modal = screen.getByTestId("agent-configure-modal");
      expect(modal.textContent).toContain("Configure agent — not yet implemented");
    });

    it("clicking agent-configure-backdrop closes the modal", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-configure"));
      expect(screen.getByTestId("agent-configure-modal")).toBeInTheDocument();
      fireEvent.click(screen.getByTestId("agent-configure-backdrop"));
      expect(screen.queryByTestId("agent-configure-modal")).toBeNull();
    });

    it("clicking agent-configure-close closes the modal", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-configure"));
      expect(screen.getByTestId("agent-configure-modal")).toBeInTheDocument();
      fireEvent.click(screen.getByTestId("agent-configure-close"));
      expect(screen.queryByTestId("agent-configure-modal")).toBeNull();
    });

    it("clicking inside the modal panel does NOT close the modal", () => {
      render(<AgentCard agent="architect" status="queued" />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      fireEvent.click(screen.getByTestId("agent-card-architect-menu-configure"));
      // Click on the modal panel itself (not the backdrop)
      fireEvent.click(screen.getByTestId("agent-configure-modal"));
      expect(screen.getByTestId("agent-configure-modal")).toBeInTheDocument();
    });

    it("outside-click closes the menu without firing onRemove or onSkip", () => {
      const onRemove = vi.fn();
      const onSkip = vi.fn();
      render(<AgentCard agent="architect" status="queued" onRemove={onRemove} onSkip={onSkip} />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(screen.getByTestId("agent-card-architect-menu-remove")).toBeInTheDocument();
      // Simulate outside mousedown
      fireEvent.mouseDown(document.body);
      expect(screen.queryByTestId("agent-card-architect-menu-remove")).toBeNull();
      expect(onRemove).not.toHaveBeenCalled();
      expect(onSkip).not.toHaveBeenCalled();
    });

    it("Escape key closes the menu without firing callbacks", () => {
      const onRemove = vi.fn();
      const onSkip = vi.fn();
      render(<AgentCard agent="architect" status="queued" onRemove={onRemove} onSkip={onSkip} />);
      fireEvent.click(screen.getByTestId("agent-card-architect-menu"));
      expect(screen.getByTestId("agent-card-architect-menu-remove")).toBeInTheDocument();
      fireEvent.keyDown(screen.getByTestId("agent-card-architect-menu-list"), { key: "Escape" });
      expect(screen.queryByTestId("agent-card-architect-menu-remove")).toBeNull();
      expect(onRemove).not.toHaveBeenCalled();
      expect(onSkip).not.toHaveBeenCalled();
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

  describe("SF-1 — kebab button aria-haspopup and aria-expanded", () => {
    it("kebab button has aria-haspopup='true' and aria-expanded='false' when menu is closed", () => {
      render(<AgentCard agent="architect" status="queued" />);
      const btn = screen.getByTestId("agent-card-architect-menu");
      expect(btn.getAttribute("aria-haspopup")).toBe("true");
      expect(btn.getAttribute("aria-expanded")).toBe("false");
    });

    it("aria-expanded flips to 'true' after clicking kebab to open the menu", () => {
      render(<AgentCard agent="architect" status="queued" />);
      const btn = screen.getByTestId("agent-card-architect-menu");
      fireEvent.click(btn);
      expect(btn.getAttribute("aria-expanded")).toBe("true");
    });

    it("aria-expanded flips back to 'false' after pressing Escape to close the menu", () => {
      render(<AgentCard agent="architect" status="queued" />);
      const btn = screen.getByTestId("agent-card-architect-menu");
      fireEvent.click(btn);
      expect(btn.getAttribute("aria-expanded")).toBe("true");
      fireEvent.keyDown(screen.getByTestId("agent-card-architect-menu-list"), { key: "Escape" });
      expect(btn.getAttribute("aria-expanded")).toBe("false");
    });

    it("aria-expanded flips back to 'false' after outside mousedown closes the menu", () => {
      render(<AgentCard agent="architect" status="queued" />);
      const btn = screen.getByTestId("agent-card-architect-menu");
      fireEvent.click(btn);
      expect(btn.getAttribute("aria-expanded")).toBe("true");
      fireEvent.mouseDown(document.body);
      expect(btn.getAttribute("aria-expanded")).toBe("false");
    });
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
