import { invoke } from "@tauri-apps/api/core";
import type { BackendKind } from "../slash/types";

/**
 * Invokes the start_ticket_run IPC with the spec title as the ticket
 * label. Used by IssueColumn's "Create spec" action and ChatColumn's
 * "New spec" composer affordance.
 *
 * The `agents` parameter is the user's selected pipeline agents list
 * (from `usePipelineMutation().pipelineAgents` in App.tsx). Pass the
 * live list rather than a hardcoded default so the user's pipeline
 * configuration is respected.
 */
export async function createSpec(
  title: string,
  backend: BackendKind,
  agents: string[],
): Promise<string | undefined> {
  const result = await invoke("start_ticket_run", {
    ticket: title,
    backend,
    model: null,
    agents,
  });
  return typeof result === "string" ? result : undefined;
}
