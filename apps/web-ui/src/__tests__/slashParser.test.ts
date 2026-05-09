import { describe, expect, it } from "vitest";
import { parseSlashCommand, formatSlashParseError } from "../slash/parser";

describe("parseSlashCommand", () => {
  it("rejects input that doesn't start with /", () => {
    const r = parseSlashCommand("hello world");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("not_a_slash_command");
  });

  it("rejects unknown command", () => {
    const r = parseSlashCommand("/foo bar");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("unknown_command");
      if (r.error.kind === "unknown_command") expect(r.error.cmd).toBe("foo");
    }
  });

  it("/plan with no args returns missing_argument", () => {
    const r = parseSlashCommand("/plan");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("missing_argument");
      if (r.error.kind === "missing_argument") {
        expect(r.error.cmd).toBe("plan");
        expect(r.error.argName).toBe("ticket");
      }
    }
  });

  it("/plan with ticket arg returns plan command", () => {
    const r = parseSlashCommand("/plan #42");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.command).toEqual({ kind: "plan", ticket: "#42" });
  });

  it("/plan joins multi-word ticket arg", () => {
    const r = parseSlashCommand("/plan add hello world test");
    expect(r.ok).toBe(true);
    if (r.ok && r.command.kind === "plan") {
      expect(r.command.ticket).toBe("add hello world test");
      expect(r.command.backend).toBeUndefined();
    }
  });

  it("/plan --backend=copilot-cli passes backend through and strips the flag from ticket", () => {
    const r = parseSlashCommand("/plan --backend=copilot-cli fix the auth race");
    expect(r.ok).toBe(true);
    if (r.ok && r.command.kind === "plan") {
      expect(r.command.backend).toBe("copilot-cli");
      expect(r.command.ticket).toBe("fix the auth race");
    }
  });

  it("/plan --backend=claude-code is also accepted explicitly", () => {
    const r = parseSlashCommand("/plan --backend=claude-code #42");
    expect(r.ok).toBe(true);
    if (r.ok && r.command.kind === "plan") {
      expect(r.command.backend).toBe("claude-code");
      expect(r.command.ticket).toBe("#42");
    }
  });

  it("/plan --backend=foo returns invalid_backend error", () => {
    const r = parseSlashCommand("/plan --backend=foo #42");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("invalid_backend");
      if (r.error.kind === "invalid_backend") {
        expect(r.error.given).toBe("foo");
      }
    }
  });

  it("/plan --backend with no = and ticket fails parse (not silently treated as ticket)", () => {
    // Conservative: a malformed flag is a user error, not part of the
    // ticket text. Forces them to re-type rather than silently
    // sending "--backend" as part of the ticket body.
    const r = parseSlashCommand("/plan --backend foo #42");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("invalid_backend");
    }
  });

  it("/plan with --backend at end and no ticket fails missing_argument", () => {
    const r = parseSlashCommand("/plan --backend=claude-code");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("missing_argument");
    }
  });

  it("/status with no args returns null runId", () => {
    const r = parseSlashCommand("/status");
    expect(r.ok).toBe(true);
    if (r.ok && r.command.kind === "status") expect(r.command.runId).toBeNull();
  });

  it("/status with one arg returns runId", () => {
    const r = parseSlashCommand("/status run-abc");
    expect(r.ok).toBe(true);
    if (r.ok && r.command.kind === "status") expect(r.command.runId).toBe("run-abc");
  });

  it("/status with extra args returns extra_argument", () => {
    const r = parseSlashCommand("/status run-abc trailing");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("extra_argument");
  });

  it("/cancel without args returns missing_argument", () => {
    const r = parseSlashCommand("/cancel");
    expect(r.ok).toBe(false);
  });

  it("/cancel with run_id returns cancel command", () => {
    const r = parseSlashCommand("/cancel run-xyz");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.command).toEqual({ kind: "cancel", runId: "run-xyz" });
  });

  it("trims leading whitespace", () => {
    const r = parseSlashCommand("   /plan ticket-1");
    expect(r.ok).toBe(true);
  });

  // C1 — uppercase command name is accepted (case-insensitive matching)
  it("C1 — /PLAN upper-case is treated identically to /plan", () => {
    const r = parseSlashCommand("/PLAN ticket text here");
    expect(r.ok).toBe(true);
    if (r.ok && r.command.kind === "plan") {
      expect(r.command.ticket).toBe("ticket text here");
    }
  });

  // C2 — mixed-case command name is accepted
  it("C2 — /Status mixed-case is treated identically to /status", () => {
    const r = parseSlashCommand("/Status");
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.command.kind).toBe("status");
      if (r.command.kind === "status") expect(r.command.runId).toBeNull();
    }
  });

  // C3 — original casing of unknown command is preserved in the error
  it("C3 — unknown command error preserves the original casing of the command", () => {
    const r = parseSlashCommand("/Foo");
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.error.kind).toBe("unknown_command");
      if (r.error.kind === "unknown_command") {
        // Should echo "Foo" (original), NOT "foo" (lowercased)
        expect(r.error.cmd).toBe("Foo");
      }
    }
  });

  it("formats error messages user-friendly", () => {
    expect(
      formatSlashParseError({ kind: "unknown_command", cmd: "foo" }),
    ).toContain("/foo");
    expect(
      formatSlashParseError({ kind: "missing_argument", cmd: "plan", argName: "ticket" }),
    ).toContain("ticket");
    expect(
      formatSlashParseError({ kind: "not_a_slash_command", input: "hello" }),
    ).toContain("hello");
    expect(
      formatSlashParseError({ kind: "extra_argument", cmd: "status", surplus: "trailing" }),
    ).toContain("trailing");
  });

  // /help tests
  it("parser_recognizes_help_with_no_args — /help returns { kind: 'help' }", () => {
    const r = parseSlashCommand("/help");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.command.kind).toBe("help");
  });

  it("parser_recognizes_help_case_insensitive — /HELP returns { kind: 'help' }", () => {
    const r = parseSlashCommand("/HELP");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.command.kind).toBe("help");
  });

  it("parser_accepts_help_with_trailing_args — /help foo silently ignores args", () => {
    const r = parseSlashCommand("/help foo");
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.command.kind).toBe("help");
  });

  it("parser_rejects_help_command_substring — /helper is unknown, not /help", () => {
    const r = parseSlashCommand("/helper");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error.kind).toBe("unknown_command");
  });
});
