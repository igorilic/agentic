export type BackendKind = "claude-code" | "copilot-cli";

export const ALLOWED_BACKENDS: readonly BackendKind[] = [
  "claude-code",
  "copilot-cli",
] as const;

/**
 * Discriminated union of supported slash commands. Each variant carries
 * its parsed args. Add new variants as new commands land in later steps.
 */
export type SlashCommand =
  | { kind: "plan"; ticket: string; backend?: BackendKind }
  | { kind: "status"; runId: string | null }
  | { kind: "cancel"; runId: string }
  | { kind: "help" };

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
  | { kind: "extra_argument"; cmd: string; surplus: string }
  | { kind: "invalid_backend"; given: string };
