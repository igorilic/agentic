import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Finding } from "../types/finding";

export type UseFindingsResult = {
  findings: Finding[];
  /** `null` while the fetch is pending or has succeeded; otherwise the
   * stringified error from the failed `list_findings` invoke. */
  error: string | null;
};

/**
 * Subscribe to findings for a run.
 *
 * Calls `list_findings(runId)` on mount (and whenever `runId` changes) and
 * stores the returned rows. Without `runId`, returns an empty list and does
 * not invoke — useful for the initial "no run yet" state.
 *
 * `refetchKey` is an opaque value the caller bumps to force a refetch on the
 * same `runId` (e.g., after a RunComplete envelope arrives). Findings are
 * not push-streamed in Phase 11.5; a future iteration may wire a
 * `agentic://finding` channel for live updates during a run.
 */
export function useFindings(runId?: string, refetchKey?: unknown): UseFindingsResult {
  const [findings, setFindings] = useState<Finding[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional: clear stale findings before refetch on runId/refetchKey change; the reset is part of the dep-change sequence, not a side-effect cascade.
    setFindings([]);
    setError(null);
    if (!runId) return;

    void (async () => {
      try {
        const rows = (await invoke("list_findings", { runId })) as Finding[];
        if (!cancelled) setFindings(rows);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [runId, refetchKey]);

  return { findings, error };
}
