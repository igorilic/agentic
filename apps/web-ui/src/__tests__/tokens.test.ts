import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const tokensCss = readFileSync(
  join(__dirname, "../styles/tokens.css"),
  "utf8"
);

describe("tokens.css", () => {
  it("does not contain @font-face (font loads via CDN <link>)", () => {
    expect(tokensCss).not.toContain("@font-face");
  });

  it("--font-sans value contains Inter", () => {
    expect(tokensCss).toMatch(/--font-sans\s*:\s*Inter/);
  });

  describe(":root block defines all required tokens", () => {
    const requiredTokens = [
      "--bg-page",
      "--bg-surface",
      "--bg-surface-2",
      "--fg",
      "--fg-muted",
      "--fg-subtle",
      "--border-soft",
      "--border",
      "--border-strong",
      "--font-sans",
      "--font-mono",
      "--radius-md",
      "--radius-lg",
      "--radius-xl",
    ];

    for (const token of requiredTokens) {
      it(`defines ${token}`, () => {
        expect(tokensCss).toContain(token);
      });
    }
  });

  describe(':root[data-theme="dark"] block', () => {
    it('exists', () => {
      expect(tokensCss).toContain(':root[data-theme="dark"]');
    });

    it("redefines --bg-page", () => {
      const darkBlock = tokensCss.slice(
        tokensCss.indexOf(':root[data-theme="dark"]')
      );
      expect(darkBlock).toContain("--bg-page");
    });

    it("redefines --bg-surface", () => {
      const darkBlock = tokensCss.slice(
        tokensCss.indexOf(':root[data-theme="dark"]')
      );
      expect(darkBlock).toContain("--bg-surface");
    });

    it("redefines --fg", () => {
      const darkBlock = tokensCss.slice(
        tokensCss.indexOf(':root[data-theme="dark"]')
      );
      expect(darkBlock).toContain("--fg");
    });

    it("redefines --fg-muted", () => {
      const darkBlock = tokensCss.slice(
        tokensCss.indexOf(':root[data-theme="dark"]')
      );
      expect(darkBlock).toContain("--fg-muted");
    });
  });

  describe("per-agent accent tokens (spec §6.1)", () => {
    it("defines --agent-architect as #3b82f6", () => {
      expect(tokensCss).toContain("--agent-architect: #3b82f6");
    });

    it("defines --agent-developer as #10b981", () => {
      expect(tokensCss).toContain("--agent-developer: #10b981");
    });

    it("defines --agent-qa as #8b5cf6", () => {
      expect(tokensCss).toContain("--agent-qa: #8b5cf6");
    });

    it("defines --agent-reviewer as #f59e0b", () => {
      expect(tokensCss).toContain("--agent-reviewer: #f59e0b");
    });
  });

  describe("status color tokens (spec §6.1)", () => {
    it("defines --status-done as #10b981", () => {
      expect(tokensCss).toContain("--status-done: #10b981");
    });

    it("defines --status-active as #f59e0b", () => {
      expect(tokensCss).toContain("--status-active: #f59e0b");
    });

    it("defines --status-queued as #a1a1aa", () => {
      expect(tokensCss).toContain("--status-queued: #a1a1aa");
    });

    it("defines --status-failed as #ef4444", () => {
      expect(tokensCss).toContain("--status-failed: #ef4444");
    });

    it("defines --status-info as #3b82f6", () => {
      expect(tokensCss).toContain("--status-info: #3b82f6");
    });
  });

  describe("Catalyst extra tokens (TD1)", () => {
    it("defines --font-display", () => {
      expect(tokensCss).toContain("--font-display");
    });

    it("defines --font-feature-settings", () => {
      expect(tokensCss).toContain("--font-feature-settings");
    });

    it("defines --radius-sm", () => {
      expect(tokensCss).toContain("--radius-sm");
    });

    it("defines --radius-2xl", () => {
      expect(tokensCss).toContain("--radius-2xl");
    });

    it("defines --radius-full", () => {
      expect(tokensCss).toContain("--radius-full");
    });

    it("defines --avatar-radius", () => {
      expect(tokensCss).toContain("--avatar-radius");
    });

    it("defines --shadow-xs", () => {
      expect(tokensCss).toContain("--shadow-xs");
    });

    it("defines --shadow-sm", () => {
      expect(tokensCss).toContain("--shadow-sm");
    });

    it("defines --shadow-md", () => {
      expect(tokensCss).toContain("--shadow-md");
    });

    it("defines --shadow-lg", () => {
      expect(tokensCss).toContain("--shadow-lg");
    });

    it("defines --focus-ring referencing --blue-500", () => {
      expect(tokensCss).toContain("--focus-ring");
      expect(tokensCss).toContain("--blue-500");
    });
  });

  describe("removed pre-named shadow aliases (superseded by Tailwind config)", () => {
    it("does not define --shadow-card", () => {
      expect(tokensCss).not.toContain("--shadow-card:");
    });

    it("does not define --shadow-popover", () => {
      expect(tokensCss).not.toContain("--shadow-popover:");
    });

    it("does not define --shadow-modal", () => {
      expect(tokensCss).not.toContain("--shadow-modal:");
    });
  });
});
