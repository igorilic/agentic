import type { MentionParseError, MentionParseResult } from "./types";

/**
 * Parse a candidate `@mention` from a chat input.
 *
 * Format: `@<agent> <body...>`. Agent name is `[a-zA-Z0-9_-]+`. Body is
 * everything after the first whitespace.
 *
 * Pure function — no side effects, no IPC.
 */
export function parseMention(input: string): MentionParseResult {
  const trimmed = input.trim();
  if (!trimmed.startsWith("@")) {
    return { ok: false, error: { kind: "not_a_mention", input } };
  }
  // Match: @ followed by valid agent name characters, then whitespace, then body.
  // The `/s` (dotAll) flag lets `.` match newline characters so that
  // multi-line bodies (e.g., shift-enter pasted prompts) are accepted as
  // a single mention rather than being truncated at the first `\n`.
  const match = trimmed.match(/^@([a-zA-Z0-9_-]+)(\s+(.+))?$/s);
  if (!match) {
    return { ok: false, error: { kind: "missing_agent", input } };
  }
  const agent = match[1];
  const body = (match[3] ?? "").trim();
  if (!body) {
    return { ok: false, error: { kind: "missing_body", agent } };
  }
  return { ok: true, command: { agent, body } };
}

export function formatMentionParseError(err: MentionParseError): string {
  switch (err.kind) {
    case "not_a_mention":
      return `Not a mention: "${err.input}"`;
    case "missing_agent":
      return `Mention requires an agent name: ${err.input}`;
    case "missing_body":
      return `Mention requires a message body: @${err.agent} <body>`;
  }
}
