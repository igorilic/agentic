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
 * State is re-seeded from `runState` whenever `activeRunId` changes
 * (new run starts or page loads). User edits made between re-seeds are
 * preserved because the dependency array only contains `activeRunId`.
 *
 * Tech-debt #7 tracks eventual backend persistence of these mutations.
 */
export function usePipelineMutation(
  runState: RunState,
  activeRunId: string | undefined,
): UsePipelineMutationResult {
  const [pipelineAgents, setPipelineAgents] = useState<string[]>(() =>
    derivePipelineSeed(runState),
  );
  const [pipelineSkipped, setPipelineSkipped] = useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );

  // Re-seed on run-id change only. Not on every runState tick — that would
  // clobber user edits made between run start and run completion.
  useEffect(() => {
    setPipelineAgents(derivePipelineSeed(runState));
    setPipelineSkipped(new Set<string>());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeRunId]);

  function onReorder(from: number, to: number) {
    setPipelineAgents((prev) => reorderArray(prev, from, to));
  }

  function onInsert(at: number, id: string) {
    setPipelineAgents((prev) => insertAt(prev, at, id));
  }

  function onRemove(at: number) {
    setPipelineAgents((prev) => prev.filter((_, i) => i !== at));
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
