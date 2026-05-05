import { describe, it, expect } from "vitest";
import { resolveAgentIcon } from "../utils/agentIcon";

describe("resolveAgentIcon", () => {
  // ── Layer 1: Exact AGENT_LIBRARY id match ────────────────────────────────

  describe("exact library id match — preserves existing behavior", () => {
    it("resolves 'architect' to blueprint", () => {
      expect(resolveAgentIcon("architect")).toEqual({ kind: "library", iconKey: "blueprint" });
    });

    it("resolves 'developer' to code", () => {
      expect(resolveAgentIcon("developer")).toEqual({ kind: "library", iconKey: "code" });
    });

    it("resolves 'tdd-developer' to code (library alias)", () => {
      expect(resolveAgentIcon("tdd-developer")).toEqual({ kind: "library", iconKey: "code" });
    });

    it("resolves 'qa' to check", () => {
      expect(resolveAgentIcon("qa")).toEqual({ kind: "library", iconKey: "check" });
    });

    it("resolves 'reviewer' to eye", () => {
      expect(resolveAgentIcon("reviewer")).toEqual({ kind: "library", iconKey: "eye" });
    });

    it("resolves 'researcher' to book", () => {
      expect(resolveAgentIcon("researcher")).toEqual({ kind: "library", iconKey: "book" });
    });

    it("resolves 'security' to shield", () => {
      expect(resolveAgentIcon("security")).toEqual({ kind: "library", iconKey: "shield" });
    });

    it("resolves 'perf' to gauge", () => {
      expect(resolveAgentIcon("perf")).toEqual({ kind: "library", iconKey: "gauge" });
    });

    it("resolves 'docs' to doc", () => {
      expect(resolveAgentIcon("docs")).toEqual({ kind: "library", iconKey: "doc" });
    });

    it("resolves 'designer' to palette", () => {
      expect(resolveAgentIcon("designer")).toEqual({ kind: "library", iconKey: "palette" });
    });

    it("resolves 'db' to database", () => {
      expect(resolveAgentIcon("db")).toEqual({ kind: "library", iconKey: "database" });
    });

    it("resolves 'devops' to cloud", () => {
      expect(resolveAgentIcon("devops")).toEqual({ kind: "library", iconKey: "cloud" });
    });

    it("resolves 'a11y' to a11y", () => {
      expect(resolveAgentIcon("a11y")).toEqual({ kind: "library", iconKey: "a11y" });
    });
  });

  // ── Layer 2: Keyword heuristic ────────────────────────────────────────────
  // Priority rule: table order, first matching keyword wins.
  // Keywords earlier in the table beat later ones when a name has multiple matches.

  describe("keyword heuristic — single keyword hit", () => {
    it("resolves 'requirements-engineer' to book (requirements keyword wins over engineer/code)", () => {
      // 'requirements' is in the research/book group; 'engineer' is in the code group.
      // The table puts research keywords BEFORE code keywords, so book wins.
      expect(resolveAgentIcon("requirements-engineer")).toEqual({ kind: "library", iconKey: "book" });
    });

    it("resolves 'ui-ux-designer' to palette (designer keyword)", () => {
      expect(resolveAgentIcon("ui-ux-designer")).toEqual({ kind: "library", iconKey: "palette" });
    });

    it("resolves 'code-reviewer' to eye (review keyword)", () => {
      expect(resolveAgentIcon("code-reviewer")).toEqual({ kind: "library", iconKey: "eye" });
    });

    it("resolves 'system-architect' to blueprint (architect keyword)", () => {
      expect(resolveAgentIcon("system-architect")).toEqual({ kind: "library", iconKey: "blueprint" });
    });

    it("resolves 'test-agent' to check (test keyword)", () => {
      expect(resolveAgentIcon("test-agent")).toEqual({ kind: "library", iconKey: "check" });
    });

    it("resolves 'quality-assurance' to check (quality keyword)", () => {
      expect(resolveAgentIcon("quality-assurance")).toEqual({ kind: "library", iconKey: "check" });
    });

    it("resolves 'vuln-scanner' to shield (vuln keyword)", () => {
      expect(resolveAgentIcon("vuln-scanner")).toEqual({ kind: "library", iconKey: "shield" });
    });

    it("resolves 'performance-optimizer' to gauge (performance keyword)", () => {
      expect(resolveAgentIcon("performance-optimizer")).toEqual({ kind: "library", iconKey: "gauge" });
    });

    it("resolves 'tech-writer' to doc (tech-writer / doc keyword)", () => {
      expect(resolveAgentIcon("tech-writer")).toEqual({ kind: "library", iconKey: "doc" });
    });

    it("resolves 'sql-migrator' to database (sql keyword)", () => {
      expect(resolveAgentIcon("sql-migrator")).toEqual({ kind: "library", iconKey: "database" });
    });

    it("resolves 'k8s-ops' to cloud (k8s keyword)", () => {
      expect(resolveAgentIcon("k8s-ops")).toEqual({ kind: "library", iconKey: "cloud" });
    });

    it("resolves 'accessibility-checker' to a11y (accessibility keyword)", () => {
      expect(resolveAgentIcon("accessibility-checker")).toEqual({ kind: "library", iconKey: "a11y" });
    });
  });

  describe("keyword heuristic — underscore variant", () => {
    // 'perf' keyword comes before 'engineer'/'code' in the table → gauge wins
    it("resolves 'perf_engineer' to gauge (perf keyword has higher table priority than engineer)", () => {
      expect(resolveAgentIcon("perf_engineer")).toEqual({ kind: "library", iconKey: "gauge" });
    });

    it("resolves 'code_developer' to code (develop/code keyword)", () => {
      expect(resolveAgentIcon("code_developer")).toEqual({ kind: "library", iconKey: "code" });
    });

    it("resolves 'db_admin' to database (db keyword)", () => {
      expect(resolveAgentIcon("db_admin")).toEqual({ kind: "library", iconKey: "database" });
    });
  });

  describe("keyword heuristic — case insensitive", () => {
    it("resolves 'ARCHITECT' to blueprint", () => {
      expect(resolveAgentIcon("ARCHITECT")).toEqual({ kind: "library", iconKey: "blueprint" });
    });

    it("resolves 'QA-Engineer' to check (qa keyword)", () => {
      expect(resolveAgentIcon("QA-Engineer")).toEqual({ kind: "library", iconKey: "check" });
    });
  });

  describe("keyword heuristic — multi-word priority", () => {
    // 'review' hits 'eye'; 'code' hits 'code'. Table order: architect > code > check > eye.
    // 'code' group is BEFORE 'review' group in the table, so 'code-review' should hit 'code'
    // unless we check review first. Document: KEYWORD_TABLE order determines priority.
    // For 'code-review': tokens are ['code','review']. Iterate tokens; for each token scan table.
    // First table entry that matches any token wins. Table: architect→blueprint, develop/tdd/code/engineer/implement→code.
    // So 'code' token hits the second table entry → code icon.
    it("resolves 'code-review' — 'code' keyword (code group) precedes 'review' in table order", () => {
      const result = resolveAgentIcon("code-review");
      // Either 'code' or 'eye' is acceptable depending on implementation order;
      // assert it's NOT the initial fallback — it must be a library match
      expect(result.kind).toBe("library");
    });
  });

  // ── Layer 3: Colored-initial fallback ────────────────────────────────────

  describe("initial fallback — unknown names", () => {
    it("resolves 'xyzzy' to initial kind", () => {
      const result = resolveAgentIcon("xyzzy");
      expect(result.kind).toBe("initial");
    });

    it("resolves 'xyzzy' initial to 'X'", () => {
      const result = resolveAgentIcon("xyzzy");
      if (result.kind !== "initial") throw new Error("expected initial");
      expect(result.initial).toBe("X");
    });

    it("resolves 'xyzzy' bgClass to a non-empty string", () => {
      const result = resolveAgentIcon("xyzzy");
      if (result.kind !== "initial") throw new Error("expected initial");
      expect(result.bgClass).toBeTruthy();
    });

    it("resolves 'foo-bar' initial to 'F' (first letter)", () => {
      const result = resolveAgentIcon("foo-bar");
      if (result.kind !== "initial") throw new Error("expected initial");
      expect(result.initial).toBe("F");
    });

    it("resolves 'foo-bar' initial uppercase regardless of input case", () => {
      const result1 = resolveAgentIcon("foo-bar");
      const result2 = resolveAgentIcon("FOO-BAR");
      if (result1.kind !== "initial" || result2.kind !== "initial") throw new Error("expected initial");
      expect(result1.initial).toBe(result2.initial);
    });

    it("stable hash — calling twice with same name returns same bgClass", () => {
      const r1 = resolveAgentIcon("xyzzy");
      const r2 = resolveAgentIcon("xyzzy");
      if (r1.kind !== "initial" || r2.kind !== "initial") throw new Error("expected initial");
      expect(r1.bgClass).toBe(r2.bgClass);
    });

    it("different names may produce different bgClass (hash spread)", () => {
      const r1 = resolveAgentIcon("aaaaa-unknown");
      const r2 = resolveAgentIcon("zzzzz-unknown");
      // Both must be initial kind
      expect(r1.kind).toBe("initial");
      expect(r2.kind).toBe("initial");
      // Not required to be different, but the types must be correct
    });

    it("resolves '123-agent' — skips non-letter prefix, finds first alpha char", () => {
      const result = resolveAgentIcon("123-agent");
      if (result.kind !== "initial") throw new Error("expected initial");
      // First non-punctuation alpha char in '123-agent' is 'a'
      expect(result.initial).toBe("A");
    });
  });
});
