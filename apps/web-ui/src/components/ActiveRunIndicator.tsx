/**
 * Compact strip shown next to the chat input while a ticket run is in
 * flight. Displays the truncated run_id, live-updating elapsed time, and
 * a Cancel button. Disappears entirely when no run is active.
 */

import { useEffect, useState } from "react";

export type ActiveRunIndicatorProps = {
  /** The currently active run, or null when no run is in flight. */
  runId: string | null;
  /** Unix-ms timestamp of the run's `RunStarted` envelope, or null
   *  before that envelope has been observed. */
  startedAtMs: number | null;
  /** Triggers the cancel IPC. The component awaits the returned promise
   *  and disables the button while it's pending. */
  onCancel: () => Promise<void>;
};

export default function ActiveRunIndicator({
  runId,
  startedAtMs,
  onCancel,
}: ActiveRunIndicatorProps) {
  const [now, setNow] = useState(() => Date.now());
  const [cancelling, setCancelling] = useState(false);

  // Tick once per second so the elapsed time stays current. Only run the
  // interval when a run is actually active to avoid pointless wakeups.
  useEffect(() => {
    if (runId === null) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [runId]);

  if (runId === null) return null;

  const elapsedText =
    startedAtMs === null ? "starting…" : formatElapsed(now - startedAtMs);

  const handleCancel = async () => {
    if (cancelling) return;
    setCancelling(true);
    try {
      await onCancel();
    } finally {
      setCancelling(false);
    }
  };

  return (
    <div
      data-testid="active-run-indicator"
      className="flex items-center gap-3 px-3 py-1.5 bg-blue-50 border border-blue-200 rounded text-xs text-blue-900"
    >
      <span className="font-mono opacity-70">run</span>
      <span className="font-mono font-semibold">{runId.slice(0, 8)}</span>
      <span data-testid="active-run-elapsed" className="ml-auto opacity-80">
        {elapsedText}
      </span>
      <button
        type="button"
        onClick={handleCancel}
        disabled={cancelling}
        data-testid="active-run-cancel"
        className="px-2 py-0.5 text-xs rounded border border-blue-300 hover:bg-blue-100 disabled:opacity-50"
      >
        {cancelling ? "Cancelling…" : "Cancel"}
      </button>
    </div>
  );
}

function formatElapsed(ms: number): string {
  if (ms < 0) ms = 0;
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const remS = s % 60;
  if (m < 60) return `${m}m ${remS}s`;
  const h = Math.floor(m / 60);
  const remM = m % 60;
  return `${h}h ${remM}m`;
}
