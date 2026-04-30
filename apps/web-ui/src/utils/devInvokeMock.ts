type WindowWithTauri = typeof globalThis & {
  __TAURI_INTERNALS__?: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
};

async function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "start_ticket_run":
      return { run_id: `dev-mock-${Date.now()}`, status: "running" };
    case "cancel_run":
      return undefined;
    case "mention_agent":
      return {
        run_id: `dev-mock-mention-${Date.now()}`,
        agent: typeof args?.agent === "string" ? args.agent : "architect",
        dispatched: true,
      };
    case "chat_send_message": {
      const session_id = (args?.sessionId as string | null) ?? `dev-session-${Date.now()}`;
      const content = (args?.content as string) ?? "";
      const now = Date.now();
      return {
        user_message: {
          id: `usr-${now}`,
          session_id,
          run_id: null,
          role: "user",
          content,
          metadata: null,
          created_at: now,
        },
        reply: {
          id: `asst-${now}`,
          session_id,
          run_id: null,
          role: "assistant",
          content: "[dev-mock] this would be the agent reply",
          metadata: null,
          created_at: now + 1,
        },
      };
    }
    case "list_runs":
    case "list_auth_accounts":
    case "get_event_history":
    case "list_findings":
      return [];
    case "subscribe_events":
    case "triage_finding":
    case "connect_github_via_gh":
    case "delete_auth_account":
      return undefined;
    // The Tauri events plugin uses plugin:event|listen / plugin:event|unlisten.
    // listen() resolves with an unsubscriber id (number); unlisten takes that id.
    case "plugin:event|listen":
      return Math.floor(Math.random() * 1_000_000);
    case "plugin:event|unlisten":
      return undefined;
    default:
      console.warn(`[dev-invoke-mock] unhandled command: ${cmd}`, args);
      return undefined;
  }
}

export function installDevInvokeMock(): boolean {
  if (!import.meta.env.DEV) {
    return false;
  }
  if (typeof window === "undefined") {
    return false;
  }
  if ((window as WindowWithTauri).__TAURI_INTERNALS__ !== undefined) {
    return false;
  }
  (window as WindowWithTauri).__TAURI_INTERNALS__ = { invoke: mockInvoke };
  console.info("[dev-invoke-mock] active — Tauri IPC stubbed for browser dev");
  return true;
}
