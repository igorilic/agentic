import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { JiraTicketDto } from "../types/jira";

export type { JiraTicketDto };

export interface UseJiraFetchResult {
  fetch: (key: string) => Promise<JiraTicketDto>;
  isLoading: boolean;
  error: string | null;
  /** Always true for v1 — env-var presence is surfaced via error message. */
  isAvailable: boolean;
}

export function useJiraFetch(): UseJiraFetchResult {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function fetchTicket(key: string): Promise<JiraTicketDto> {
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
  }

  return { fetch: fetchTicket, isLoading, error, isAvailable: true };
}
