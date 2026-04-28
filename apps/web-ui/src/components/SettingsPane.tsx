import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AuthAccount } from "../types/auth";

export default function SettingsPane() {
  const [accounts, setAccounts] = useState<AuthAccount[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [clientId, setClientId] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [connectError, setConnectError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const rows = (await invoke("list_auth_accounts")) as AuthAccount[];
      setAccounts(rows);
      setLoadError(null);
    } catch (e) {
      setLoadError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const onConnect = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!clientId.trim() || connecting) return;
    setConnecting(true);
    setConnectError(null);
    try {
      await invoke("connect_github", { clientId: clientId.trim() });
      setClientId("");
      await refresh();
    } catch (err) {
      setConnectError(String(err));
    } finally {
      setConnecting(false);
    }
  };

  const onDisconnect = async (accountId: string) => {
    try {
      await invoke("delete_auth_account", { accountId });
      await refresh();
    } catch (err) {
      setLoadError(`Disconnect failed: ${err}`);
    }
  };

  return (
    <section
      data-testid="settings-pane"
      className="border border-gray-200 rounded p-4 space-y-4"
    >
      <h2 className="text-lg font-semibold text-gray-900">Settings — Accounts</h2>

      {loadError && (
        <div
          role="alert"
          data-testid="settings-load-error"
          className="px-3 py-2 bg-red-50 border border-red-200 rounded text-sm text-red-700"
        >
          {loadError}
        </div>
      )}

      <ul
        className="divide-y divide-gray-100 border border-gray-100 rounded"
        aria-label="Connected accounts"
      >
        {accounts.length === 0 && (
          <li className="px-3 py-2 italic text-gray-400">
            No accounts connected.
          </li>
        )}
        {accounts.map((a) => (
          <li
            key={a.id}
            data-testid={`auth-account-row-${a.id}`}
            className="px-3 py-2 flex items-center justify-between gap-3"
          >
            <div className="flex flex-col">
              <span className="text-sm font-mono text-gray-800">
                {a.provider} · {a.host}
              </span>
              {a.username && (
                <span className="text-xs text-gray-500">@{a.username}</span>
              )}
            </div>
            <button
              type="button"
              onClick={() => onDisconnect(a.id)}
              data-testid={`disconnect-${a.id}`}
              className="px-2 py-1 text-xs rounded border border-gray-300 text-gray-700 hover:bg-gray-50"
            >
              Disconnect
            </button>
          </li>
        ))}
      </ul>

      <form
        onSubmit={onConnect}
        className="flex flex-col gap-2 pt-2 border-t border-gray-200"
        data-testid="connect-github-form"
      >
        <label className="text-sm text-gray-700">
          Connect GitHub — paste your OAuth App's <code>client_id</code>
        </label>
        <p className="text-xs text-gray-500">
          Register at{" "}
          <span className="font-mono">
            github.com → Settings → Developer settings → OAuth Apps
          </span>
          . Set the callback URL to <span className="font-mono">http://127.0.0.1/*</span>.
        </p>
        <div className="flex gap-2">
          <input
            type="text"
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
            placeholder="Iv1.…"
            data-testid="connect-github-client-id"
            className="flex-1 px-2 py-1 border border-gray-300 rounded text-sm font-mono"
            disabled={connecting}
          />
          <button
            type="submit"
            disabled={!clientId.trim() || connecting}
            data-testid="connect-github-submit"
            className="px-3 py-1 bg-blue-600 text-white rounded text-sm disabled:bg-gray-400"
          >
            {connecting ? "Connecting…" : "Connect"}
          </button>
        </div>
        {connectError && (
          <div
            role="alert"
            data-testid="connect-github-error"
            className="text-xs text-red-700 bg-red-50 border border-red-200 rounded px-2 py-1"
          >
            {connectError}
          </div>
        )}
      </form>
    </section>
  );
}
