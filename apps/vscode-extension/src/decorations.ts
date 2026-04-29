// Step 14.6 — Findings → editor decorations
//
// Exports:
//   Finding           — minimal shape for a reviewer finding.
//   FindingEnvelope   — shape of the event envelope from the napi stream.
//   buildHoverMarkdown — pure helper; returns a trusted MarkdownString with
//                        message + optional suggestion + [Fix] [Tech-debt] [Ignore] links.
//   FindingsDecorator  — manages per-URI finding state and applies squiggle
//                        decorations to the active editor.

import * as path from "path";
import * as vscode from "vscode";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Minimal shape of a reviewer finding (matches napi Event::Finding data
 *  plus an external `runId` field injected when the envelope is processed). */
export interface Finding {
  id: string;
  runId: string;
  severity: "error" | "warning" | "info";
  /** Absolute or workspace-relative path, or undefined (drop silently). */
  file?: string;
  /** 0-based line number, or undefined (store but don't decorate). */
  line?: number;
  message: string;
  suggestion?: string;
}

/** Shape of the event envelope as serialised by agentic-core → napi stream. */
export interface FindingEnvelope {
  run_id: string;
  event: {
    type: "Finding";
    finding_id: string;
    severity: "error" | "warning" | "info";
    file?: string;
    /** 0-based line number */
    line?: number;
    message: string;
    suggestion?: string;
  };
}

/** Injected factory so tests can stub TextEditorDecorationType creation. */
export type SeverityToDecorationType = (
  severity: "error" | "warning" | "info",
) => vscode.TextEditorDecorationType;

// ---------------------------------------------------------------------------
// Default factory (used by the real extension host)
// ---------------------------------------------------------------------------

/** Creates a squiggle TextEditorDecorationType for the given severity. */
export function defaultDecorationTypeFactory(
  severity: "error" | "warning" | "info",
): vscode.TextEditorDecorationType {
  const colorMap: Record<string, string> = {
    error: "rgba(255,80,80,0.7)",
    warning: "rgba(255,200,0,0.7)",
    info: "rgba(80,160,255,0.7)",
  };
  return vscode.window.createTextEditorDecorationType({
    textDecoration: `underline wavy ${colorMap[severity]}`,
  });
}

// ---------------------------------------------------------------------------
// buildHoverMarkdown
// ---------------------------------------------------------------------------

/**
 * Build a trusted MarkdownString containing:
 *   - The finding message (bold)
 *   - (Optional) suggestion text
 *   - Three command-URI links: [Fix], [Tech-debt], [Ignore]
 *
 * `isTrusted` is set to `true` so VS Code renders the `command:` URIs.
 */
export function buildHoverMarkdown(
  finding: Finding,
  dataDir: string,
): vscode.MarkdownString {
  const makeLink = (label: string, triage: string): string => {
    const args = encodeURIComponent(
      JSON.stringify({
        dataDir,
        runId: finding.runId,
        findingId: finding.id,
        triage,
      }),
    );
    return `[${label}](command:agentic.triage?${args})`;
  };

  const lines: string[] = [];
  lines.push(`**${finding.message}**`);
  if (finding.suggestion) {
    lines.push(`\n*Suggestion*: ${finding.suggestion}`);
  }
  lines.push(
    `\n${makeLink("Fix", "fix")} | ${makeLink("Tech-debt", "tech-debt")} | ${makeLink("Ignore", "ignore")}`,
  );

  const md = new vscode.MarkdownString(lines.join("\n"));
  md.isTrusted = true;
  return md;
}

// ---------------------------------------------------------------------------
// FindingsDecorator
// ---------------------------------------------------------------------------

/**
 * Manages per-URI finding state and applies squiggle decorations.
 *
 * Constructor-injected `typeFactory` creates the three severity-keyed
 * TextEditorDecorationType instances so unit tests can stub them.
 */
export class FindingsDecorator {
  /** Per-URI list of stored findings. Key is `vscode.Uri.file(path).toString()`. */
  private readonly findings = new Map<string, Finding[]>();

  private readonly types: Record<string, vscode.TextEditorDecorationType>;

  constructor(
    typeFactory: SeverityToDecorationType,
    private readonly dataDir: string,
  ) {
    this.types = {
      error: typeFactory("error"),
      warning: typeFactory("warning"),
      info: typeFactory("info"),
    };
  }

  // ── Public API ────────────────────────────────────────────────────────────

  /**
   * Ingest a Finding event envelope.
   *
   * - Drops silently when `event.file` is absent.
   * - Resolves relative paths against `workspaceRoot`.
   * - Stores the finding; if an `editor` whose URI matches the file is
   *   provided, immediately applies decorations to it.
   */
  handleFinding(
    env: FindingEnvelope,
    workspaceRoot: string,
    editor?: vscode.TextEditor,
  ): void {
    const { event } = env;
    if (!event.file) return;

    const absPath = path.isAbsolute(event.file)
      ? event.file
      : path.join(workspaceRoot, event.file);

    const uriKey = vscode.Uri.file(absPath).toString();

    const finding: Finding = {
      id: event.finding_id,
      runId: env.run_id,
      severity: event.severity,
      file: absPath,
      line: event.line,
      message: event.message,
      suggestion: event.suggestion,
    };

    const list = this.findings.get(uriKey) ?? [];
    // Dedup by id — bus replay (retry, re-subscription) can re-emit the
    // same finding. Without this guard the line would render two
    // overlapping squiggles. GH #84 covers stacking *distinct* findings
    // on the same line; this is the same-id-twice case.
    if (list.some((f) => f.id === finding.id)) {
      return;
    }
    list.push(finding);
    this.findings.set(uriKey, list);

    // Apply decorations to the provided editor if its URI matches
    if (editor && editor.document.uri.toString() === uriKey) {
      this._applyToEditor(editor, list);
    }
  }

  /**
   * Remove the finding identified by `findingId` from internal state and
   * refresh decorations on `editor` if supplied.
   */
  clearFinding(findingId: string, editor?: vscode.TextEditor): void {
    for (const [key, list] of this.findings.entries()) {
      const idx = list.findIndex((f) => f.id === findingId);
      if (idx === -1) continue;

      const updated = list.filter((f) => f.id !== findingId);
      if (updated.length === 0) {
        this.findings.delete(key);
      } else {
        this.findings.set(key, updated);
      }

      // Refresh the editor if it shows the file we just mutated
      if (editor && editor.document.uri.toString() === key) {
        this._applyToEditor(editor, updated);
      }
      return;
    }
  }

  /**
   * Re-apply decorations for the active editor.
   * Called by `vscode.window.onDidChangeActiveTextEditor`.
   * No-op when the editor's URI has no cached findings.
   */
  reapply(editor: vscode.TextEditor | undefined): void {
    if (!editor) return;
    const key = editor.document.uri.toString();
    const list = this.findings.get(key);
    if (!list || list.length === 0) return;
    this._applyToEditor(editor, list);
  }

  /** Dispose all three TextEditorDecorationType instances. */
  dispose(): void {
    for (const dt of Object.values(this.types)) {
      dt.dispose();
    }
  }

  // ── Test-visible inspection helpers ──────────────────────────────────────
  // These are intentionally public for white-box unit testing.

  getFindingsForUri(uriKey: string): Finding[] | undefined {
    return this.findings.get(uriKey);
  }

  getAllUriKeys(): string[] {
    return Array.from(this.findings.keys());
  }

  // ── Private helpers ───────────────────────────────────────────────────────

  /** Apply all three severity buckets to `editor`. */
  private _applyToEditor(
    editor: vscode.TextEditor,
    findings: Finding[],
  ): void {
    for (const sev of ["error", "warning", "info"] as const) {
      const opts: vscode.DecorationOptions[] = findings
        .filter((f) => f.severity === sev && f.line !== undefined)
        .map((f) => {
          // f.line is 0-based per the napi contract
          const line = f.line!;
          const lineText = editor.document.lineAt(line);
          const range = lineText.range;
          return {
            range,
            hoverMessage: buildHoverMarkdown(f, this.dataDir),
          };
        });
      editor.setDecorations(this.types[sev], opts);
    }
  }
}
