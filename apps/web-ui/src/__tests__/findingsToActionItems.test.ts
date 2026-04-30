import { describe, it, expect } from "vitest";
import { findingsToActionItems } from "../utils/findingsToActionItems";
import type { Finding } from "../types/finding";

function finding(opts: Partial<Finding> & Pick<Finding, "id" | "severity">): Finding {
  return {
    run_id: "run-1",
    step_id: "developer",
    file_path: null,
    line: null,
    message: opts.id + " message",
    suggestion: null,
    triage: null,
    triaged_at: null,
    created_at: 0,
    ...opts,
  };
}

describe("findingsToActionItems", () => {
  it("maps three severities to three ActionItems in order", () => {
    const findings = [
      finding({ id: "f1", severity: "error" }),
      finding({ id: "f2", severity: "warning" }),
      finding({ id: "f3", severity: "info" }),
    ];
    const result = findingsToActionItems(findings);
    expect(result).toHaveLength(3);
    expect(result[0].kind).toBe("warning");
    expect(result[1].kind).toBe("followup");
    expect(result[2].kind).toBe("issue");
  });

  it("maps error severity to warning kind", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "error" })]);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("warning");
  });

  it("maps warning severity to followup kind", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "warning" })]);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("followup");
  });

  it("maps info severity to issue kind", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "info" })]);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("issue");
  });

  it("skips unknown severity", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "critical" })]);
    expect(result).toHaveLength(0);
  });

  it("filters out findings with non-null triage, keeps triage:null", () => {
    const findings = [
      finding({ id: "f1", severity: "error", triage: "fix" }),
      finding({ id: "f2", severity: "warning", triage: null }),
    ];
    const result = findingsToActionItems(findings);
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("f2");
  });

  it("filters out all triage variants: fix", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "error", triage: "fix" })]);
    expect(result).toHaveLength(0);
  });

  it("filters out all triage variants: tech-debt", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "error", triage: "tech-debt" })]);
    expect(result).toHaveLength(0);
  });

  it("filters out all triage variants: ignore", () => {
    const result = findingsToActionItems([finding({ id: "f1", severity: "error", triage: "ignore" })]);
    expect(result).toHaveLength(0);
  });

  it("maps suggestion to description", () => {
    const result = findingsToActionItems([
      finding({ id: "f1", severity: "error", suggestion: "do X" }),
    ]);
    expect(result[0].description).toBe("do X");
  });

  it("omits description when suggestion is null", () => {
    const result = findingsToActionItems([
      finding({ id: "f1", severity: "error", suggestion: null }),
    ]);
    expect(result[0].description).toBeUndefined();
  });

  it("maps message to title", () => {
    const result = findingsToActionItems([
      finding({ id: "f1", severity: "info", message: "some important message" }),
    ]);
    expect(result[0].title).toBe("some important message");
  });

  it("maps step_id to fromAgent", () => {
    const result = findingsToActionItems([
      finding({ id: "f1", severity: "warning", step_id: "qa" }),
    ]);
    expect(result[0].fromAgent).toBe("qa");
  });

  it("id flows through", () => {
    const result = findingsToActionItems([
      finding({ id: "abc-123", severity: "info" }),
    ]);
    expect(result[0].id).toBe("abc-123");
  });

  it("preserves order across 3 findings", () => {
    const findings = [
      finding({ id: "first", severity: "info" }),
      finding({ id: "second", severity: "error" }),
      finding({ id: "third", severity: "warning" }),
    ];
    const result = findingsToActionItems(findings);
    expect(result.map((r) => r.id)).toEqual(["first", "second", "third"]);
  });

  it("returns empty array for empty input", () => {
    expect(findingsToActionItems([])).toEqual([]);
  });
});
