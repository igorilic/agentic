import { render, screen } from "@testing-library/react";
import DiffViewer from "../components/DiffViewer";

const SAMPLE = `--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 fn answer() -> u32 {
-    41
+    42
 }
`;

describe("DiffViewer", () => {
  it("renders an empty placeholder for an empty diff", () => {
    render(<DiffViewer diff="" />);
    expect(screen.getByTestId("diff-viewer")).toBeInTheDocument();
    expect(screen.getByTestId("diff-empty")).toBeInTheDocument();
  });

  it("renders the file headers", () => {
    render(<DiffViewer diff={SAMPLE} />);
    const lines = screen.getAllByTestId(/^diff-line-/);
    expect(lines.length).toBeGreaterThanOrEqual(7);
    expect(screen.getByText("--- a/src/lib.rs")).toBeInTheDocument();
    expect(screen.getByText("+++ b/src/lib.rs")).toBeInTheDocument();
  });

  it("classifies add lines and styles them as additions", () => {
    render(<DiffViewer diff={SAMPLE} />);
    const adds = screen.getAllByTestId("diff-line-add");
    expect(adds).toHaveLength(1);
    // .textContent preserves whitespace; toHaveTextContent collapses it.
    expect(adds[0].textContent).toBe("+    42");
    expect(adds[0].className).toMatch(/text-green/);
  });

  it("classifies remove lines and styles them as removals", () => {
    render(<DiffViewer diff={SAMPLE} />);
    const removes = screen.getAllByTestId("diff-line-remove");
    expect(removes).toHaveLength(1);
    expect(removes[0].textContent).toBe("-    41");
    expect(removes[0].className).toMatch(/text-red/);
  });

  it("classifies hunk headers", () => {
    render(<DiffViewer diff={SAMPLE} />);
    const hunks = screen.getAllByTestId("diff-line-hunk");
    expect(hunks).toHaveLength(1);
    expect(hunks[0]).toHaveTextContent("@@ -1,3 +1,3 @@");
  });

  it("classifies file headers", () => {
    render(<DiffViewer diff={SAMPLE} />);
    const headers = screen.getAllByTestId("diff-line-file-header");
    expect(headers).toHaveLength(2);
  });

  it("classifies context lines", () => {
    render(<DiffViewer diff={SAMPLE} />);
    const ctx = screen.getAllByTestId("diff-line-context");
    // Two context rows in SAMPLE: ` fn answer()` and ` }`.
    expect(ctx).toHaveLength(2);
  });

  it("renders multi-file diffs as multiple file-header sections", () => {
    const multi = `--- a/foo.rs
+++ b/foo.rs
@@ -1 +1 @@
-old foo
+new foo
--- a/bar.rs
+++ b/bar.rs
@@ -1 +1 @@
-old bar
+new bar
`;
    render(<DiffViewer diff={multi} />);
    expect(screen.getAllByTestId("diff-line-file-header")).toHaveLength(4);
    expect(screen.getAllByTestId("diff-line-add")).toHaveLength(2);
    expect(screen.getAllByTestId("diff-line-remove")).toHaveLength(2);
  });

  it("treats `+++` and `---` as file headers, not add/remove rows", () => {
    // Template literal — JSX attribute strings don't process `\n`.
    render(<DiffViewer diff={`--- a/x\n+++ b/x\n`} />);
    expect(screen.getAllByTestId("diff-line-file-header")).toHaveLength(2);
    expect(screen.queryByTestId("diff-line-add")).toBeNull();
    expect(screen.queryByTestId("diff-line-remove")).toBeNull();
  });
});
