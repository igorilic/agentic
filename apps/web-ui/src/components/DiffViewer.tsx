// Step 13.2: in-house unified-diff viewer. Mirrors the TUI's
// `views::diff` parser so behaviour stays consistent across shells.
//
// Decision: skipped Monaco and `@git-diff-view/react`. Spec §25.3
// flags Monaco as ~3 MB gzipped — too heavy when this same bundle
// is embedded in the VS Code webview. `@git-diff-view/react` is
// lighter but adds a dep + learning surface for what is really a
// `<pre>` with classified lines. We can swap to either later by
// keeping the prop shape (`{ diff: string }`) stable.

export type DiffViewerProps = {
  diff: string;
};

type DiffLine =
  | { kind: "file-header"; text: string }
  | { kind: "hunk"; text: string }
  | { kind: "add"; text: string } // text includes the leading `+`
  | { kind: "remove"; text: string } // text includes the leading `-`
  | { kind: "context"; text: string };

function parseUnified(diff: string): DiffLine[] {
  if (diff.length === 0) return [];
  return diff.split("\n").flatMap((line, idx, arr): DiffLine[] => {
    // Drop the trailing empty entry produced by `split` on a string
    // that ends with `\n`. Other empty lines inside the diff are rare
    // but legal — keep them as context.
    if (line === "" && idx === arr.length - 1) return [];
    if (line.startsWith("--- ") || line.startsWith("+++ ")) {
      return [{ kind: "file-header", text: line }];
    }
    if (line.startsWith("@@")) {
      return [{ kind: "hunk", text: line }];
    }
    if (line.startsWith("+")) return [{ kind: "add", text: line }];
    if (line.startsWith("-")) return [{ kind: "remove", text: line }];
    return [{ kind: "context", text: line }];
  });
}

const STYLES: Record<DiffLine["kind"], string> = {
  "file-header": "text-cyan-700 font-semibold",
  hunk: "text-purple-700",
  add: "text-green-700 bg-green-50",
  remove: "text-red-700 bg-red-50",
  context: "text-gray-600",
};

export default function DiffViewer({ diff }: DiffViewerProps) {
  const lines = parseUnified(diff);

  if (lines.length === 0) {
    return (
      <section
        data-testid="diff-viewer"
        className="border border-gray-200 rounded p-4 text-sm text-gray-500 italic"
      >
        <span data-testid="diff-empty">No diff to display.</span>
      </section>
    );
  }

  return (
    <section
      data-testid="diff-viewer"
      className="border border-gray-200 rounded overflow-hidden font-mono text-xs"
    >
      <pre className="m-0 p-0 overflow-x-auto">
        {lines.map((line, idx) => (
          <div
            key={idx}
            data-testid={`diff-line-${line.kind}`}
            className={`px-3 py-0.5 whitespace-pre ${STYLES[line.kind]}`}
          >
            {line.text || " "}
          </div>
        ))}
      </pre>
    </section>
  );
}
