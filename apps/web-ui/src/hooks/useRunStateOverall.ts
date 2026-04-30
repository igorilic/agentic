import { useMemo, useState, useEffect } from "react";
import type { EventEnvelope } from "../types/event";
import type { RunStateOverall } from "../types/pipeline";

export type UseRunStateOverallResult = {
  overallRunState: RunStateOverall;
  startedAtMs: number | null;
  elapsedMs: number | null;
};

export function useRunStateOverall(
  events: EventEnvelope[],
  activeRunId: string | undefined,
): UseRunStateOverallResult {
  const overallRunState: RunStateOverall = useMemo(() => {
    if (activeRunId === undefined) {
      return events.some((e) => e.event.type === "RunComplete") ? "completed" : "idle";
    }
    const lastEvent = events[events.length - 1];
    if (lastEvent?.event.type === "RunComplete") return "completed";
    if (lastEvent?.event.type === "RunFailed") return "failed";
    return "running";
  }, [activeRunId, events]);

  const startedAtMs = useMemo<number | null>(() => {
    if (!activeRunId) return null;
    const first = events.find((e) => e.run_id === activeRunId);
    return first ? first.timestamp_ms : null;
  }, [events, activeRunId]);

  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (overallRunState !== "running") return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [overallRunState]);

  const elapsedMs = useMemo<number | null>(() => {
    if (overallRunState === "idle") return null;
    if (startedAtMs === null) return null;
    return now - startedAtMs;
  }, [overallRunState, startedAtMs, now]);

  return { overallRunState, startedAtMs, elapsedMs };
}
