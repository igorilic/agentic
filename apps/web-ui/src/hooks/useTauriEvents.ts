import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { EventEnvelope } from "../types/event";
import { isEventEnvelope, isEventEnvelopeArray } from "../types/event";

export const EVENT_CHANNEL = "agentic://event";

/**
 * Maximum number of envelopes kept in the sliding-window buffer.
 * When the buffer exceeds this size the oldest entries are dropped so that
 * React state stays lightweight (several KB instead of several MB for long
 * pipeline runs that can produce 10k+ events).
 */
export const MAX_EVENTS = 500;

export type UseTauriEventsResult = {
  events: EventEnvelope[];
  /** `null` while history is loading or has succeeded; otherwise the error
   * from the failed `get_event_history` invoke (stringified). */
  historyError: string | null;
};

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
 * Returns `{ events, historyError }` where `events` is the running list of
 * envelopes capped at MAX_EVENTS (most-recent-N sliding window), and
 * `historyError` is `null` on success or the stringified error on failure.
 */
export function useTauriEvents(runId?: string): UseTauriEventsResult {
  const [events, setEvents] = useState<EventEnvelope[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    // Only clear state when entering a new run (defined runId). Transitioning
    // to `undefined` — what App.tsx does on RunComplete (clears activeRunId) —
    // keeps the just-finished run's events visible so the user can review them.
    // Without this guard, the activity log would go blank the instant a run
    // finishes, and the FindingsTable refetch (which keys on the last event
    // being RunComplete) would silently miss its trigger.
    if (runId !== undefined) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional: clear stale events when entering a new run; the empty/clear here is part of the run-key reset sequence, not a synchronous setState cascade.
      setEvents([]);
      setHistoryError(null);
    }

    void (async () => {
      try {
        // Fetch history first if a runId is known.
        if (runId) {
          try {
            const result = await invoke("get_event_history", { runId });
            if (!isEventEnvelopeArray(result)) {
              throw new Error(
                `get_event_history returned malformed shape: ${
                  typeof result === "object"
                    ? JSON.stringify(result).slice(0, 200)
                    : String(result)
                }`,
              );
            }
            const history = result;
            if (!cancelled) {
              setEvents((prev) => {
                const seen = new Set(prev.map((e) => e.event_id));
                const fresh = history.filter((e) => !seen.has(e.event_id));
                const next = [...prev, ...fresh];
                return next.length > MAX_EVENTS ? next.slice(-MAX_EVENTS) : next;
              });
            }
          } catch (e) {
            if (!cancelled) {
              setHistoryError(String(e));
            }
          }
        }

        const stop = await listen<EventEnvelope>(EVENT_CHANNEL, (event) => {
          if (!isEventEnvelope(event.payload)) {
            console.error(
              "[useTauriEvents] malformed live envelope skipped:",
              event.payload,
            );
            return;
          }
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
      } catch (e) {
        // listen() or subscribe_events failed — surface to the user instead
        // of turning into an unhandled promise rejection (the old `void` IIFE
        // without an outer try/catch would silently discard these errors).
        if (!cancelled) {
          setHistoryError(String(e));
        }
      }
    })();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [runId]);

  return { events, historyError };
}
