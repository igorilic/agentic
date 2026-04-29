import * as path from "path";
import * as vscode from "vscode";
import { registerCommands, registerTriageCommand } from "./commands";
import {
  AgenticDiffProvider,
  openDiffForFileChange,
  FileChangeEnvelope,
} from "./diff";
import {
  FindingsDecorator,
  FindingEnvelope,
  defaultDecorationTypeFactory,
} from "./decorations";
import { AgenticChatViewProvider } from "./views/sidebar";

/** Shape of the @agentic/node exports used by the extension. */
interface AgenticNode {
  getFileSnapshot: (opts: { dataDir: string; hash: string }) => Promise<Buffer>;
  subscribeEvents: (runId: string) => unknown;
  iterate: (stream: unknown) => AsyncIterable<{
    event: {
      type: string;
      path?: string;
      before_hash?: string;
      after_hash?: string;
    };
  }>;
  triageFinding: (args: {
    dataDir: string;
    runId: string;
    findingId: string;
    triage: string;
  }) => Promise<void>;
}

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
  // (GH issue tracking this is filed alongside the Step 14.5 review.)
  const dataDir = path.join(context.globalStorageUri.fsPath, "agentic");

  // ── FindingsDecorator ─────────────────────────────────────────────────────
  // Manages per-URI finding state and applies squiggle decorations.
  // Created before the lazy node require so the triage command can be
  // registered on the context.subscriptions before activate() might throw.
  // Disposed via context.subscriptions when the extension deactivates.
  const decorator = new FindingsDecorator(defaultDecorationTypeFactory, dataDir);
  context.subscriptions.push(decorator);

  // Re-apply decorations when the user switches editor tabs.
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((ed) => {
      decorator.reapply(ed);
    }),
  );

  // Cover the cold-open case: VS Code only fires
  // `onDidChangeActiveTextEditor` when an editor *switches* — opening a
  // file from the Explorer in a fresh window (no prior active editor)
  // fires `onDidOpenTextDocument` only. Match the opened doc against
  // visibleTextEditors and reapply for each that points at the same
  // URI. `reapply` is a no-op when the URI has no findings.
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      for (const ed of vscode.window.visibleTextEditors) {
        if (ed.document.uri.toString() === doc.uri.toString()) {
          decorator.reapply(ed);
        }
      }
    }),
  );

  // ── Register agentic:// TextDocumentContentProvider ──────────────────────
  // The real napi module is loaded lazily so the extension host doesn't fail
  // to activate when @agentic/node is not installed (e.g. in CI without the
  // native .node binary). If the require fails, napi-dependent features are
  // silently disabled — the decorator and command surface are already wired up.
  let node: AgenticNode | null = null;
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    node = require("@agentic/node/lib.js") as AgenticNode;
  } catch {
    // napi binary unavailable (e.g. running tests without a built native module).
    // Log a warning but don't crash activation.
    console.warn("agentic: @agentic/node not available — diff and event features disabled");
  }

  // Register `agentic.triage` with its real handler (replaces the stub).
  // We pass a thin wrapper so that if node is unavailable the command still
  // registers but shows an error message rather than crashing.
  registerTriageCommand(context, decorator, {
    triageFinding: async (args) => {
      if (!node) {
        throw new Error("@agentic/node is not available");
      }
      return node.triageFinding(args);
    },
  });

  if (node) {
    const diffProvider = new AgenticDiffProvider(
      dataDir,
      node.getFileSnapshot.bind(node),
    );
    context.subscriptions.push(
      vscode.workspace.registerTextDocumentContentProvider("agentic", diffProvider),
    );

    // ── Subscribe to all-runs event stream ─────────────────────────────────
    // The loop is owned by an AbortController so deactivate() can cancel it.
    const workspaceRoot =
      vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? ".";
    const SENTINEL_RUN_ID = "__agentic_active_run__";
    const stream = node.subscribeEvents(SENTINEL_RUN_ID);

    const abort = new AbortController();
    context.subscriptions.push({ dispose: () => abort.abort() });

    void (async () => {
      try {
        for await (const env of node!.iterate(stream)) {
          if (abort.signal.aborted) return;
          if (env.event.type === "FileChange") {
            await openDiffForFileChange(
              env as unknown as FileChangeEnvelope,
              workspaceRoot,
            );
          } else if (env.event.type === "Finding") {
            decorator.handleFinding(
              env as unknown as FindingEnvelope,
              workspaceRoot,
              vscode.window.activeTextEditor,
            );
          }
        }
      } catch (err) {
        // AbortError on deactivate is expected; anything else is worth logging
        // but shouldn't crash the extension host.
        if ((err as { name?: string })?.name !== "AbortError") {
          console.error("agentic event-loop error:", err);
        }
      }
    })();
  }
}

export function deactivate(): void {
  // AbortController disposer pushed onto context.subscriptions in activate()
  // tears the event loop down — VS Code calls dispose() on each subscription
  // automatically, so this hook can stay empty.
}
