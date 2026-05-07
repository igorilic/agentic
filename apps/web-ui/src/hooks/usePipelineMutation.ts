import { useEffect, useState } from "react";
import type { RunState } from "../types/run";
import { reorderArray, insertAt } from "../utils/arrayMove";

export type UsePipelineMutationResult = {
  pipelineAgents: string[];
  pipelineSkipped: ReadonlySet<string>;
  onReorder: (from: number, to: number) => void;
  onInsert: (at: number, id: string) => void;
  onRemove: (at: number) => void;
  onSkip: (at: number) => void;
};

/**
 * Manages the local-only mutable pipeline state (spec §6.8.3).
 *
 * When `externalAgents` + `setExternalAgents` are provided (from
 * `usePipelinePersistence`), they are used as the canonical state. Otherwise
 * an internal `useState` is used (backward-compatible for tests / contexts
 * without persistence).
 *
 * The `runState` parameter is retained in the signature for caller
 * compatibility but is intentionally not read — pipeline display from run
 * steps is handled at the App layer. See I.7 TD1 for context.
 */
export function usePipelineMutation(
  _runState: RunState,
  activeRunId: string | undefined,
  externalAgents?: string[],
  setExternalAgents?: (next: string[]) => void,
): UsePipelineMutationResult {
  // Internal fallback state — only used when no external state is provided.
  const [internalAgents, setInternalAgents] = useState<string[]>(() => []);
  const [pipelineSkipped, setPipelineSkipped] = useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );

  const pipelineAgents = externalAgents ?? internalAgents;
  const setPipelineAgents = setExternalAgents ?? setInternalAgents;

  // Reset skip-set when a new run starts. Do NOT overwrite the configured
  // pipeline (externalAgents / internalAgents) with run-derived agents — the
  // user's configured pipeline is the source of truth and must survive run
  // start/stop. Overwriting it was the root cause of the "pipeline agents
  // disappear after run" symptom (I.7 fix-loop TD1).
  useEffect(() => {
    if (!activeRunId) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional: reset skip-set when a new run starts; this is a "clear on dep change" pattern keyed on activeRunId, not a synchronous cascade.
    setPipelineSkipped(new Set<string>());
  }, [activeRunId]);

  function onReorder(from: number, to: number) {
    setPipelineAgents(reorderArray(pipelineAgents, from, to));
  }

  function onInsert(at: number, id: string) {
    setPipelineAgents(insertAt(pipelineAgents, at, id));
  }

  function onRemove(at: number) {
    setPipelineAgents(pipelineAgents.filter((_, i) => i !== at));
  }

  function onSkip(at: number) {
    setPipelineSkipped((prev) => {
      const next = new Set(prev);
      const id = pipelineAgents[at];
      if (id !== undefined) next.add(id);
      return next;
    });
  }

  return { pipelineAgents, pipelineSkipped, onReorder, onInsert, onRemove, onSkip };
}
