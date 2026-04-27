import type { EventEnvelope } from "./event";

export type StepStatus =
  | "pending"
  | "running"
  | "passed"
  | "failed"
  | "needs_triage"
  | "skipped";

export type StepInfo = {
  agent: string;
  status: StepStatus;
  /** Tokens used by this step. 0 if not yet observed. */
  tokens: number;
  /** Cost USD; null if pricing unavailable. */
  costUsd: number | null;
  /** Duration in ms; 0 if not yet finished. */
  durationMs: number;
  /** Optional summary text from StepComplete. */
  summary: string | null;
};

export type RunState = {
  steps: StepInfo[];
  /** Sum of `step.tokens` across all steps. */
  totalTokens: number;
  /** Sum of `step.costUsd` for steps where it's non-null. */
  totalCostUsd: number;
};

/** Default agents for the standard 4-step pipeline. */
export const DEFAULT_AGENTS: readonly string[] = [
  "architect",
  "tdd-developer",
  "qa",
  "reviewer",
];

/** Build the initial RunState — all steps pending, zero tokens. */
export function emptyRunState(
  agents: readonly string[] = DEFAULT_AGENTS,
): RunState {
  return {
    steps: agents.map((agent) => ({
      agent,
      status: "pending",
      tokens: 0,
      costUsd: null,
      durationMs: 0,
      summary: null,
    })),
    totalTokens: 0,
    totalCostUsd: 0,
  };
}

/**
 * Derive a fresh RunState from a stream of envelopes for a single run.
 * Idempotent over re-application: replaying the same events produces
 * the same state.
 */
export function deriveRunState(
  events: EventEnvelope[],
  agents: readonly string[] = DEFAULT_AGENTS,
): RunState {
  const state = emptyRunState(agents);
  // Index by agent name for O(1) lookups.
  const byAgent = new Map<string, StepInfo>(
    state.steps.map((s) => [s.agent, s]),
  );

  // Track agent-name per step_id (StepStarted carries it; StepComplete carries
  // step_id but not agent). For step_id-keyed lookups in StepComplete.
  const stepIdToAgent = new Map<string, string>();

  for (const env of events) {
    const data = (env.event.data ?? {}) as Record<string, unknown>;
    switch (env.event.type) {
      case "StepStarted": {
        const agent = (data.agent as string | undefined) ?? "";
        const step = byAgent.get(agent);
        if (step) {
          step.status = "running";
          if (env.step_id) stepIdToAgent.set(env.step_id, agent);
        }
        break;
      }
      case "StepComplete": {
        const agent = env.step_id
          ? stepIdToAgent.get(env.step_id)
          : undefined;
        const step = agent ? byAgent.get(agent) : undefined;
        if (step) {
          const status = data.status as StepStatus | undefined;
          if (status) step.status = status;
          step.durationMs = (data.duration_ms as number | undefined) ?? 0;
          step.summary = (data.summary as string | undefined) ?? null;
          const usage = data.token_usage as
            | Record<string, number>
            | undefined;
          if (usage) {
            step.tokens =
              (usage.input_tokens ?? 0) +
              (usage.output_tokens ?? 0) +
              (usage.cache_read_input_tokens ?? 0) +
              (usage.cache_creation_input_tokens ?? 0);
          }
          step.costUsd =
            (data.cost_usd as number | null | undefined) ?? null;
        }
        break;
      }
      // Other event types don't directly mutate step state.
    }
  }

  state.totalTokens = state.steps.reduce((sum, s) => sum + s.tokens, 0);
  state.totalCostUsd = state.steps.reduce(
    (sum, s) => sum + (s.costUsd ?? 0),
    0,
  );

  return state;
}
