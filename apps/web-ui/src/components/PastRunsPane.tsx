import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RunSummary } from "../types/run_summary";

const STATUS_STYLES: Record<string, string> = {
  pending: "bg-gray-100 text-gray-600",
  running: "bg-blue-100 text-blue-800",
  completed: "bg-green-100 text-green-800",
  completed_with_tech_debt: "bg-yellow-100 text-yellow-800",
  failed: "bg-red-100 text-red-800",
  cancelled: "bg-orange-100 text-orange-800",
  crashed: "bg-purple-100 text-purple-800",
};

export type PastRunsPaneProps = {
  /** Optional click handler — receives the full run_id when a row is
   *  selected. The cockpit uses this to pin its FindingsTable to that
   *  run for after-the-fact triage. */
  onSelectRun?: (runId: string) => void;
};

export default function PastRunsPane({ onSelectRun }: PastRunsPaneProps = {}) {
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const rows = (await invoke("list_runs", { limit: 50 })) as RunSummary[];
      setRuns(rows);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <section
      data-testid="past-runs-pane"
      className="border border-gray-200 rounded p-4 space-y-3"
    >
      <div className="flex items-baseline justify-between">
        <h2 className="text-lg font-semibold text-gray-900">Past runs</h2>
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
          data-testid="past-runs-refresh"
          className="text-xs text-blue-600 hover:underline disabled:opacity-50"
        >
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {error && (
        <div
          role="alert"
          data-testid="past-runs-error"
          className="px-3 py-2 bg-red-50 border border-red-200 rounded text-sm text-red-700"
        >
          {error}
        </div>
      )}

      {!error && runs.length === 0 && !loading && (
        <p className="text-sm text-gray-400 italic">No past runs yet.</p>
      )}

      {runs.length > 0 && (
        <ul className="divide-y divide-gray-100 border border-gray-100 rounded overflow-hidden">
          {runs.map((r) => {
            const statusClass = STATUS_STYLES[r.status] ?? "bg-gray-100 text-gray-600";
            const startedAt = new Date(r.started_at).toLocaleString();
            const duration =
              r.duration_ms != null ? `${(r.duration_ms / 1000).toFixed(1)}s` : null;
            return (
              <li
                key={r.id}
                data-testid={`past-run-row-${r.id}`}
                onClick={() => onSelectRun?.(r.id)}
                className={`px-3 py-2 flex items-center gap-3 ${
                  onSelectRun ? "cursor-pointer hover:bg-gray-50" : ""
                }`}
              >
                <span
                  className={`px-2 py-0.5 text-xs rounded font-medium shrink-0 ${statusClass}`}
                >
                  {r.status}
                </span>
                <span className="font-mono text-xs text-gray-500 shrink-0">
                  {r.id.slice(0, 8)}
                </span>
                <span className="text-sm text-gray-800 flex-1 truncate">
                  {r.ticket_label ?? <em className="text-gray-400">(no ticket)</em>}
                </span>
                <span className="text-xs text-gray-400 shrink-0">{r.backend}</span>
                {duration && (
                  <span className="text-xs text-gray-400 shrink-0">{duration}</span>
                )}
                <span className="text-xs text-gray-400 shrink-0">{startedAt}</span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
