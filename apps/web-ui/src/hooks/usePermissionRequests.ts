import { useMemo } from "react";
import { useTauriEvents } from "./useTauriEvents";
import { isPermissionRequestEvent, isPermissionResolvedEvent } from "../types/event";
import type { PermissionRequest } from "../types/pipeline";

/**
 * Subscribes to the useTauriEvents envelope stream and reduces it into the
 * current set of unresolved PermissionRequest entries, keyed by request_id.
 *
 * A matching PermissionResolved envelope removes the entry from the set.
 * The event log (useTauriEvents) is the single source of truth — this hook
 * holds no independent state of its own.
 *
 * Wire format (snake_case `request_id`) is mapped to the UI type's camelCase
 * `requestId` field (renamed in P.3.2). Other fields flow through unchanged.
 */
export function usePermissionRequests(): PermissionRequest[] {
  const { events } = useTauriEvents();

  return useMemo(() => {
    const map = new Map<string, PermissionRequest>();
    for (const env of events) {
      if (isPermissionRequestEvent(env.event)) {
        const d = env.event.data;
        map.set(d.request_id, {
          requestId: d.request_id,
          agent: d.agent,
          tool: d.tool,
          arg: d.arg,
          scope: d.scope,
          risk: d.risk,
          reason: d.reason,
        });
      } else if (isPermissionResolvedEvent(env.event)) {
        map.delete(env.event.data.request_id);
      }
    }
    return Array.from(map.values());
  }, [events]);
}
