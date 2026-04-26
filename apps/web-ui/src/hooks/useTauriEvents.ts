import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { EventEnvelope } from "../types/event";

export const EVENT_CHANNEL = "agentic://event";

/**
 * Subscribe to backend events emitted by the Tauri `subscribe_events` command.
 * Returns the running list of envelopes received since mount, in arrival order.
 *
 * - Registers the listener BEFORE invoking `subscribe_events` to avoid the
 *   spawn-vs-attach race (Step 10.3 doc).
 * - Tolerates re-mounts: each mount registers a fresh listener and re-invokes
 *   the command. The Rust side aborts the previous forwarder per Step 10.3 F2.
 */
export function useTauriEvents(): EventEnvelope[] {
  const [events, setEvents] = useState<EventEnvelope[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    (async () => {
      const stop = await listen<EventEnvelope>(EVENT_CHANNEL, (event) => {
        setEvents((prev) => [...prev, event.payload]);
      });
      if (cancelled) {
        stop();
        return;
      }
      unlisten = stop;
      // Listener attached — now ask the backend to start forwarding.
      await invoke("subscribe_events");
    })();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  return events;
}
