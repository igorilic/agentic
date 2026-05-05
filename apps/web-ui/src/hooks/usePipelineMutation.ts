import { useEffect, useState } from "react";
import type { RunState } from "../types/run";
import { reorderArray, insertAt } from "../utils/arrayMove";
import { derivePipelineSeed } from "../utils/derivePipelineSeed";

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
 * Re-seeds `pipelineAgents` from `runState` whenever `activeRunId` changes
 * from undefined → string. If the run's steps are non-empty, they override
 * the current list (e.g. re-attaching to an in-progress run).
 */
export function usePipelineMutation(
  runState: RunState,
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

  // Re-seed on run-id change only. Not on every runState tick — that would
  // clobber user edits made between run start and run completion.
  useEffect(() => {
    if (!activeRunId) return;  // only re-seed on undefined → string per spec §6.8.3
    const seed = derivePipelineSeed(runState);
    if (seed.length > 0) {
      setPipelineAgents(seed);
    }
    setPipelineSkipped(new Set<string>());
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
