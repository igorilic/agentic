import { SLASH_COMMAND_LIBRARY } from "./library";
import type { BackendKind, SlashCommand } from "./types";

/**
 * Services injected into the dispatcher. The dispatcher is pure-ish:
 * it doesn't import `@tauri-apps/api/core` directly so unit tests can
 * mock the services without mocking the entire Tauri layer.
 */
export type SlashServices = {
  /**
   * Start a planned run from a ticket reference. `backend` is the
   * user-supplied `--backend=…` flag from the slash parser; `undefined`
   * means the service should pick its default. Returns the new run_id.
   */
  plan: (ticket: string, backend: BackendKind | undefined) => Promise<string>;
  /** Get status text for a run (or "no active run" if no runId). */
  status: (runId: string | null) => Promise<string>;
  /** Cancel an in-flight run. Returns true if the run was active. */
  cancel: (runId: string) => Promise<boolean>;
};

/**
 * Result returned to the chat layer for display.
 */
export type DispatchResult = {
  /** Human-readable line to render as a system message. */
  message: string;
  /** Optional run_id resulting from the dispatch. */
  runId?: string;
};

export async function dispatchSlashCommand(
  cmd: SlashCommand,
  services: SlashServices,
): Promise<DispatchResult> {
  switch (cmd.kind) {
    case "plan": {
      const runId = await services.plan(cmd.ticket, cmd.backend);
      const backendNote = cmd.backend ? ` [${cmd.backend}]` : "";
      return {
        message: `Started run ${runId} for ticket${backendNote}: ${cmd.ticket}`,
        runId,
      };
    }
    case "status": {
      const text = await services.status(cmd.runId);
      return { message: text };
    }
    case "cancel": {
      const cancelled = await services.cancel(cmd.runId);
      return {
        message: cancelled
          ? `Cancelled run ${cmd.runId}`
          : `No active run with id ${cmd.runId}`,
      };
    }
    case "help": {
      const helpText = SLASH_COMMAND_LIBRARY
        .map((c) => `/${c.name} — ${c.desc}`)
        .join("\n");
      return { message: helpText };
    }
  }
}
