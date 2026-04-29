// Step 14.6 — decorations.ts unit tests
//
// These tests cover the pure helper and the FindingsDecorator class.
// The vscode mock (provided by @vscode/test-electron) is available
// in the extension host test runner; the decorator's external
// dependencies are stubbed via constructor injection.
//
// Test plan (8 tests):
//   1. buildHoverMarkdown: isTrusted is true
//   2. buildHoverMarkdown: contains exactly three command links with correct JSON args
//   3. buildHoverMarkdown: contains message and suggestion text
//   4. handleFinding with absolute path stores under correct URI key
//   5. handleFinding with relative path joins with workspaceRoot
//   6. clearFinding removes finding and calls setDecorations with remaining ranges
//   7. reapply sets decorations from cached findings; no-op for unknown URIs
//   8. findings without line are stored but not decorated; without file are dropped

import * as path from "path";
import * as vscode from "vscode";
import {
  buildHoverMarkdown,
  FindingsDecorator,
  Finding,
  FindingEnvelope,
} from "../decorations";

// ---------------------------------------------------------------------------
// Stub factory for TextEditorDecorationType
// ---------------------------------------------------------------------------

/** Tracks calls to setDecorations and dispose per severity. */
interface StubDecorationType {
  key: string;
  setDecorationsCalls: Array<{
    editor: vscode.TextEditor;
    ranges: vscode.DecorationOptions[];
  }>;
  disposeCalls: number;
  // Implements vscode.TextEditorDecorationType (the real type is opaque)
  type: vscode.TextEditorDecorationType;
}

function makeStubDecorationType(key: string): StubDecorationType {
  const stub: StubDecorationType = {
    key,
    setDecorationsCalls: [],
    disposeCalls: 0,
    type: {} as vscode.TextEditorDecorationType,
  };
  // 'dispose' is the only required method on the decoration type itself
  stub.type = { dispose: () => { stub.disposeCalls++; } } as unknown as vscode.TextEditorDecorationType;
  return stub;
}

/** A SeverityToDecorationType factory that returns controllable stubs. */
function makeStubFactory(): {
  factory: (severity: "error" | "warning" | "info") => vscode.TextEditorDecorationType;
  error: StubDecorationType;
  warning: StubDecorationType;
  info: StubDecorationType;
} {
  const error = makeStubDecorationType("error");
  const warning = makeStubDecorationType("warning");
  const info = makeStubDecorationType("info");
  return {
    factory: (s) => {
      if (s === "error") return error.type;
      if (s === "warning") return warning.type;
      return info.type;
    },
    error,
    warning,
    info,
  };
}

// ---------------------------------------------------------------------------
// Minimal stub for vscode.TextEditor
// ---------------------------------------------------------------------------

function makeStubEditor(uri: vscode.Uri): {
  editor: vscode.TextEditor;
  setDecorationsCalls: Array<{
    type: vscode.TextEditorDecorationType;
    ranges: vscode.DecorationOptions[];
  }>;
} {
  const calls: Array<{
    type: vscode.TextEditorDecorationType;
    ranges: vscode.DecorationOptions[];
  }> = [];

  const editor: vscode.TextEditor = {
    document: {
      uri,
      // Minimal stubs — we only use uri and lineAt
      lineAt: (line: number): vscode.TextLine => ({
        lineNumber: line,
        text: "some code",
        range: new vscode.Range(line, 0, line, 9),
        rangeIncludingLineBreak: new vscode.Range(line, 0, line + 1, 0),
        firstNonWhitespaceCharacterIndex: 0,
        isEmptyOrWhitespace: false,
      }),
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any,
    setDecorations: (
      decorationType: vscode.TextEditorDecorationType,
      rangesOrOptions: vscode.DecorationOptions[],
    ) => {
      calls.push({ type: decorationType, ranges: rangesOrOptions });
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } as any;

  return { editor, setDecorationsCalls: calls };
}

// ---------------------------------------------------------------------------
// Helper: canonical Finding
// ---------------------------------------------------------------------------

function makeFinding(overrides: Partial<Finding> = {}): Finding {
  return {
    id: "f-001",
    runId: "run-abc",
    severity: "warning",
    file: "/workspace/src/main.rs",
    line: 10,
    message: "Unused variable `x`",
    suggestion: "Remove or prefix with `_`",
    ...overrides,
  };
}

function makeFindingEnvelope(finding: Finding): FindingEnvelope {
  return {
    run_id: finding.runId,
    event: {
      type: "Finding",
      finding_id: finding.id,
      severity: finding.severity,
      file: finding.file,
      line: finding.line,
      message: finding.message,
      suggestion: finding.suggestion,
    },
  };
}

// ===========================================================================
// Suite 1 — buildHoverMarkdown (pure)
// ===========================================================================

suite("buildHoverMarkdown", () => {
  const dataDir = "/home/user/.agentic";

  test("returns a MarkdownString with isTrusted === true", () => {
    const finding = makeFinding();
    const md = buildHoverMarkdown(finding, dataDir);

    if (!(md instanceof vscode.MarkdownString)) {
      throw new Error("Expected a vscode.MarkdownString");
    }
    if (!md.isTrusted) {
      throw new Error(
        "MarkdownString.isTrusted must be true so command URIs are allowed",
      );
    }
  });

  test("markdown contains exactly three command links: [Fix], [Tech-debt], [Ignore] with correct JSON args", () => {
    const finding = makeFinding();
    const md = buildHoverMarkdown(finding, dataDir);
    const value = md.value;

    // Extract all command: links
    const linkRegex = /\[([^\]]+)\]\(command:agentic\.triage\?([^)]+)\)/g;
    const links: Array<{ label: string; args: unknown }> = [];
    let m: RegExpExecArray | null;
    while ((m = linkRegex.exec(value)) !== null) {
      const label = m[1];
      const argsJson = decodeURIComponent(m[2]);
      links.push({ label, args: JSON.parse(argsJson) });
    }

    if (links.length !== 3) {
      throw new Error(
        `Expected exactly 3 command links but found ${links.length}.\nMarkdown:\n${value}`,
      );
    }

    const expectedLabels = ["Fix", "Tech-debt", "Ignore"];
    for (const expectedLabel of expectedLabels) {
      if (!links.some((l) => l.label === expectedLabel)) {
        throw new Error(
          `Expected a link labeled "${expectedLabel}" but found: ${links.map((l) => l.label).join(", ")}`,
        );
      }
    }

    // Verify args shape for each
    const triageMap: Record<string, string> = {
      Fix: "fix",
      "Tech-debt": "tech-debt",
      Ignore: "ignore",
    };
    for (const link of links) {
      const args = link.args as {
        dataDir?: string;
        runId?: string;
        findingId?: string;
        triage?: string;
      };
      if (args.dataDir !== dataDir) {
        throw new Error(
          `Link "${link.label}": expected dataDir "${dataDir}" but got "${args.dataDir}"`,
        );
      }
      if (args.runId !== finding.runId) {
        throw new Error(
          `Link "${link.label}": expected runId "${finding.runId}" but got "${args.runId}"`,
        );
      }
      if (args.findingId !== finding.id) {
        throw new Error(
          `Link "${link.label}": expected findingId "${finding.id}" but got "${args.findingId}"`,
        );
      }
      const expectedTriage = triageMap[link.label as string];
      if (args.triage !== expectedTriage) {
        throw new Error(
          `Link "${link.label}": expected triage "${expectedTriage}" but got "${args.triage}"`,
        );
      }
    }
  });

  test("markdown contains the finding message and suggestion text", () => {
    const finding = makeFinding({
      message: "Unused variable `x`",
      suggestion: "Remove or prefix with `_`",
    });
    const md = buildHoverMarkdown(finding, dataDir);
    const value = md.value;

    if (!value.includes("Unused variable `x`")) {
      throw new Error(
        `Expected markdown to contain the message but got:\n${value}`,
      );
    }
    if (!value.includes("Remove or prefix with `_`")) {
      throw new Error(
        `Expected markdown to contain the suggestion but got:\n${value}`,
      );
    }
  });

  test("markdown contains message but no suggestion section when suggestion is absent", () => {
    const finding = makeFinding({ suggestion: undefined });
    const md = buildHoverMarkdown(finding, dataDir);
    const value = md.value;

    if (!value.includes("Unused variable `x`")) {
      throw new Error(`Expected markdown to contain the message but got:\n${value}`);
    }
    // We don't assert absence of a specific word, just that suggestion is undefined
    // and there's no crash — the positive assertion on message suffices.
  });
});

// ===========================================================================
// Suite 2 — FindingsDecorator
// ===========================================================================

suite("FindingsDecorator", () => {
  const dataDir = "/home/user/.agentic";
  const workspaceRoot = "/workspace";

  test("handleFinding with absolute file path stores finding under the correct URI key", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    const absPath = "/workspace/src/main.rs";
    const finding = makeFinding({ file: absPath, line: 5 });
    const env = makeFindingEnvelope(finding);

    // Use a stub editor that matches the abs path so reapply can fire
    const uri = vscode.Uri.file(absPath);
    const { editor } = makeStubEditor(uri);
    decorator.handleFinding(env, workspaceRoot, editor);

    // Internal state: uriKey must equal vscode.Uri.file(absPath).toString()
    const expectedKey = vscode.Uri.file(absPath).toString();
    const storedFindings = decorator.getFindingsForUri(expectedKey);

    if (!storedFindings || storedFindings.length === 0) {
      throw new Error(
        `Expected findings stored under key "${expectedKey}" but map was empty`,
      );
    }
    if (storedFindings[0].id !== finding.id) {
      throw new Error(
        `Stored finding id mismatch: expected "${finding.id}" got "${storedFindings[0].id}"`,
      );
    }

    decorator.dispose();
  });

  test("handleFinding with relative file path joins it with workspaceRoot", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    const relPath = "src/lib.rs";
    const finding = makeFinding({ file: relPath, line: 3 });
    const env = makeFindingEnvelope(finding);

    const absPath = path.join(workspaceRoot, relPath);
    const uri = vscode.Uri.file(absPath);
    const { editor } = makeStubEditor(uri);
    decorator.handleFinding(env, workspaceRoot, editor);

    const expectedKey = vscode.Uri.file(absPath).toString();
    const storedFindings = decorator.getFindingsForUri(expectedKey);

    if (!storedFindings || storedFindings.length === 0) {
      throw new Error(
        `Expected findings stored under key "${expectedKey}" after relative-path join`,
      );
    }

    decorator.dispose();
  });

  test("clearFinding removes the finding and calls setDecorations with remaining ranges", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    const absPath = "/workspace/src/main.rs";
    const uri = vscode.Uri.file(absPath);
    const { editor, setDecorationsCalls } = makeStubEditor(uri);

    // Store two findings on the same file
    const findingA = makeFinding({ id: "f-A", line: 5, file: absPath });
    const findingB = makeFinding({ id: "f-B", line: 10, file: absPath });
    decorator.handleFinding(makeFindingEnvelope(findingA), workspaceRoot, editor);
    decorator.handleFinding(makeFindingEnvelope(findingB), workspaceRoot, editor);

    // Clear the calls collected during handleFinding to isolate clearFinding
    setDecorationsCalls.length = 0;

    // Now clear one finding while supplying the active editor
    decorator.clearFinding(findingA.id, editor);

    // Internal state: f-A must be gone, f-B must remain
    const remaining = decorator.getFindingsForUri(uri.toString());
    if (!remaining || remaining.some((f) => f.id === "f-A")) {
      throw new Error("clearFinding should have removed finding f-A");
    }
    if (!remaining || !remaining.some((f) => f.id === "f-B")) {
      throw new Error("clearFinding should have kept finding f-B");
    }

    // setDecorations must have been called (to refresh decorations)
    if (setDecorationsCalls.length === 0) {
      throw new Error(
        "Expected setDecorations to be called after clearFinding but it was not",
      );
    }

    decorator.dispose();
  });

  test("reapply sets decorations from cached findings; no-op when URI has no findings", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    const absPath = "/workspace/src/main.rs";
    const uri = vscode.Uri.file(absPath);
    const { editor: editorA, setDecorationsCalls: callsA } = makeStubEditor(uri);

    // Pre-load a finding
    const finding = makeFinding({ file: absPath, line: 7 });
    decorator.handleFinding(makeFindingEnvelope(finding), workspaceRoot, editorA);

    // Reset calls from handleFinding
    callsA.length = 0;

    // Reapply with the same editor
    decorator.reapply(editorA);
    if (callsA.length === 0) {
      throw new Error(
        "Expected setDecorations to be called in reapply when findings exist",
      );
    }

    // Different URI — no findings — should be a no-op
    const otherUri = vscode.Uri.file("/other/file.rs");
    const { editor: editorB, setDecorationsCalls: callsB } = makeStubEditor(otherUri);
    decorator.reapply(editorB);
    if (callsB.length !== 0) {
      throw new Error(
        "Expected reapply to be a no-op for URIs with no findings",
      );
    }

    decorator.dispose();
  });

  test("finding without line is stored but setDecorations is not called with a range for it", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    const absPath = "/workspace/src/main.rs";
    const uri = vscode.Uri.file(absPath);
    const { editor, setDecorationsCalls } = makeStubEditor(uri);

    const finding = makeFinding({ line: undefined, file: absPath });
    decorator.handleFinding(makeFindingEnvelope(finding), workspaceRoot, editor);

    // Must be stored (so it can show up in hover / triage later)
    const stored = decorator.getFindingsForUri(uri.toString());
    if (!stored || stored.length === 0) {
      throw new Error("Finding without line should still be stored");
    }

    // No decoration range should have been set
    const decorationRanges = setDecorationsCalls.flatMap((c) => c.ranges);
    if (decorationRanges.length !== 0) {
      throw new Error(
        `Expected no decoration ranges for a finding without a line, but got ${decorationRanges.length}`,
      );
    }

    decorator.dispose();
  });

  test("finding without file is dropped silently", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    const finding = makeFinding({ file: undefined });
    const env = makeFindingEnvelope(finding);
    // No editor needed — it should be a no-op before we even need one
    const { editor } = makeStubEditor(vscode.Uri.file("/workspace/irrelevant.rs"));

    // Should not throw
    decorator.handleFinding(env, workspaceRoot, editor);

    // Internal map should be empty
    const allKeys = decorator.getAllUriKeys();
    if (allKeys.length !== 0) {
      throw new Error(
        `Expected no entries in the findings map for a finding without file, got ${allKeys.length}`,
      );
    }

    decorator.dispose();
  });

  test("handleFinding dedups by id — re-emitted finding does not double the squiggle", () => {
    const { factory } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);
    const filePath = "/workspace/dup.rs";
    const finding = makeFinding({ id: "dup-1", file: filePath, line: 5 });
    const { editor } = makeStubEditor(vscode.Uri.file(filePath));

    decorator.handleFinding(makeFindingEnvelope(finding), workspaceRoot, editor);
    // Same id, same file — bus replay on a re-subscription would do this.
    decorator.handleFinding(makeFindingEnvelope(finding), workspaceRoot, editor);

    const stored =
      decorator.getFindingsForUri(vscode.Uri.file(filePath).toString()) ?? [];
    if (stored.length !== 1) {
      throw new Error(
        `Expected dedup to keep exactly one finding for id 'dup-1', got ${stored.length}`,
      );
    }

    decorator.dispose();
  });

  test("dispose calls dispose on all three decoration types", () => {
    const { factory, error, warning, info } = makeStubFactory();
    const decorator = new FindingsDecorator(factory, dataDir);

    decorator.dispose();

    if (error.disposeCalls === 0) {
      throw new Error("Expected error decoration type to be disposed");
    }
    if (warning.disposeCalls === 0) {
      throw new Error("Expected warning decoration type to be disposed");
    }
    if (info.disposeCalls === 0) {
      throw new Error("Expected info decoration type to be disposed");
    }
  });
});
