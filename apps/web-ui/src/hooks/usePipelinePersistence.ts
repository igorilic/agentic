import { useCallback, useEffect, useState } from "react";

/**
 * Persists the user's chosen pipeline agent list per-workspace in localStorage.
 *
 * Key format: `agentic.pipeline.<wsId>`
 * Value:      JSON-serialized `string[]`
 *
 * When `wsId` is `null` (workspace id not yet resolved), returns an empty list
 * and a no-op setter — mutations are silently dropped until wsId is known.
 *
 * On parse error (corrupt JSON or non-array value):
 *   - returns `[]`
 *   - clears the corrupt key from localStorage
 *   - emits a `console.warn` with the bad payload
 */
export function usePipelinePersistence(wsId: string | null): {
  pipelineAgents: string[];
  setPipelineAgents: (next: string[]) => void;
} {
  const [pipelineAgents, setPipelineAgentsState] = useState<string[]>(() => {
    if (wsId === null) return [];
    return readFromStorage(wsId);
  });

  // Re-read storage whenever wsId changes
  useEffect(() => {
    if (wsId === null) {
      setPipelineAgentsState([]);
      return;
    }
    setPipelineAgentsState(readFromStorage(wsId));
  }, [wsId]);

  const setPipelineAgents = useCallback(
    (next: string[]) => {
      if (wsId === null) return; // no-op
      localStorage.setItem(`agentic.pipeline.${wsId}`, JSON.stringify(next));
      setPipelineAgentsState(next);
    },
    [wsId],
  );

  return { pipelineAgents, setPipelineAgents };
}

function readFromStorage(wsId: string): string[] {
  const raw = localStorage.getItem(`agentic.pipeline.${wsId}`);
  if (raw === null) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      console.warn(
        `[usePipelinePersistence] expected array at agentic.pipeline.${wsId}, got:`,
        raw,
      );
      localStorage.removeItem(`agentic.pipeline.${wsId}`);
      return [];
    }
    return parsed as string[];
  } catch {
    console.warn(
      `[usePipelinePersistence] corrupt JSON at agentic.pipeline.${wsId}:`,
      raw,
    );
    localStorage.removeItem(`agentic.pipeline.${wsId}`);
    return [];
  }
}
