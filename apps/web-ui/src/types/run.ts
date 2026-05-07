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

/** Build the initial RunState — all steps pending, zero tokens. */
export function emptyRunState(
  agents: readonly string[],
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
 * Scan events for a RunStarted envelope and extract its agents list.
 * Returns the agents array if RunStarted is found AND the field is a
 * proper array (even if empty — that's authoritative).
 * Returns null if RunStarted is absent or its data lacks the agents field.
 */
function extractAgentsFromEvents(events: EventEnvelope[]): string[] | null {
  for (const env of events) {
    if (env.event.type === "RunStarted") {
      const data = (env.event.data ?? {}) as Record<string, unknown>;
      if (Array.isArray(data.agents)) {
        return data.agents as string[];
      }
      // RunStarted found but no agents field — signal caller to fall back.
      return null;
    }
  }
  // No RunStarted found — fall back.
  return null;
}

/**
 * Derive a fresh RunState from a stream of envelopes for a single run.
 * Idempotent over re-application: replaying the same events produces
 * the same state.
 *
 * Agent list resolution (Issue 1):
 * 1. If a RunStarted event with an agents array is present, use it
 *    (even if empty — that's authoritative: the run has no steps).
 * 2. If RunStarted is present but lacks agents (legacy event), fall back
 *    to the `agents` parameter.
 * 3. If no RunStarted at all, fall back to the `agents` parameter.
 *
 * Multi-run safety (GH #66):
 * If `activeRunId` is provided, only envelopes whose `run_id` matches are
 * processed. This prevents late-arriving or broadcast envelopes from other
 * runs from corrupting the state. Pass `undefined` to retain legacy behavior
 * (all events processed — useful when the caller already manages a
 * single-run buffer).
 */
export function deriveRunState(
  events: EventEnvelope[],
  activeRunId?: string,
  agents: readonly string[] = [],
): RunState {
  const filtered = activeRunId
    ? events.filter((e) => e.run_id === activeRunId)
    : events;
  const eventAgents = extractAgentsFromEvents(filtered);
  // eventAgents is null when: no RunStarted, or RunStarted lacks agents field.
  // In both cases fall back to the caller-supplied agents list.
  const resolvedAgents: readonly string[] = eventAgents !== null ? eventAgents : agents;

  const state = emptyRunState(resolvedAgents);
  // Index by agent name for O(1) lookups.
  const byAgent = new Map<string, StepInfo>(
    state.steps.map((s) => [s.agent, s]),
  );

  // Track agent-name per step_id (StepStarted carries it; StepComplete carries
  // step_id but not agent). For step_id-keyed lookups in StepComplete.
  const stepIdToAgent = new Map<string, string>();

  for (const env of filtered) {
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
