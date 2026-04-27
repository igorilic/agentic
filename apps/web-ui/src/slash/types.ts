/**
 * Discriminated union of supported slash commands. Each variant carries
 * its parsed args. Add new variants as new commands land in later steps.
 */
export type SlashCommand =
  | { kind: "plan"; ticket: string }
  | { kind: "status"; runId: string | null }
  | { kind: "cancel"; runId: string };

/**
 * Result of parsing a candidate slash command. Either a typed command or
 * a structured validation error. The parser does NOT throw.
 */
export type SlashParseResult =
  | { ok: true; command: SlashCommand }
  | { ok: false; error: SlashParseError };

export type SlashParseError =
  | { kind: "not_a_slash_command"; input: string }
  | { kind: "unknown_command"; cmd: string }
  | { kind: "missing_argument"; cmd: string; argName: string }
  | { kind: "extra_argument"; cmd: string; surplus: string };
