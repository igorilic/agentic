import { describe, it, expect } from "vitest";
import { AGENT_ACCENTS, getAgentAccent, FALLBACK_ACCENT } from "../utils/agentAccents";

describe("agentAccents", () => {
  describe("AGENT_ACCENTS map — all 12 entries present", () => {
    const expectedIds = [
      "architect", "developer", "qa", "reviewer", "researcher",
      "security", "perf", "docs", "designer", "db", "devops", "a11y",
    ];

    for (const id of expectedIds) {
      it(`has entry for "${id}"`, () => {
        expect(AGENT_ACCENTS[id]).toBeDefined();
        expect(AGENT_ACCENTS[id].bg).toBeTruthy();
        expect(AGENT_ACCENTS[id].fg).toBeTruthy();
      });
    }

    it("has exactly 12 entries", () => {
      expect(Object.keys(AGENT_ACCENTS)).toHaveLength(12);
    });
  });

  describe("getAgentAccent — known agents", () => {
    it("architect fg is 'rgb(2 87 130)'", () => {
      expect(getAgentAccent("architect").fg).toBe("rgb(2 87 130)");
    });

    it("architect bg contains 0.10", () => {
      expect(getAgentAccent("architect").bg).toContain("0.10");
    });

    it("developer fg is 'rgb(63 98 18)'", () => {
      expect(getAgentAccent("developer").fg).toBe("rgb(63 98 18)");
    });
  });

  describe("getAgentAccent — tdd-developer alias", () => {
    it("tdd-developer returns same accent as developer", () => {
      const tddAccent = getAgentAccent("tdd-developer");
      const devAccent = getAgentAccent("developer");
      expect(tddAccent).toEqual(devAccent);
    });

    it("tdd-developer fg matches developer fg", () => {
      expect(getAgentAccent("tdd-developer").fg).toBe(getAgentAccent("developer").fg);
    });

    it("tdd-developer bg matches developer bg", () => {
      expect(getAgentAccent("tdd-developer").bg).toBe(getAgentAccent("developer").bg);
    });
  });

  describe("getAgentAccent — unknown agent fallback", () => {
    it("unknown agent returns FALLBACK_ACCENT", () => {
      expect(getAgentAccent("unknown")).toEqual(FALLBACK_ACCENT);
    });

    it("empty string returns FALLBACK_ACCENT", () => {
      expect(getAgentAccent("")).toEqual(FALLBACK_ACCENT);
    });

    it("FALLBACK_ACCENT equals docs accent", () => {
      expect(FALLBACK_ACCENT).toEqual(AGENT_ACCENTS.docs);
    });
  });
});
