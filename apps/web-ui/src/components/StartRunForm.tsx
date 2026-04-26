import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function StartRunForm() {
  const [scriptPath, setScriptPath] = useState("");
  const [delayMs, setDelayMs] = useState(100);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const onStart = async () => {
    setError(null);
    if (!scriptPath.trim()) {
      setError("Script path is required.");
      return;
    }
    try {
      const runId: string = await invoke("start_scripted_run", {
        scriptPath: scriptPath.trim(),
        delayMs,
      });
      setActiveRunId(runId);
    } catch (e) {
      setError(String(e));
    }
  };

  const onCancel = async () => {
    if (!activeRunId) return;
    try {
      await invoke("cancel_run", { runId: activeRunId });
      setActiveRunId(null);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="px-6 py-4 border-b border-gray-200 flex flex-col gap-3" data-testid="start-run-form">
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
            type="number"
            value={delayMs}
            onChange={(e) => setDelayMs(Number(e.target.value) || 0)}
            className="px-3 py-2 border border-gray-300 rounded font-mono text-sm"
            min={0}
            data-testid="delay-ms-input"
          />
        </label>
        <button
          type="button"
          onClick={onStart}
          disabled={!!activeRunId}
          className="px-4 py-2 bg-blue-600 text-white rounded font-medium hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed self-end"
          data-testid="start-button"
        >
          Start
        </button>
        <button
          type="button"
          onClick={onCancel}
          disabled={!activeRunId}
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
    </div>
  );
}
