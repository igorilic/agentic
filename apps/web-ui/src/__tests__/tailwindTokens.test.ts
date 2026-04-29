import { describe, it, expect } from "vitest";
import config from "../../tailwind.config.js";

describe("tailwind.config.js theme.extend", () => {
  describe("colors — design token aliases", () => {
    const colorKeys = [
      "bg-page",
      "bg-surface",
      "bg-surface-2",
      "fg",
      "fg-muted",
      "fg-subtle",
      "border-soft",
      "border-strong",
      "status-done",
      "status-active",
      "status-queued",
      "status-failed",
      "status-info",
      "agent-architect",
      "agent-developer",
      "agent-qa",
      "agent-reviewer",
    ] as const;

    for (const key of colorKeys) {
      it(`colors["${key}"] is a var(--…) reference pointing to --${key}`, () => {
        const colors = config.theme.extend.colors as Record<string, string>;
        const value = colors[key];
        expect(value).toMatch(/^var\(--[a-z0-9-]+\)$/);
        expect(value).toBe(`var(--${key})`);
      });
    }
  });

  describe("fontFamily", () => {
    it('fontFamily.sans[0] is "Inter"', () => {
      const fontFamily = config.theme.extend.fontFamily as Record<
        string,
        string[]
      >;
      expect(fontFamily.sans[0]).toBe("Inter");
    });
  });

  describe("boxShadow — semantic aliases", () => {
    const shadowKeys = ["card", "popover", "modal"] as const;

    for (const key of shadowKeys) {
      it(`boxShadow["${key}"] is a var(--…) reference`, () => {
        const boxShadow = config.theme.extend.boxShadow as Record<
          string,
          string
        >;
        const value = boxShadow[key];
        expect(value).toMatch(/^var\(--[a-z0-9-]+\)$/);
      });
    }
  });
});
