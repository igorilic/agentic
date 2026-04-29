import * as path from "path";
import * as vscode from "vscode";
import { registerCommands } from "./commands";
import {
  AgenticDiffProvider,
  openDiffForFileChange,
  FileChangeEnvelope,
} from "./diff";
import { AgenticChatViewProvider } from "./views/sidebar";

export function activate(context: vscode.ExtensionContext): void {
  registerCommands(context);

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      AgenticChatViewProvider.viewType,
      new AgenticChatViewProvider(context.extensionUri),
    ),
  );

  // ── Snapshot store location ──────────────────────────────────────────────
  // Data lives under <globalStorageUri>/agentic so each VS Code profile gets
  // its own snapshot directory. The extension creates the directory if absent.
  //
  // TODO(14.6+): use the active workspace's data_dir instead so the extension
  // and the agentic-core run that created the snapshots share the same dir.
  const dataDir = path.join(context.globalStorageUri.fsPath, "agentic");

  // ── Register agentic:// TextDocumentContentProvider ──────────────────────
  // The real napi fetcher is loaded lazily so the extension host doesn't fail
  // to activate when @agentic/node is not installed (e.g. in CI without the
  // native .node binary). Callers that test the provider pass a stub fetcher
  // through the constructor; the real fetcher is only loaded here.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const node = require("@agentic/node/lib.js") as {
    getFileSnapshot: (opts: { dataDir: string; hash: string }) => Promise<Buffer>;
    subscribeEvents: (runId: string) => unknown;
    iterate: (stream: unknown) => AsyncIterable<{ event: { type: string; path?: string; before_hash?: string; after_hash?: string } }>;
  };

  const diffProvider = new AgenticDiffProvider(dataDir, node.getFileSnapshot.bind(node));
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider("agentic", diffProvider),
  );

  // ── Subscribe to all-runs event stream ───────────────────────────────────
  // Step 14.6 will narrow this to a specific active run_id via QuickPick /
  // sidebar state. For now we subscribe to a sentinel run_id that the user
  // will start from the sidebar; events for unrecognised run_ids are silently
  // dropped by EventStream.next() because the filter never matches.
  //
  // The subscription is a no-op at runtime until the user kicks off a run
  // from the sidebar that emits FileChange events. The openDiffForFileChange
  // path is exercised by the unit test that stubs executeCommand directly.
  const workspaceRoot =
    vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? ".";

  // Subscribe using a well-known sentinel; 14.6 replaces this with the
  // actual run_id returned by startRun.
  const SENTINEL_RUN_ID = "__agentic_active_run__";
  const stream = node.subscribeEvents(SENTINEL_RUN_ID);

  void (async () => {
    for await (const env of node.iterate(stream)) {
      if (env.event.type === "FileChange") {
        await openDiffForFileChange(env as unknown as FileChangeEnvelope, workspaceRoot);
      }
    }
  })();
}

export function deactivate(): void {
  // nothing to clean up
}
