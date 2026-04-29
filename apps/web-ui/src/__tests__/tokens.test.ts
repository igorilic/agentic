import { describe, it, expect } from "vitest";
import tokensCss from "../styles/tokens.css?raw";

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
      "--shadow-card",
      "--shadow-popover",
      "--shadow-modal",
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
});
