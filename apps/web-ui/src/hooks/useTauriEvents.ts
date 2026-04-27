import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { EventEnvelope } from "../types/event";

export const EVENT_CHANNEL = "agentic://event";

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
 * On mount, optionally calls `get_event_history(runId)` first to pre-seed
 * state with envelopes published before the listener attached. Live events
 * arriving after history-fetch are deduplicated by `event_id`.
 *
 * Without `runId`, no history fetch is performed (useful for the initial
 * "no run yet" state). When the user starts a run, the hook can be re-keyed
 * by passing the new run_id.
 *
 * When `runId` changes, state is cleared so only the new run's events are
 * shown — users navigating between runs expect to see only the current run.
 *
 * Returns the running list of envelopes received since mount, capped at
 * MAX_EVENTS (most-recent-N sliding window).
 */
export function useTauriEvents(runId?: string): EventEnvelope[] {
  const [events, setEvents] = useState<EventEnvelope[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    // Clear state when runId changes so only the new run's events are shown.
    setEvents([]);

    (async () => {
      // Fetch history first if a runId is known.
      if (runId) {
        try {
          const history = (await invoke("get_event_history", { runId })) as EventEnvelope[];
          if (!cancelled) {
            setEvents((prev) => {
              const seen = new Set(prev.map((e) => e.event_id));
              const fresh = history.filter((e) => !seen.has(e.event_id));
              const next = [...prev, ...fresh];
              return next.length > MAX_EVENTS ? next.slice(-MAX_EVENTS) : next;
            });
          }
        } catch (e) {
          console.warn("get_event_history failed:", e);
        }
      }

      const stop = await listen<EventEnvelope>(EVENT_CHANNEL, (event) => {
        setEvents((prev) => {
          // Dedupe by event_id (handles overlap with history fetch).
          if (prev.some((e) => e.event_id === event.payload.event_id)) {
            return prev;
          }
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
  }, [runId]);

  return events;
}
