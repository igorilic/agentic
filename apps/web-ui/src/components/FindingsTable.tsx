import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Finding, Triage } from "../types/finding";

const TRIAGE_OPTIONS: ReadonlyArray<{ value: Triage; label: string }> = [
  { value: "fix", label: "Fix" },
  { value: "tech-debt", label: "Tech-debt" },
  { value: "ignore", label: "Ignore" },
];

export type FindingsTableProps = {
  findings: Finding[];
};

export default function FindingsTable({ findings }: FindingsTableProps) {
  // Per-row optimistic triage state. After invoke succeeds, the row's badge
  // updates locally without a round-trip to refetch the run's findings —
  // re-fetching is a future concern (Phase 11.5+ may add `list_findings`).
  const [pending, setPending] = useState<Record<string, boolean>>({});
  const [overrides, setOverrides] = useState<Record<string, Triage>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  // Which rows have their suggestion section expanded. Suggestions can be
  // long, so default-collapsed; user opts in per-row.
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const onTriage = async (runId: string, findingId: string, triage: Triage) => {
    setPending((p) => ({ ...p, [findingId]: true }));
    setErrors((e) => {
      const { [findingId]: _drop, ...rest } = e;
      return rest;
    });
    try {
      // findings PK is composite (run_id, id) since migration 0008.
      await invoke("triage_finding", { runId, findingId, triage });
      setOverrides((o) => ({ ...o, [findingId]: triage }));
    } catch (err) {
      setErrors((e) => ({ ...e, [findingId]: String(err) }));
    } finally {
      setPending((p) => ({ ...p, [findingId]: false }));
    }
  };

  if (findings.length === 0) {
    return (
      <section
        data-testid="findings-table"
        className="border border-gray-200 rounded p-4 text-sm text-gray-500 italic"
      >
        No findings yet.
      </section>
    );
  }

  return (
    <section
      data-testid="findings-table"
      className="border border-gray-200 rounded overflow-hidden"
    >
      <ul className="divide-y divide-gray-100">
        {findings.map((f) => {
          const currentTriage = overrides[f.id] ?? f.triage;
          const isPending = pending[f.id] ?? false;
          const error = errors[f.id];
          return (
            <li
              key={f.id}
              data-testid={`finding-row-${f.id}`}
              className="px-3 py-2 flex flex-col gap-1"
            >
              <div className="flex gap-3 items-baseline">
                <span
                  className={`text-xs font-semibold uppercase shrink-0 ${
                    f.severity === "error"
                      ? "text-red-600"
                      : f.severity === "warning"
                        ? "text-yellow-700"
                        : "text-gray-500"
                  }`}
                >
                  {f.severity}
                </span>
                <span className="text-sm text-gray-800 flex-1">{f.message}</span>
                {f.suggestion && (
                  <button
                    type="button"
                    onClick={() =>
                      setExpanded((e) => ({ ...e, [f.id]: !e[f.id] }))
                    }
                    aria-expanded={expanded[f.id] ?? false}
                    aria-label={
                      expanded[f.id] ? "Hide suggestion" : "Show suggestion"
                    }
                    data-testid={`suggestion-toggle-${f.id}`}
                    className="shrink-0 text-xs px-1.5 py-0.5 rounded border border-gray-200 text-gray-500 hover:bg-gray-50"
                    title="Suggestion"
                  >
                    💡
                  </button>
                )}
                {f.file_path && (
                  <span
                    data-testid={`finding-file-${f.id}`}
                    className="block text-xs text-gray-400 font-mono sm:shrink-0"
                  >
                    {f.file_path}
                    {f.line != null ? `:${f.line}` : ""}
                  </span>
                )}
              </div>
              {f.suggestion && expanded[f.id] && (
                <div
                  data-testid={`suggestion-body-${f.id}`}
                  className="ml-6 px-3 py-2 bg-blue-50 border-l-2 border-blue-200 text-sm text-gray-700 whitespace-pre-wrap"
                >
                  <span className="text-xs uppercase font-semibold text-blue-700 mr-2">
                    suggestion
                  </span>
                  {f.suggestion}
                </div>
              )}
              <div
                data-testid={`triage-actions-${f.id}`}
                className="flex flex-wrap gap-2 items-center"
              >
                {TRIAGE_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => void onTriage(f.run_id, f.id, opt.value)}
                    disabled={isPending}
                    data-testid={`triage-${opt.value}-${f.id}`}
                    className={`px-2 py-0.5 text-xs rounded border transition ${
                      currentTriage === opt.value
                        ? "bg-blue-600 text-white border-blue-600"
                        : "bg-white text-gray-700 border-gray-300 hover:bg-gray-50"
                    } disabled:opacity-50 disabled:cursor-not-allowed`}
                  >
                    {opt.label}
                  </button>
                ))}
                {currentTriage && (
                  <span
                    data-testid={`triage-badge-${f.id}`}
                    className="text-xs text-gray-500 ml-auto"
                  >
                    {currentTriage}
                  </span>
                )}
              </div>
              {error && (
                <div
                  role="alert"
                  data-testid={`triage-error-${f.id}`}
                  className="text-xs text-red-600"
                >
                  {error}
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
