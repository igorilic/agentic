import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AuthAccount } from "../types/auth";

export default function SettingsPane() {
  const [accounts, setAccounts] = useState<AuthAccount[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
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
    void refresh();
  }, [refresh]);

  const onConnect = async () => {
    if (connecting) return;
    setConnecting(true);
    setConnectError(null);
    try {
      // Spec §15.4: zero-config delegate to the user's existing `gh` CLI
      // session. No browser flow, no OAuth-app registration. If the user
      // hasn't run `gh auth login`, the IPC returns an actionable error.
      await invoke("connect_github_via_gh");
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
              onClick={() => void onDisconnect(a.id)}
              data-testid={`disconnect-${a.id}`}
              className="px-2 py-1 text-xs rounded border border-gray-300 text-gray-700 hover:bg-gray-50"
            >
              Disconnect
            </button>
          </li>
        ))}
      </ul>

      <div className="flex flex-col gap-2 pt-2 border-t border-gray-200">
        <p className="text-xs text-gray-500">
          Reuses your existing <span className="font-mono">gh</span> CLI session.
          Run <span className="font-mono">gh auth login</span> first if you
          haven&apos;t already.
        </p>
        <button
          type="button"
          onClick={() => void onConnect()}
          disabled={connecting}
          data-testid="connect-github-submit"
          className="self-start px-3 py-1 bg-blue-600 text-white rounded text-sm disabled:bg-gray-400"
        >
          {connecting ? "Connecting…" : "Connect GitHub"}
        </button>
        {connectError && (
          <div
            role="alert"
            data-testid="connect-github-error"
            className="text-xs text-red-700 bg-red-50 border border-red-200 rounded px-2 py-1 whitespace-pre-wrap"
          >
            {connectError}
          </div>
        )}
      </div>
    </section>
  );
}
