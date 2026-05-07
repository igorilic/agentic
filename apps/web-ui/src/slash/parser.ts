import {
  ALLOWED_BACKENDS,
  type BackendKind,
  type SlashParseError,
  type SlashParseResult,
} from "./types";

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
  const rawCmd = parts[0] ?? "";
  const cmd = rawCmd.toLowerCase();
  const args = parts.slice(1);

  switch (cmd) {
    case "plan": {
      // Strip leading flags before joining the rest as ticket text.
      // Currently only `--backend=<value>` is recognised; bare `--backend`
      // (no `=`) and unknown values are explicit user errors. Unknown
      // `--…` flags stop flag-parsing — they get treated as ticket text
      // (so e.g. `/plan --foo bar` sends "--foo bar" to the agent).
      let backend: BackendKind | undefined;
      let i = 0;
      while (i < args.length && args[i].startsWith("--")) {
        const flag = args[i];
        if (flag.startsWith("--backend=")) {
          const given = flag.slice("--backend=".length);
          if (!ALLOWED_BACKENDS.includes(given as BackendKind)) {
            return { ok: false, error: { kind: "invalid_backend", given } };
          }
          backend = given as BackendKind;
        } else if (flag === "--backend") {
          return { ok: false, error: { kind: "invalid_backend", given: "" } };
        } else {
          break;
        }
        i++;
      }
      const rest = args.slice(i);
      if (rest.length === 0) {
        return { ok: false, error: { kind: "missing_argument", cmd: "plan", argName: "ticket" } };
      }
      const ticket = rest.join(" ").trim();
      return { ok: true, command: { kind: "plan", ticket, backend } };
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
      return { ok: false, error: { kind: "unknown_command", cmd: rawCmd } };
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
    case "invalid_backend":
      return err.given.length === 0
        ? `Missing value for --backend (use --backend=claude-code or --backend=copilot-cli)`
        : `Invalid backend "${err.given}" (allowed: ${ALLOWED_BACKENDS.join(", ")})`;
  }
}
