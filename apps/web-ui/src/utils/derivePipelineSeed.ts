import type { RunState } from "../types/run";
import { DEFAULT_AGENTS } from "../types/run";

/**
 * Derive the initial pipeline agent list from a RunState.
 *
 * - If `runState.steps` is non-empty, use those agent ids in order.
 * - Otherwise fall back to DEFAULT_AGENTS.
 *
 * Returns a new array every call (never the original reference).
 * This is called only on `activeRunId` change, not on every runState tick,
 * so user edits made between seedings are preserved.
 */
export function derivePipelineSeed(runState: RunState): string[] {
  return runState.steps.length > 0
    ? runState.steps.map((s) => s.agent)
    : [...DEFAULT_AGENTS];
}
