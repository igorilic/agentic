import type { EventEnvelope } from "../types/event";

type EventListProps = {
  events: EventEnvelope[];
};

/**
 * Primitive scrollable list rendering one row per event. No filtering,
 * no styling beyond Tailwind. Phase 11 (cockpit) replaces this with
 * grouped/typed rendering.
 */
export default function EventList({ events }: EventListProps) {
  if (events.length === 0) {
    return (
      <div className="p-4 text-gray-500 italic">No events yet.</div>
    );
  }

  return (
    <ul
      className="divide-y divide-gray-200 overflow-y-auto max-h-[80vh]"
      data-testid="event-list"
    >
      {events.map((env) => (
        <li
          key={env.event_id}
          className="px-4 py-2 font-mono text-sm flex gap-4"
          data-testid="event-row"
        >
          <span className="text-gray-400 shrink-0">
            {new Date(env.timestamp_ms).toLocaleTimeString()}
          </span>
          <span className="text-blue-600 font-semibold shrink-0">
            {env.event.type}
          </span>
          <span className="text-gray-700 truncate" title={env.run_id}>
            {env.step_id ?? env.run_id.slice(0, 8)}
          </span>
        </li>
      ))}
    </ul>
  );
}
