import { describe, expect, it } from "vitest";
import { parseMention, formatMentionParseError } from "../mention/parser";

describe("parseMention", () => {
  it("rejects input that does not start with @", () => {
    const r = parseMention("hello world");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("not_a_mention");
  });

  it("rejects bare @ with no agent name", () => {
    const r = parseMention("@");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("missing_agent");
  });

  it("rejects @agent with no body", () => {
    const r = parseMention("@architect");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("missing_body");
      if (r.error.kind === "missing_body") expect(r.error.agent).toBe("architect");
    }
  });

  it("parses @architect rest of message", () => {
    const r = parseMention("@architect rest of message");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.command.agent).toBe("architect");
      expect(r.command.body).toBe("rest of message");
    }
  });

  it("trims extra whitespace from body", () => {
    const r = parseMention("@architect    extra   whitespace");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.command.agent).toBe("architect");
      expect(r.command.body).toBe("extra   whitespace");
    }
  });

  it("allows agent names with underscores and hyphens and digits", () => {
    const r = parseMention("@arch_name-1 body");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.command.agent).toBe("arch_name-1");
      expect(r.command.body).toBe("body");
    }
  });

  it("rejects agent names containing @ (bad chars)", () => {
    const r = parseMention("@bad@chars body");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("missing_agent");
  });

  it("rejects whitespace-only body after agent name", () => {
    const r = parseMention("@architect   ");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("missing_body");
  });

  it("trims leading whitespace from input before parsing", () => {
    const r = parseMention("  @architect hello");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.command.agent).toBe("architect");
      expect(r.command.body).toBe("hello");
    }
  });

  it("preserves newlines in the body via the /s (dotAll) regex flag", () => {
    const r = parseMention("@architect line one\nline two\nline three");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.command.agent).toBe("architect");
      expect(r.command.body).toBe("line one\nline two\nline three");
    }
  });
});

describe("formatMentionParseError", () => {
  it("formats not_a_mention error", () => {
    const msg = formatMentionParseError({ kind: "not_a_mention", input: "hello" });
    expect(msg).toContain("hello");
  });

  it("formats missing_agent error", () => {
    const msg = formatMentionParseError({ kind: "missing_agent", input: "@" });
    expect(msg).toContain("agent");
  });

  it("formats missing_body error", () => {
    const msg = formatMentionParseError({ kind: "missing_body", agent: "architect" });
    expect(msg).toContain("architect");
    expect(msg).toContain("body");
  });
});
