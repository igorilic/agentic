// Native diff editor support for FileChange events.
//
// Provides two exports:
//
//   AgenticDiffProvider — a TextDocumentContentProvider for the
//   "agentic" scheme. Resolves agentic://before/<hash> by fetching the
//   before-state blob via the injected snapshot fetcher (real code passes
//   getFileSnapshot from @agentic/node; tests pass a stub).
//
//   openDiffForFileChange — opens vscode.diff for a FileChange event
//   envelope. Constructs the virtual before-URI and the workspace file
//   URI, then invokes 'vscode.diff' via executeCommand.

import * as path from "path";
import * as vscode from "vscode";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Minimal shape of a FileChange event as serialised by agentic-core. */
export interface FileChangeEnvelope {
  run_id: string;
  step_id?: string | null;
  event: {
    type: "FileChange";
    /** Relative or absolute path of the changed file. */
    path: string;
    before_hash: string;
    after_hash: string;
  };
}

/** Injected snapshot-fetcher signature (matches the napi export shape). */
export type SnapshotFetcher = (opts: {
  dataDir: string;
  hash: string;
}) => Promise<Buffer>;

// ---------------------------------------------------------------------------
// AgenticDiffProvider
// ---------------------------------------------------------------------------

/**
 * TextDocumentContentProvider for the `agentic` URI scheme.
 *
 * URI format: `agentic://before/<hash>`
 *
 * The `before` host identifies the before-state bucket; `<hash>` is the
 * blake3 hex from the FileChange event's `before_hash` field.
 *
 * The `dataDir` and `fetcher` are injected so unit tests can stub both
 * without touching the napi binary or the file system.
 */
export class AgenticDiffProvider
  implements vscode.TextDocumentContentProvider
{
  constructor(
    private readonly dataDir: string,
    private readonly fetcher: SnapshotFetcher,
  ) {}

  async provideTextDocumentContent(uri: vscode.Uri): Promise<string> {
    // Only `agentic://before/<hash>` is recognised today. A future
    // step may add `agentic://after/...` — when it does, route here
    // rather than silently calling the fetcher with whatever hash.
    if (uri.authority !== "before") {
      throw new Error(
        `Unsupported agentic:// URI authority "${uri.authority}" — expected "before"`,
      );
    }
    // URI path is "/<hash>" — strip the leading slash.
    const hash = uri.path.replace(/^\//, "");
    const bytes = await this.fetcher({ dataDir: this.dataDir, hash });
    return bytes.toString("utf8");
  }
}

// ---------------------------------------------------------------------------
// openDiffForFileChange
// ---------------------------------------------------------------------------

/**
 * Open the native VS Code diff editor for a FileChange event.
 *
 * Left side: `agentic://before/<before_hash>` — the before-state blob
 *   fetched by AgenticDiffProvider.
 * Right side: `file://<workspaceRoot>/<path>` — the live workspace file.
 * Label: `<filename> (Agentic change)` — contains "Agentic" so users can
 *   distinguish agent-originated diffs from normal editor diffs.
 *
 * The `workspaceRoot` should be the root of the repository the agent is
 * modifying. For 14.6+ this will be the workspace's `root_path`; for now
 * callers supply it explicitly.
 */
export async function openDiffForFileChange(
  envelope: FileChangeEnvelope,
  workspaceRoot: string,
): Promise<void> {
  const { path: filePath, before_hash } = envelope.event;

  const beforeUri = vscode.Uri.parse(`agentic://before/${before_hash}`);

  const absolutePath = path.isAbsolute(filePath)
    ? filePath
    : path.join(workspaceRoot, filePath);
  const workspaceUri = vscode.Uri.file(absolutePath);

  const filename = path.basename(filePath);
  const label = `${filename} (Agentic change)`;

  await vscode.commands.executeCommand(
    "vscode.diff",
    beforeUri,
    workspaceUri,
    label,
  );
}
