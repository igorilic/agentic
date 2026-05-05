import { EVENT_CHANNEL } from "../hooks/useTauriEvents";

type TauriHandler = (event: unknown) => void;

type WindowWithTauri = typeof globalThis & {
  __TAURI_INTERNALS__?: {
    invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    transformCallback: (handler: TauriHandler, once?: boolean) => number;
  };
};

// Callback registry: numeric id → handler function.
// Populated by transformCallback; cleared by plugin:event|unlisten.
let nextCallbackId = 1;
const callbacks = new Map<number, TauriHandler>();

// Subscription registry: event name → Set of callback ids.
const subscriptions = new Map<string, Set<number>>();

function transformCallback(handler: TauriHandler, _once = false): number {
  const id = nextCallbackId++;
  callbacks.set(id, handler);
  return id;
}

function emitToListeners(eventName: string, envelope: unknown): void {
  const ids = subscriptions.get(eventName);
  if (!ids) return;
  for (const id of ids) {
    const handler = callbacks.get(id);
    if (handler) {
      // Tauri 2 delivers events as { id, event, payload, windowLabel }
      handler({ id, event: eventName, payload: envelope, windowLabel: "main" });
    }
  }
}

function makeEnvelope(
  runId: string,
  eventId: string,
  stepId: string | null,
  event: { type: string; data?: Record<string, unknown> },
): Record<string, unknown> {
  return {
    schema_version: 1,
    event_id: eventId,
    run_id: runId,
    step_id: stepId,
    timestamp_ms: Date.now(),
    event,
  };
}

const AGENTS = ["architect", "tdd-developer", "qa", "reviewer"] as const;

function scheduleSimulatedRun(runId: string, ticketLabel: string): void {
  const t0 = Date.now();
  let delay = 250;

  // RunStarted
  setTimeout(() => {
    emitToListeners(
      EVENT_CHANNEL,
      makeEnvelope(runId, `e-${t0}-rs`, null, {
        type: "RunStarted",
        data: { ticket: ticketLabel },
      }),
    );
  }, delay);
  delay += 400;

  for (const agent of AGENTS) {
    const startDelay = delay;
    setTimeout(() => {
      emitToListeners(
        EVENT_CHANNEL,
        makeEnvelope(runId, `e-${t0}-${agent}-start`, agent, {
          type: "StepStarted",
          data: { agent },
        }),
      );
    }, startDelay);
    delay += 800;

    const completeDelay = delay;
    setTimeout(() => {
      emitToListeners(
        EVENT_CHANNEL,
        makeEnvelope(runId, `e-${t0}-${agent}-complete`, agent, {
          type: "StepComplete",
          data: {
            agent,
            status: "passed",
            duration_ms: 600,
            cost_usd: null,
            summary: null,
          },
        }),
      );
    }, completeDelay);
    delay += 200;
  }

  // RunComplete
  setTimeout(() => {
    emitToListeners(
      EVENT_CHANNEL,
      makeEnvelope(runId, `e-${t0}-rc`, null, {
        type: "RunComplete",
        data: { status: "completed" },
      }),
    );
  }, delay);
}

async function mockInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "start_ticket_run": {
      const runId = `dev-mock-${Date.now()}`;
      const ticketLabel = typeof args?.ticket === "string" ? args.ticket : "Untitled run";
      scheduleSimulatedRun(runId, ticketLabel);
      return runId;
    }
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
    case "get_workspace_id":
      return "ws-dev-mock-1234567";
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
    case "plugin:event|listen": {
      const eventName = (args?.event as string | undefined) ?? "";
      const handlerId = args?.handler as number | undefined;
      if (handlerId !== undefined && eventName) {
        if (!subscriptions.has(eventName)) {
          subscriptions.set(eventName, new Set());
        }
        subscriptions.get(eventName)!.add(handlerId);
      }
      return handlerId ?? Math.floor(Math.random() * 1_000_000);
    }
    case "plugin:event|unlisten": {
      const eventId = args?.eventId as number | undefined;
      if (eventId !== undefined) {
        for (const ids of subscriptions.values()) {
          ids.delete(eventId);
        }
        callbacks.delete(eventId);
      }
      return undefined;
    }
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
  // Reset module-level state on each install (important for test isolation).
  nextCallbackId = 1;
  callbacks.clear();
  subscriptions.clear();

  (window as WindowWithTauri).__TAURI_INTERNALS__ = {
    invoke: mockInvoke,
    transformCallback,
  };
  console.info("[dev-invoke-mock] active — Tauri IPC stubbed for browser dev");
  return true;
}
