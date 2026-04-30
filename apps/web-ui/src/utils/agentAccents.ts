// Per-agent accent colors sourced from design handoff agents.jsx lines 20-32.
// Inline styles bypass the W.0.1 token system — these per-agent accent values
// were not included in the original token catalogue.

export type AgentAccent = { bg: string; fg: string };

export const AGENT_ACCENTS: Record<string, AgentAccent> = {
  architect:  { bg: "rgb(2 132 199 / 0.10)",  fg: "rgb(2 87 130)" },
  developer:  { bg: "rgb(132 204 22 / 0.15)", fg: "rgb(63 98 18)" },
  qa:         { bg: "rgb(217 70 239 / 0.10)", fg: "rgb(134 25 143)" },
  reviewer:   { bg: "rgb(245 158 11 / 0.15)", fg: "rgb(120 53 15)" },
  researcher: { bg: "rgb(99 102 241 / 0.10)", fg: "rgb(55 48 163)" },
  security:   { bg: "rgb(220 38 38 / 0.10)",  fg: "rgb(153 27 27)" },
  perf:       { bg: "rgb(20 184 166 / 0.12)", fg: "rgb(15 118 110)" },
  docs:       { bg: "rgb(0 0 0 / 0.06)",      fg: "rgb(63 63 70)" },
  designer:   { bg: "rgb(236 72 153 / 0.10)", fg: "rgb(157 23 77)" },
  db:         { bg: "rgb(124 58 237 / 0.10)", fg: "rgb(91 33 182)" },
  devops:     { bg: "rgb(8 145 178 / 0.10)",  fg: "rgb(14 116 144)" },
  a11y:       { bg: "rgb(34 197 94 / 0.12)",  fg: "rgb(22 101 52)" },
};

export const FALLBACK_ACCENT: AgentAccent = AGENT_ACCENTS.docs;

export function getAgentAccent(agent: string): AgentAccent {
  // Map tdd-developer → developer accent
  if (agent === "tdd-developer") return AGENT_ACCENTS.developer ?? FALLBACK_ACCENT;
  return AGENT_ACCENTS[agent] ?? FALLBACK_ACCENT;
}
