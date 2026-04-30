/**
 * Maps a known-agent id to its Tailwind text-color utility (`text-agent-*`).
 * Unknown agents fall back to `text-fg` via {@link agentColorClass}.
 *
 * The literal class strings live here so Tailwind's JIT scanner picks
 * them up. Don't refactor away the literals into template strings.
 */
export const AGENT_COLOR_CLASS: Readonly<Record<string, string>> = {
  architect: "text-agent-architect",
  developer: "text-agent-developer",
  qa: "text-agent-qa",
  reviewer: "text-agent-reviewer",
};

/**
 * Resolve the per-agent text-color utility, falling back to `text-fg`
 * for unknown agents.
 */
export function agentColorClass(agent: string): string {
  return AGENT_COLOR_CLASS[agent] ?? "text-fg";
}
