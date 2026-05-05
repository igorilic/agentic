import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { JiraTicketDto } from "../types/jira";

export type { JiraTicketDto };

export interface UseJiraFetchResult {
  fetch: (key: string) => Promise<JiraTicketDto>;
  isLoading: boolean;
  error: string | null;
}

export function useJiraFetch(): UseJiraFetchResult {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // useCallback with empty deps: setters are stable, no closed-over state.
  // Stable identity prevents re-render loops when consumers put `fetch` in
  // a useEffect / memo dependency array (e.g. F.2.4's SpecDialog row).
  const fetchTicket = useCallback(async (key: string): Promise<JiraTicketDto> => {
    setIsLoading(true);
    setError(null);
    try {
      const dto = await invoke<JiraTicketDto>("fetch_jira_ticket", { key });
      return dto;
    } catch (err) {
      const message = typeof err === "string" ? err : String(err);
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  return { fetch: fetchTicket, isLoading, error };
}
