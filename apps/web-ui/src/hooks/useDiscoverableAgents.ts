import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useBackend } from "./useBackend";
import type { AgentInfoDto } from "../types/agents";

export type { AgentInfoDto };

export interface UseDiscoverableAgentsResult {
  agents: AgentInfoDto[];
  isLoading: boolean;
  error: string | null;
  refetch: () => void;
}

/**
 * Fetch the list of discoverable agents for the currently selected backend.
 *
 * Re-fetches automatically when the backend selection changes.
 * Exposes `refetch()` for manual refresh (e.g. after the user adds a new
 * agent file to their repository).
 */
export function useDiscoverableAgents(): UseDiscoverableAgentsResult {
  const { backend } = useBackend();
  const [agents, setAgents] = useState<AgentInfoDto[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Increment to trigger a re-fetch without changing `backend`.
  const [fetchTick, setFetchTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(null);

    invoke<AgentInfoDto[]>("list_agents", { backend })
      .then((result) => {
        if (!cancelled) {
          setAgents(result);
          setIsLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          const message = typeof err === "string" ? err : String(err);
          setError(message);
          setAgents([]);
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
    // fetchTick is included so refetch() triggers a new effect run.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backend, fetchTick]);

  const refetch = useCallback(() => {
    setFetchTick((t) => t + 1);
  }, []);

  return { agents, isLoading, error, refetch };
}
