import type { SlashParseError, SlashParseResult } from "./types";

/**
 * Parse a candidate slash command from a chat input. Pure function — no
 * side effects, no IPC.
 *
 * Recognized commands:
 *   /plan <ticket>      — start the default pipeline against a ticket
 *   /status [run_id]    — show status of the current/given run
 *   /cancel <run_id>    — cancel an in-flight run
 *
 * Inputs that don't start with `/` return `{ ok: false, error: not_a_slash_command }`.
 */
export function parseSlashCommand(input: string): SlashParseResult {
  const trimmed = input.trim();
  if (!trimmed.startsWith("/")) {
    return { ok: false, error: { kind: "not_a_slash_command", input } };
  }

  const parts = trimmed.slice(1).split(/\s+/);
  const cmd = parts[0] ?? "";
  const args = parts.slice(1);

  switch (cmd) {
    case "plan": {
      if (args.length === 0) {
        return { ok: false, error: { kind: "missing_argument", cmd: "plan", argName: "ticket" } };
      }
      const ticket = args.join(" ").trim();
      return { ok: true, command: { kind: "plan", ticket } };
    }
    case "status": {
      if (args.length > 1) {
        return { ok: false, error: { kind: "extra_argument", cmd: "status", surplus: args.slice(1).join(" ") } };
      }
      const runId = args[0] ?? null;
      return { ok: true, command: { kind: "status", runId } };
    }
    case "cancel": {
      if (args.length === 0) {
        return { ok: false, error: { kind: "missing_argument", cmd: "cancel", argName: "run_id" } };
      }
      if (args.length > 1) {
        return { ok: false, error: { kind: "extra_argument", cmd: "cancel", surplus: args.slice(1).join(" ") } };
      }
      return { ok: true, command: { kind: "cancel", runId: args[0] } };
    }
    default: {
      return { ok: false, error: { kind: "unknown_command", cmd } };
    }
  }
}

/**
 * Render a `SlashParseError` as a single-line user-facing string.
 * Used by the dispatcher / chat integration to display error feedback.
 */
export function formatSlashParseError(err: SlashParseError): string {
  switch (err.kind) {
    case "not_a_slash_command":
      return `Not a slash command: "${err.input}"`;
    case "unknown_command":
      return `Unknown command: /${err.cmd}`;
    case "missing_argument":
      return `Missing argument for /${err.cmd}: <${err.argName}>`;
    case "extra_argument":
      return `Extra argument for /${err.cmd}: ${err.surplus}`;
  }
}
