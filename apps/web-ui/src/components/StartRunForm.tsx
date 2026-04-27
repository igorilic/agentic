import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { EventEnvelope } from "../types/event";

type StartRunFormProps = {
  events: EventEnvelope[];
  activeRunId?: string | undefined;
  onActiveRunIdChange?: (id: string | undefined) => void;
};

// data-testid attributes below are stable test contracts — do not rename without
// updating StartRunForm.test.tsx:
//   start-run-form, script-path-input, delay-ms-input,
//   start-button, cancel-button, active-run-id, error-message

export default function StartRunForm({ events, activeRunId, onActiveRunIdChange }: StartRunFormProps) {
  const [scriptPath, setScriptPath] = useState("");
  const [delayMsRaw, setDelayMsRaw] = useState("100");
  const [error, setError] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);

  // F1: clear active run when its terminal RunComplete event arrives.
  useEffect(() => {
    if (!activeRunId) return;
    const last = events[events.length - 1];
    if (last && last.event.type === "RunComplete" && last.run_id === activeRunId) {
      onActiveRunIdChange?.(undefined);
    }
  }, [events, activeRunId, onActiveRunIdChange]);

  const onStart = async (e?: React.FormEvent) => {
    e?.preventDefault();
    setError(null);
    if (!scriptPath.trim()) {
      setError("Script path is required.");
      return;
    }
    setIsStarting(true);
    try {
      const result = await invoke("start_scripted_run", {
        scriptPath: scriptPath.trim(),
        delayMs: Math.max(0, Number(delayMsRaw) || 0), // F3: clamp negative values
      });
      // F5: runtime guard — invoke must return a string run_id
      if (typeof result !== "string") {
        setError(`Unexpected return from start_scripted_run: ${typeof result}`);
        return;
      }
      onActiveRunIdChange?.(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsStarting(false);
    }
  };

  const onCancel = async () => {
    if (!activeRunId) return;
    try {
      await invoke("cancel_run", { runId: activeRunId });
      onActiveRunIdChange?.(undefined);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    // F7: form wrapper so Enter in path input triggers Start
    <form
      className="px-6 py-4 border-b border-gray-200 flex flex-col gap-3"
      data-testid="start-run-form"
      onSubmit={onStart}
    >
      <div className="flex gap-3 items-center">
        <label className="flex flex-col gap-1 flex-1">
          <span className="text-sm font-medium text-gray-700">Script path</span>
          <input
            type="text"
            placeholder="/path/to/script.json"
            value={scriptPath}
            onChange={(e) => setScriptPath(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded font-mono text-sm"
            data-testid="script-path-input"
          />
        </label>
        <label className="flex flex-col gap-1 w-28">
          <span className="text-sm font-medium text-gray-700">Delay (ms)</span>
          <input
            type="text"
            inputMode="numeric"
            value={delayMsRaw}
            onChange={(e) => setDelayMsRaw(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded font-mono text-sm"
            data-testid="delay-ms-input"
          />
        </label>
        {/* stable test selector — do not rename without updating StartRunForm.test.tsx */}
        <button
          type="submit"
          disabled={!!activeRunId || isStarting}
          className="px-4 py-2 bg-blue-600 text-white rounded font-medium hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed self-end"
          data-testid="start-button"
        >
          Start
        </button>
        {/* stable test selector — do not rename without updating StartRunForm.test.tsx */}
        <button
          type="button"
          onClick={onCancel}
          disabled={!activeRunId || isStarting}
          className="px-4 py-2 bg-red-600 text-white rounded font-medium hover:bg-red-700 disabled:bg-gray-400 disabled:cursor-not-allowed self-end"
          data-testid="cancel-button"
        >
          Cancel
        </button>
      </div>
      {activeRunId && (
        <p className="text-xs text-gray-600" data-testid="active-run-id">
          Active run: <code className="font-mono">{activeRunId}</code>
        </p>
      )}
      {error && (
        <p className="text-sm text-red-600" data-testid="error-message" role="alert">
          {error}
        </p>
      )}
    </form>
  );
}
