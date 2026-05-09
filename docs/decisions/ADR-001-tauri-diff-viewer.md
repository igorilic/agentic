# ADR-001: In-house DiffViewer for unified-diff text

- **Status**: Accepted
- **Date**: 2026-04-28 (back-filled 2026-05-09)
- **Source commit**: `9fedb98` (Step 13.2)
- **Spec reference**: §20.2, §25.3
- **Closes**: #78

## Context

Step 13.2 of the redesign needed a way to render unified-diff text in the
Tauri web UI. Spec §25.3 listed three candidates:

1. **Monaco** — full-featured editor with built-in diff view (~3 MB gzipped).
2. **`@git-diff-view/react`** — focused diff component, smaller footprint
   but adds an external dep and learning surface.
3. **In-house `DiffViewer`** — parse `--- / +++ / @@ / + / -` prefixes
   ourselves and render in a Tailwind-styled `<pre>`.

The diff text is a simple, well-specified format. The shape of the input
(`{ diff: string }`) is small and stable.

## Decision

Ship an **in-house `DiffViewer`** that parses unified-diff prefixes into
typed lines and renders each in a Tailwind-styled `<pre>` with
green / red / cyan / purple coloring, matching the TUI's `views::diff`
for cross-shell visual consistency.

## Consequences

### Positive

- Bundle size: zero new external deps; the renderer is ~100 lines of
  TypeScript.
- Visual consistency with the TUI's diff renderer — same color semantics
  across both shells.
- Stable prop shape (`{ diff: string }`) means we can swap the
  implementation later without touching any callers.

### Negative

- No syntax highlighting inside `+`/`-` lines (would need `syntect` /
  `highlight.js` / similar). Tracked as a future upgrade.
- Manual parser maintenance — though the unified-diff format is stable
  and changes are unlikely.

### Original size context (now partially moot)

At decision time, the same React bundle was expected to ship in a
VS Code webview, where Monaco's ~3 MB cost was especially painful.
**Note (back-fill 2026-05-09)**: VS Code support has since been abandoned
(see closed VS Code tech-debt issues), so the VS Code-webview argument
no longer applies. The size, dep-surface, and visual-consistency
arguments remain.

## Alternatives rejected

- **Monaco** — too heavy for what is essentially a classified `<pre>`.
- **`@git-diff-view/react`** — adds dep + learning surface for marginal
  gain over the in-house renderer.

## References

- Source commit body: `9fedb98 feat(web): in-house DiffViewer for unified-diff text (Step 13.2)`
- Spec: `docs/redesign/spec.md` §20.2, §25.3
- TUI counterpart: `crates/agentic-tui/src/views/diff.rs`
- Web component: `apps/web-ui/src/components/DiffViewer.tsx`
- TUI syntax-highlighting follow-up: GH #76 (deferred — distribution-driven)
