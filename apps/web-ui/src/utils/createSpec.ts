import { invoke } from "@tauri-apps/api/core";

/**
 * Invokes the start_ticket_run IPC with the spec title as the ticket
 * label. Used by IssueColumn's "Create spec" action and ChatColumn's
 * "New spec" composer affordance.
 *
 * The body parameter is captured at the SpecDialog boundary but
 * intentionally dropped at the IPC layer — start_ticket_run accepts
 * only { ticket, backend, model }. Tracked in GH #92 for when the
 * backend gains a body/description field.
 */
export async function createSpec(title: string): Promise<string | undefined> {
  const result = (await invoke("start_ticket_run", {
    ticket: title,
    backend: "claude-code",
    model: null,
  })) as { run_id?: string };
  return typeof result.run_id === "string" ? result.run_id : undefined;
}
