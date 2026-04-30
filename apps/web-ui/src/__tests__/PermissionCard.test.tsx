import PermissionCard from "../components/PermissionCard";
import type { PermissionRequest } from "../types/pipeline";
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

function permission(overrides: Partial<PermissionRequest> = {}): PermissionRequest {
  return {
    id: "p2",
    agent: "developer",
    tool: "shell",
    arg: "redis-cli FLUSHDB",
    scope: "shell.destructive",
    risk: "high",
    reason: "Reset local Redis to validate cold-start bucket behavior.",
    t: 1_700_000_000_000,
    ...overrides,
  };
}

describe("PermissionCard", () => {
  describe("outer container", () => {
    it("renders with data-testid permission-card", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      expect(screen.getByTestId("permission-card")).toBeInTheDocument();
    });
  });

  describe("header row", () => {
    it("renders '{agent} requests permission' header text", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      expect(screen.getByText("developer requests permission")).toBeInTheDocument();
    });

    it("renders warn icon with data-testid permission-card-warn-icon and aria-hidden=true", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const icon = screen.getByTestId("permission-card-warn-icon");
      expect(icon).toBeInTheDocument();
      expect(icon).toHaveAttribute("aria-hidden", "true");
    });
  });

  describe("risk pill", () => {
    it("shows 'HIGH RISK' text and bg-red class for risk=high", () => {
      render(<PermissionCard permission={permission({ risk: "high" })} onDecision={vi.fn()} />);
      const pill = screen.getByTestId("permission-card-risk");
      expect(pill.textContent).toBe("HIGH RISK");
      expect(pill.className).toMatch(/bg-red/);
    });

    it("shows 'MEDIUM' text and bg-amber class for risk=medium", () => {
      render(<PermissionCard permission={permission({ risk: "medium" })} onDecision={vi.fn()} />);
      const pill = screen.getByTestId("permission-card-risk");
      expect(pill.textContent).toBe("MEDIUM");
      expect(pill.className).toMatch(/bg-amber/);
    });

    it("shows 'LOW' text and bg-zinc class for risk=low", () => {
      render(<PermissionCard permission={permission({ risk: "low" })} onDecision={vi.fn()} />);
      const pill = screen.getByTestId("permission-card-risk");
      expect(pill.textContent).toBe("LOW");
      expect(pill.className).toMatch(/bg-zinc/);
    });
  });

  describe("command preview", () => {
    it("renders data-testid permission-card-command with '$ ' prefix and arg", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const preview = screen.getByTestId("permission-card-command");
      expect(preview).toBeInTheDocument();
      expect(preview.textContent).toContain("$ redis-cli FLUSHDB");
    });

    it("command block className contains bg-black", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const preview = screen.getByTestId("permission-card-command");
      expect(preview.className).toContain("bg-black");
    });

    it("command block className contains font-mono", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const preview = screen.getByTestId("permission-card-command");
      expect(preview.className).toContain("font-mono");
    });
  });

  describe("reason text", () => {
    it("renders the reason string in the document", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      expect(
        screen.getByText("Reset local Redis to validate cold-start bucket behavior.")
      ).toBeInTheDocument();
    });
  });

  describe("scope pill", () => {
    it("renders data-testid permission-card-scope containing scope value", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const scopePill = screen.getByTestId("permission-card-scope");
      expect(scopePill).toBeInTheDocument();
      expect(scopePill.textContent).toContain("shell.destructive");
    });
  });

  describe("action buttons presence", () => {
    it("renders permission-card-allow-once button", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      expect(screen.getByTestId("permission-card-allow-once")).toBeInTheDocument();
    });

    it("renders permission-card-allow-session button", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      expect(screen.getByTestId("permission-card-allow-session")).toBeInTheDocument();
    });

    it("renders permission-card-deny button", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      expect(screen.getByTestId("permission-card-deny")).toBeInTheDocument();
    });
  });

  describe("action button callbacks", () => {
    it("clicking Allow once calls onDecision('once') once", () => {
      const onDecision = vi.fn();
      render(<PermissionCard permission={permission()} onDecision={onDecision} />);
      fireEvent.click(screen.getByTestId("permission-card-allow-once"));
      expect(onDecision).toHaveBeenCalledTimes(1);
      expect(onDecision).toHaveBeenCalledWith("once");
    });

    it("clicking Allow for session calls onDecision('session') once", () => {
      const onDecision = vi.fn();
      render(<PermissionCard permission={permission()} onDecision={onDecision} />);
      fireEvent.click(screen.getByTestId("permission-card-allow-session"));
      expect(onDecision).toHaveBeenCalledTimes(1);
      expect(onDecision).toHaveBeenCalledWith("session");
    });

    it("clicking Deny calls onDecision('deny') once", () => {
      const onDecision = vi.fn();
      render(<PermissionCard permission={permission()} onDecision={onDecision} />);
      fireEvent.click(screen.getByTestId("permission-card-deny"));
      expect(onDecision).toHaveBeenCalledTimes(1);
      expect(onDecision).toHaveBeenCalledWith("deny");
    });

    it("Deny button className contains text-red", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const denyBtn = screen.getByTestId("permission-card-deny");
      expect(denyBtn.className).toMatch(/text-red/);
    });
  });

  describe("border and bg styling", () => {
    it("outer card className contains border-l-[3px] for left accent", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const card = screen.getByTestId("permission-card");
      expect(card.className).toContain("border-l-[3px]");
    });

    it("outer card has #fca5a5 border color via arbitrary class or border-red family", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const card = screen.getByTestId("permission-card");
      const hasBorderColor =
        card.className.includes("#fca5a5") || card.className.match(/border-red/) !== null;
      expect(hasBorderColor).toBe(true);
    });

    it("outer card bg uses rgba(252, 165, 165, 0.06) via inline style or arbitrary class", () => {
      render(<PermissionCard permission={permission()} onDecision={vi.fn()} />);
      const card = screen.getByTestId("permission-card") as HTMLElement;
      const hasBgStyle =
        card.style.backgroundColor !== "" ||
        card.className.includes("rgba(252") ||
        card.className.includes("rgba(252,");
      expect(hasBgStyle).toBe(true);
    });
  });

  describe("t field type check", () => {
    it("fixture compiles and renders with t as a number", () => {
      // This is a type-level smoke check. If t were typed as string, this would fail TypeScript.
      const p = permission({ t: 1_700_000_000_000 });
      expect(typeof p.t).toBe("number");
      render(<PermissionCard permission={p} onDecision={vi.fn()} />);
      expect(screen.getByTestId("permission-card")).toBeInTheDocument();
    });
  });
});
