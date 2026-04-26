import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { EventEnvelope } from "../types/event";

export const EVENT_CHANNEL = "agentic.event";

/**
 * Maximum number of envelopes kept in the sliding-window buffer.
 * When the buffer exceeds this size the oldest entries are dropped so that
 * React state stays lightweight (several KB instead of several MB for long
 * pipeline runs that can produce 10k+ events).
 */
export const MAX_EVENTS = 500;

/**
 * Subscribe to backend events emitted by the Tauri `subscribe_events` command.
 *
 * Returns the running list of envelopes received since mount, capped at
 * MAX_EVENTS (most-recent-N sliding window).
 *
 * - Registers the listener BEFORE invoking `subscribe_events` to avoid the
 *   spawn-vs-attach race (Step 10.3 doc).
 * - Tolerates re-mounts: each mount registers a fresh listener and re-invokes
 *   the command. The Rust side aborts the previous forwarder per Step 10.3 F2.
 *
 * KNOWN GAP (Phase 11): events published on the bus BEFORE the Rust forwarder
 * subscribes are silently dropped — Tauri `emit` is fire-and-forget with no
 * replay. Mid-run attach (e.g., refreshing the webview during a long run)
 * loses the prefix of the event stream. Phase 11 cockpit work should add a
 * `get_event_history` Tauri command that returns the persisted prefix so the
 * hook can pre-seed state before attaching the live listener. See GH issue.
 */
export function useTauriEvents(): EventEnvelope[] {
  const [events, setEvents] = useState<EventEnvelope[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    (async () => {
      const stop = await listen<EventEnvelope>(EVENT_CHANNEL, (event) => {
        setEvents((prev) => {
          const next = [...prev, event.payload];
          return next.length > MAX_EVENTS ? next.slice(-MAX_EVENTS) : next;
        });
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
