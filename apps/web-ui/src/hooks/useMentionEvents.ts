import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { EventEnvelope } from "../types/event";

/**
 * Channel name for @mention-dispatch event envelopes. Distinct from the
 * cockpit `agentic://event` channel so the Stepper does NOT pick up chat
 * traffic.
 */
export const MENTION_EVENT_CHANNEL = "agentic://mention-event";

/**
 * Maximum number of envelopes kept in the chat-side mention buffer. Sized
 * smaller than the cockpit window because mention output is conversational
 * and does not need long replay.
 */
export const MAX_MENTION_EVENTS = 200;

/**
 * Subscribe to backend mention events emitted by `mention_agent`.
 *
 * Unlike `useTauriEvents`, there is no history-fetch IPC for mention output
 * (Phase 11.4 stubs do not persist). The hook simply attaches a listener and
 * accumulates a deduplicated, capped sliding window of envelopes for display
 * in the chat pane.
 */
export function useMentionEvents(): EventEnvelope[] {
  const [events, setEvents] = useState<EventEnvelope[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    void (async () => {
      const stop = await listen<EventEnvelope>(MENTION_EVENT_CHANNEL, (event) => {
        setEvents((prev) => {
          if (prev.some((e) => e.event_id === event.payload.event_id)) {
            return prev;
          }
          const next = [...prev, event.payload];
          return next.length > MAX_MENTION_EVENTS ? next.slice(-MAX_MENTION_EVENTS) : next;
        });
      });
      if (cancelled) {
        stop();
        return;
      }
      unlisten = stop;
    })();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  return events;
}
