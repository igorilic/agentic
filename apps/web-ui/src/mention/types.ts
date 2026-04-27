export type MentionCommand = {
  agent: string;
  body: string;
};

export type MentionParseResult =
  | { ok: true; command: MentionCommand }
  | { ok: false; error: MentionParseError };

export type MentionParseError =
  | { kind: "not_a_mention"; input: string }
  | { kind: "missing_agent"; input: string }
  | { kind: "missing_body"; agent: string };
