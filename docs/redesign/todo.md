# Agentic UI Redesign — Implementation Todos

Source spec: `docs/redesign/spec.md`
Generated: 2026-04-29

Execution contract:
- One step per `tdd-developer` invocation.
- Each step = single TDD cycle (RED → GREEN → REFACTOR → commit), targeted at ~30–90 min of focused work.
- Steps are linear within a phase; respect explicit `Depends on` notes across phases.
- Stack invariants (do not re-decide per step):
  - Web: React 18, TS, Vite, Tailwind 3.x, Vitest, Testing Library, **no new dependencies** unless flagged in the step's Notes block.
  - Tauri: Tauri 2.x; no IPC changes.
  - TUI: Rust 2024, ratatui, crossterm; existing `apply_envelope` / `handle_key` contract preserved.
- Commit style: Conventional Commits, body explains "why", trailers include `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>` per repo convention.
- After each step: spawn `qa` + `reviewer` in parallel per `CLAUDE.md` rule 3.

Legend:
- Step IDs: `W.x.y` = web, `T.x.y` = TUI, `X.x.y` = Tauri / cross-cutting.
- Crate shorthand: `web` = `apps/web-ui`, `tui` = `crates/agentic-tui`, `tauri` = `crates/agentic-tauri`.

---

## Phase 0 — Tokens & foundation

Sets up the design-token plumbing and theme primitives used by every subsequent web step. Land all of Phase 0 before Phase 1 — without tokens, the new components have no colors to bind to.

### [x] Step W.0.1: Wire Inter via Google Fonts CDN + token CSS file

**Goal**: Pull `Inter` from Google Fonts (no committed `.ttf`) and ship a
CSS file that exposes every token from `colors_and_type.css` as a CSS
variable, ready to be consumed by Tailwind and direct CSS.

**Depends on**: none.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/inter-font.test.ts`:
  - Reads `apps/web-ui/index.html` from disk via `fs.readFileSync`.
  - Asserts the `<head>` contains a `<link rel="preconnect" href="https://fonts.googleapis.com">` tag.
  - Asserts the `<head>` contains a `<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>` tag.
  - Asserts the `<head>` contains a `<link rel="stylesheet">` whose `href`
    starts with `https://fonts.googleapis.com/css2?family=Inter` and
    contains `display=swap`.
- New test `apps/web-ui/src/__tests__/tokens.test.ts`:
  - Imports `apps/web-ui/src/styles/tokens.css` as a raw string (`?raw`).
  - Asserts the file does **not** contain `@font-face` (font is loaded via
    CDN `<link>`, not declared inline).
  - Asserts `--font-sans` value contains `Inter`.
  - Asserts `:root` block defines all of: `--bg-page`, `--bg-surface`, `--bg-surface-2`, `--fg`, `--fg-muted`, `--fg-subtle`, `--border-soft`, `--border`, `--border-strong`, `--font-sans`, `--font-mono`, `--radius-md`, `--radius-lg`, `--radius-xl`, `--shadow-card`, `--shadow-popover`, `--shadow-modal`.
  - Asserts a `:root[data-theme="dark"]` block redefines `--bg-page`, `--bg-surface`, `--fg`, `--fg-muted`.

**Implement** (GREEN):
- Edit `apps/web-ui/index.html`: add the two `<link rel="preconnect">`
  tags and the Google Fonts stylesheet `<link>` per spec §6.2.
- Create `apps/web-ui/src/styles/tokens.css` with the full token list per
  spec §6.1. **Do not** commit a `.ttf` asset and **do not** emit a
  `@font-face` block — Inter resolves through the Google Fonts CDN
  stylesheet; `--font-sans` simply lists `Inter` as the first family with
  the existing fallback stack.
- Import `./styles/tokens.css` from `apps/web-ui/src/index.css` (above the
  `@tailwind` directives so tokens are available everywhere).

**Refactor**: None.

**Commit**: `feat(web): add design tokens and Inter via Google Fonts CDN`

**Verification**: `pnpm -F @agentic/web-ui test inter-font tokens`

**Notes**: No new asset and no new npm package. The only network dependency
is the Google Fonts CDN; it gracefully falls back to the system stack if
blocked.

---

### [x] Step W.0.2: Extend `tailwind.config.js` with semantic color + token aliases

**Goal**: Tailwind utilities like `bg-bg-surface`, `text-fg-muted`, `border-border` resolve to the CSS variables from W.0.1.

**Depends on**: W.0.1.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/tailwindTokens.test.ts`:
  - Imports the Tailwind config (`import config from "../../tailwind.config.js"`).
  - Asserts `config.theme.extend.colors` contains keys `bg-page`, `bg-surface`, `bg-surface-2`, `fg`, `fg-muted`, `fg-subtle`, `border-soft`, `border-strong`, `status-done`, `status-active`, `status-queued`, `status-failed`, `status-info`, `agent-architect`, `agent-developer`, `agent-qa`, `agent-reviewer`.
  - Asserts each value is a `var(--…)` reference string.
  - Asserts `config.theme.extend.fontFamily.sans[0]` is `"Inter"`.
  - Asserts `config.theme.extend.boxShadow` contains keys `card`, `popover`, `modal`.

**Implement** (GREEN):
- Edit `apps/web-ui/tailwind.config.js`. Replace the empty `extend: {}` with the full extension matching the test assertions. All values reference `var(--…)`.

**Refactor**: None.

**Commit**: `feat(web): extend tailwind theme with design tokens`

**Verification**: `pnpm -F @agentic/web-ui test tailwindTokens`

---

### [x] Step W.0.3: Add `useTheme` hook

**Goal**: Hook that reads `localStorage["agentic.theme"]`, sets `data-theme` on `<html>`, returns `(theme, setTheme, toggle)`. Persists across reloads.

**Depends on**: W.0.1.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/useTheme.test.ts`:
  - Renders `useTheme` via `renderHook`.
  - Asserts default theme is `"light"` when localStorage and `prefers-color-scheme` are unset (jsdom defaults).
  - Asserts calling `toggle()` flips to `"dark"` and writes `localStorage["agentic.theme"] = "dark"`.
  - Asserts `document.documentElement.getAttribute("data-theme")` is `"dark"` after toggle.
  - Asserts a fresh hook instance reads back `"dark"` from localStorage.
  - Asserts setTheme("light") removes (or sets) `data-theme` accordingly.

**Implement** (GREEN):
- Create `apps/web-ui/src/hooks/useTheme.ts`:
  - State init from `localStorage.getItem("agentic.theme")`.
  - `useEffect` writes `data-theme` attribute on `document.documentElement`.
  - Returns `{ theme, setTheme, toggle }`.

**Refactor**: None.

**Commit**: `feat(web): add useTheme hook with localStorage persistence`

**Verification**: `pnpm -F @agentic/web-ui test useTheme`

---

### [x] Step W.0.4: Add `pipeline.ts` types module

**Goal**: Centralize the new state shapes (`AgentInstance`, `PermissionRequest`, `ActionItem`, `IssueTicket`) per spec §6.4 so subsequent steps import from one place.

**Depends on**: none.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/pipelineTypes.test.ts`:
  - Imports the new module and uses TypeScript type-level assertions (`expectTypeOf` from `vitest`) to verify each interface has the required fields.
  - Runtime assertions: helper `agentInstanceFromStep(stepInfo)` adapts an existing `StepInfo` to `AgentInstance`. Test 4 input statuses (`pending → queued`, `running → active`, `passed → done`, `failed → failed`).

**Implement** (GREEN):
- Create `apps/web-ui/src/types/pipeline.ts` with the interfaces per spec §6.4 and the `agentInstanceFromStep` adapter.

**Refactor**: None.

**Commit**: `feat(web): add pipeline types module`

**Verification**: `pnpm -F @agentic/web-ui test pipelineTypes`

---

## Phase 1 — Web header + run-state badge

### [x] Step W.1.1: New `HeaderBar` component (idle state)

**Goal**: Render the 48 px header bar with brand, slug, settings/theme/avatar, and an idle "Run pipeline" button. No real run state yet — just the chrome.

**Depends on**: W.0.2, W.0.3.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/HeaderBar.test.tsx`:
  - Renders `<HeaderBar brand="Agentic" ticketSlug={null} runState="idle" theme="light" onThemeToggle={fn} ... />`.
  - Asserts `[data-testid="header-bar"]` is in the document with computed height `48px`.
  - Asserts brand text "Agentic" and absence of slug when `ticketSlug` is null.
  - Asserts a primary button "Run pipeline" exists (`data-testid="header-run"`).
  - Asserts a button `[data-testid="header-theme-toggle"]` with `aria-pressed="false"`.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/HeaderBar.tsx`. Use Tailwind tokens (`bg-bg-surface`, `border-border-soft`, `text-fg`, etc.).
- Brand tile is a 26 × 26 black rounded-square with the diamond SVG glyph from spec §3.1.

**Refactor**: None.

**Commit**: `feat(web): add HeaderBar component with idle state`

**Verification**: `pnpm -F @agentic/web-ui test HeaderBar`

---

### [x] Step W.1.2: HeaderBar — running and completed badges

**Goal**: Add the two non-idle pill variants per spec §3.1, with elapsed-time formatting and Stop / Re-run buttons.

**Depends on**: W.1.1.

**Test first** (RED):
- Extend `HeaderBar.test.tsx`:
  - When `runState="running"`, `elapsedMs={154000}`, asserts a pill with text matching `/Pipeline running · 02:34/` and a Stop button (`data-testid="header-stop"`).
  - When `runState="completed"`, `elapsedMs={258000}`, asserts a pill with text matching `/Completed · 04:18/` and a Re-run button (`data-testid="header-rerun"`).
  - Click Stop fires `onStopRun`; click Re-run fires `onRerun`.

**Implement** (GREEN):
- Extend `HeaderBar.tsx` to switch chrome by `runState`.
- Reuse `formatElapsed` shape from `ActiveRunIndicator.tsx` but expressed as `MM:SS`.

**Refactor**: Extract `RunStateBadge` into a sibling component if branching grows.

**Commit**: `feat(web): add running and completed run-state badges to HeaderBar`

**Verification**: `pnpm -F @agentic/web-ui test HeaderBar`

---

### [x] Step W.1.3: HeaderBar — theme toggle wires `useTheme`

**Goal**: Click the theme toggle and the document attribute flips. Existing tests in W.0.3 verified the hook; this step verifies integration.

**Depends on**: W.0.3, W.1.1.

**Test first** (RED):
- Extend `HeaderBar.test.tsx`:
  - Mount in jsdom. Click `[data-testid="header-theme-toggle"]`.
  - Assert `document.documentElement.getAttribute("data-theme") === "dark"`.
  - Click again, assert it flips back to `light`.
  - Assert `aria-pressed` reflects state.

**Implement** (GREEN):
- Wire `useTheme()` inside `HeaderBar.tsx` (or accept `onThemeToggle` prop and let the App provide it; pick the simpler path — internal hook).

**Refactor**: None.

**Commit**: `feat(web): wire HeaderBar theme toggle to useTheme hook`

**Verification**: `pnpm -F @agentic/web-ui test HeaderBar`

---

## Phase 2 — Web pipeline bar

### [x] Step W.2.1: `AgentCard` component

**Goal**: Render one agent card with avatar, name, status pill, and the kebab menu placeholder. Pure presentation.

**Depends on**: W.0.2, W.0.4.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/AgentCard.test.tsx`:
  - Renders three variants: `status="queued" | "active" | "done"`, asserts:
    - Card has `data-testid="agent-card-{agent}"` and `data-status` attribute.
    - The active variant has class containing `border-status-active` (or equivalent) and a pulse-animated indicator.
    - The kebab button `[data-testid="agent-card-{agent}-menu"]` is present.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/AgentCard.tsx`.
- Status → ring color uses the new `status-*` Tailwind tokens (W.0.2).
- Avatar tile is a 44 × 44 rounded-square with the agent's accent bg and an inline SVG icon (initially a placeholder rect; per-agent SVG ships in W.7.x).

**Refactor**: None.

**Commit**: `feat(web): add AgentCard component`

**Verification**: `pnpm -F @agentic/web-ui test AgentCard`

---

### [x] Step W.2.2: `Connector` between agent cards

**Goal**: Render a horizontal line + chevron between cards. Active hand-off uses an animated dashed line.

**Depends on**: W.2.1.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/Connector.test.tsx`:
  - Renders `<Connector active={false} />` and asserts a static `[data-testid="connector"][data-active="false"]`.
  - Renders `<Connector active={true} />`, asserts `data-active="true"` and presence of a class indicating dashed/animated.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/Connector.tsx`. SVG arrow + 1 px line.

**Refactor**: None.

**Commit**: `feat(web): add Connector component for pipeline bar`

**Verification**: `pnpm -F @agentic/web-ui test Connector`

---

### Step W.2.3: `PipelineBar` shell — render cards + connectors

**Goal**: Compose `AgentCard` + `Connector` for a fixed agents prop, with the trailing dashed `+ Add agent` end cap.

**Depends on**: W.2.1, W.2.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/PipelineBar.test.tsx`:
  - Renders `<PipelineBar agents={["architect","developer","qa","reviewer"]} statuses={{architect:"done", developer:"active", qa:"queued", reviewer:"queued"}} activeIndex={1} />`.
  - Asserts 4 `agent-card-*` testids in order.
  - Asserts 3 `connector` testids interleaved (one between each adjacent pair).
  - Asserts the connector at index 0 has `data-active="false"`, the one at index 1 (between developer and qa) has `data-active="false"` (active means *current* hand-off, only set after StepComplete; can be all false here).
  - Asserts a `[data-testid="pipeline-add-agent"]` end cap button exists.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/PipelineBar.tsx`.
- Map `agents` to `AgentCard` + `Connector` pairs; append `+ Add agent` button.

**Refactor**: None.

**Commit**: `feat(web): add PipelineBar shell with cards and connectors`

**Verification**: `pnpm -F @agentic/web-ui test PipelineBar`

---

### [x] Step W.2.4: Insert `+` chip between cards

**Goal**: Add the 16 × 16 `+` chip in each gap. Click invokes `onInsert(atIndex)`. Hover affordance is opacity-based.

**Depends on**: W.2.3.

**Test first** (RED):
- Extend `PipelineBar.test.tsx`:
  - Asserts 3 `[data-testid^="pipeline-insert-"]` chips for a 4-agent pipeline.
  - Click `pipeline-insert-1` fires `onInsert` with `atIndex === 1`.
  - Click `pipeline-add-agent` fires `onInsert` with `atIndex === 4` (i.e. end).

**Implement** (GREEN):
- Render an absolutely-positioned `+` button inside each `Connector` slot.
- Wire `onInsert` callback.

**Refactor**: None.

**Commit**: `feat(web): add insert chips to PipelineBar`

**Verification**: `pnpm -F @agentic/web-ui test PipelineBar`

---

### [x] Step W.2.5: `AgentPicker` popover (search + select)

**Goal**: Standalone popover with search input and a scrollable list of agents not already in the pipeline.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/AgentPicker.test.tsx`:
  - Renders `<AgentPicker excludeIds={["architect","developer"]} onPick={fn} onClose={fn} />`.
  - Asserts the search input has placeholder `Search agents…`.
  - Asserts excluded agents are not in the list.
  - Type `qa` into the input; only the QA row visible.
  - Click QA row; `onPick` called with `"qa"`.
  - Press Escape; `onClose` called.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/AgentPicker.tsx`.
- Hardcode the 12 agents from spec §3.3 (matches `data.js`) into a constant `AGENT_LIBRARY` exported from `apps/web-ui/src/types/pipeline.ts`.
- Outside-click handler optional this step (covered in W.2.6).

**Refactor**: None.

**Commit**: `feat(web): add AgentPicker popover with search`

**Verification**: `pnpm -F @agentic/web-ui test AgentPicker`

---

### Step W.2.6: Wire AgentPicker into PipelineBar insert flow

**Goal**: Click `+` chip → picker opens anchored to that gap; selecting an agent calls `onInsert(atIndex, agentId)`. Click outside dismisses.

**Depends on**: W.2.4, W.2.5.

**Test first** (RED):
- Extend `PipelineBar.test.tsx`:
  - Click `pipeline-insert-2`. Assert AgentPicker is in the document.
  - Type `qa`, click QA. Assert `onInsert` called with `(2, "qa")`.
  - Click `pipeline-insert-2` again, then click outside. Assert picker closed (no AgentPicker in DOM).

**Implement** (GREEN):
- `PipelineBar` holds `pickerOpenAt: number | "end" | null` state.
- Render a single `AgentPicker` anchored to the open position.
- Outside-click via `mousedown` + ref check (mirrors prototype).

**Refactor**: None.

**Commit**: `feat(web): wire AgentPicker into PipelineBar insert flow`

**Verification**: `pnpm -F @agentic/web-ui test PipelineBar`

---

### Step W.2.7: PipelineBar drag-reorder

**Goal**: Dragging a card across gaps reorders the `agents` array via `onReorder(fromIndex, toIndex)`. Drop indicator is a 2 px vertical accent bar at the drop position.

**Depends on**: W.2.3.

**Test first** (RED):
- Extend `PipelineBar.test.tsx`:
  - `fireEvent.dragStart` on the architect card.
  - `fireEvent.dragOver` on the gap before qa.
  - Assert the gap node has `data-drop-active="true"`.
  - `fireEvent.drop` on that gap. Assert `onReorder` called with `(0, 2)` (or matching indices per the implementation contract — document and assert accordingly).

**Implement** (GREEN):
- Add HTML5 DnD handlers to `AgentCard` (drag source) and to the gap elements between cards (drop targets) per spec §6.5. No new dependency.
- Compute `toIndex` from gap index, accounting for whether `fromIndex < toIndex` (subtract 1 if so).

**Refactor**: Extract `useDragReorder` hook if the body grows past ~40 lines.

**Commit**: `feat(web): add drag-reorder to PipelineBar`

**Verification**: `pnpm -F @agentic/web-ui test PipelineBar`

---

### [x] Step W.2.8: AgentCard kebab menu (Remove / Skip / Configure)

**Goal**: Click the kebab opens a dropdown with three items. Remove and Skip fire callbacks. Configure is a no-op (placeholder modal opens and closes).

**Depends on**: W.2.1.

**Test first** (RED):
- Extend `AgentCard.test.tsx`:
  - Click kebab; assert menu items "Remove", "Skip this run", "Configure…" visible.
  - Click "Remove"; assert `onRemove` called.
  - Click "Skip this run"; assert `onSkip` called.
  - Click "Configure…"; assert a modal `[data-testid="agent-configure-modal"]` opens; click backdrop, assert it closes.

**Implement** (GREEN):
- Add menu state + click handler in `AgentCard.tsx`.
- The configure modal is a minimal placeholder — header "Configure agent — not yet implemented", close button.

**Refactor**: None.

**Commit**: `feat(web): add AgentCard kebab menu with Remove/Skip/Configure placeholder`

**Verification**: `pnpm -F @agentic/web-ui test AgentCard`

---

## Phase 3 — Web 3-column shell

### [x] Step W.3.1: `AppShell` component with grid layout

**Goal**: New top-level layout component: `HeaderBar` + `PipelineBar` + 3-column grid (`1fr 1fr 340px`). Accepts `dense: boolean` to flip the right column to 280 px.

**Depends on**: W.1.1, W.2.3.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/AppShell.test.tsx`:
  - Renders `<AppShell dense={false}><div data-testid="col-1" /><div data-testid="col-2" /><div data-testid="col-3" /></AppShell>`.
  - Asserts the three slot children are present.
  - Asserts the grid container has class `grid` and `grid-cols-[1fr_1fr_340px]` (or equivalent inline style).
  - When `dense={true}`, asserts the third column resolves to `280px` width.
  - Asserts header and pipeline bar regions exist (`[data-testid="app-shell-header"]`, `[data-testid="app-shell-pipeline"]`).

**Implement** (GREEN):
- Create `apps/web-ui/src/components/AppShell.tsx`. Accepts `header`, `pipelineBar`, and three column children (or named slots).
- Use arbitrary Tailwind variants for the grid template, switched by `dense`.

**Refactor**: None.

**Commit**: `feat(web): add AppShell with header + pipeline + 3-column grid`

**Verification**: `pnpm -F @agentic/web-ui test AppShell`

---

### [x] Step W.3.2: Tauri-dense detection helper

**Goal**: Pure helper `isTauriDense()` that returns true when `window.__TAURI_INTERNALS__` exists or `import.meta.env.TAURI === "1"`.

**Depends on**: none.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/isTauriDense.test.ts`:
  - When neither flag is set, returns `false`.
  - When `window.__TAURI_INTERNALS__` is truthy, returns `true`.
  - When `import.meta.env.TAURI === "1"` (mocked via `vi.stubEnv`), returns `true`.

**Implement** (GREEN):
- Create `apps/web-ui/src/utils/isTauriDense.ts`.

**Refactor**: None.

**Commit**: `feat(web): add isTauriDense detection helper`

**Verification**: `pnpm -F @agentic/web-ui test isTauriDense`

---

## Phase 4 — Web Chat column

### Step W.4.1: `ChatMessage` component variants

**Goal**: Pure presentation for one message in three variants (`user`, `system`, `agent`). Renders the visual contract of spec §3.4.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/ChatMessage.test.tsx`:
  - User variant: avatar placeholder + "Erica" + timestamp + body. `[data-testid="chat-message-user"]`.
  - System variant: centered text without bubble. `[data-testid="chat-message-system"]`. Assert the formatted hand-off text matches `/── .* ──/`.
  - Agent variant: agent name in agent color, body bubble has 3 px left border accent. `[data-testid="chat-message-agent"][data-agent="architect"]`.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/ChatMessage.tsx`.
- Per-agent accent map driven by Tailwind tokens (W.0.2).

**Refactor**: None.

**Commit**: `feat(web): add ChatMessage component with user/system/agent variants`

**Verification**: `pnpm -F @agentic/web-ui test ChatMessage`

---

### Step W.4.2: `ChatMessage` inline token highlighter ✓

**Goal**: Slash commands and `@mentions` inside message bodies render as highlighted tokens (light yellow bg, 2 px radius).

**Depends on**: W.4.1.

**Test first** (RED):
- Extend `ChatMessage.test.tsx`:
  - Render a user message with text `"/develop AGT-204 @architect please"`.
  - Assert `[data-testid="chat-token"]` appears 2 times (one for `/develop`, one for `@architect`).
  - Assert the rest of the text is regular spans.

**Implement** (GREEN):
- Add a `renderInline(text)` helper inside `ChatMessage.tsx` that splits on the regex `/(\/[a-z]+|@[a-z]+)/g` and wraps matches in `<span data-testid="chat-token">`.

**Refactor**: None.

**Commit**: `feat(web): highlight slash and mention tokens in ChatMessage`

**Verification**: `pnpm -F @agentic/web-ui test ChatMessage`

---

### Step W.4.3: `ChatComposer` — textarea + quick-pick chips + send

**Goal**: Composer chrome (chips + 1-row textarea + send button) without slash/mention popovers yet. Submit fires `onSend(text)`.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/ChatComposer.test.tsx`:
  - Renders `<ChatComposer onSend={fn} />`.
  - Asserts 4 chips: `Plan`, `Brainstorm`, `Develop`, `Spec`.
  - Click `Plan`. Asserts the textarea now contains `"/plan "` and is focused.
  - Type `hello`. Click send. Asserts `onSend("/plan hello")`.
  - Press Cmd+Enter (jsdom: `metaKey: true, key: "Enter"`). Asserts `onSend` fires.
  - Press Enter alone. Asserts a newline is inserted (textarea value contains `\n`) and `onSend` does not fire.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/ChatComposer.tsx`.
- Cmd/Ctrl+Enter sends; Enter alone inserts newline (matches spec §3.4 — behavior change vs. today's ChatPane).

**Refactor**: None.

**Commit**: `feat(web): add ChatComposer with quick-pick chips and Cmd+Enter submit`

**Verification**: `pnpm -F @agentic/web-ui test ChatComposer`

**Notes**: This is a behavior change (today: Enter sends). Existing `ChatPane.test.tsx` will break; re-align in W.4.6.

---

### Step W.4.4: Slash popover inside ChatComposer

**Goal**: Typing `/` opens a popover above the textarea with matching commands. Arrow keys navigate, Enter inserts, Esc closes.

**Depends on**: W.4.3.

**Test first** (RED):
- Extend `ChatComposer.test.tsx`:
  - Type `/`. Assert `[data-testid="slash-popover"]` is in the document.
  - Type `pl`. Assert only `/plan` matches.
  - Press ArrowDown then Enter. Assert textarea contains `/plan `.
  - Press Esc. Assert popover dismissed.

**Implement** (GREEN):
- Reuse `parseSlashCommand` from `apps/web-ui/src/slash/parser.ts` for matching prefixes.
- Show popover when draft starts with `/` and contains no spaces.

**Refactor**: None.

**Commit**: `feat(web): add slash command popover to ChatComposer`

**Verification**: `pnpm -F @agentic/web-ui test ChatComposer`

---

### Step W.4.5: Mention popover inside ChatComposer

**Goal**: Typing `@` opens a 240 px agent-picker-shaped popover. Selecting inserts `@<agent> `.

**Depends on**: W.4.3, W.2.5 (AgentPicker).

**Test first** (RED):
- Extend `ChatComposer.test.tsx`:
  - Type `hi @ar`. Assert `[data-testid="mention-popover"]` is in the document.
  - Click `architect` row. Assert textarea contains `hi @architect `.
  - Asserts `parseMention(...)` from `mention/parser.ts` is exercised (assert the popover passes the trailing query through).

**Implement** (GREEN):
- Reuse `AgentPicker` styled at 240 px width.
- Trigger when last `@` follows a space or is at position 0.

**Refactor**: None.

**Commit**: `feat(web): add mention popover to ChatComposer`

**Verification**: `pnpm -F @agentic/web-ui test ChatComposer`

---

### Step W.4.6: `ChatColumn` integrates new ChatPane behavior

**Goal**: New `ChatColumn` component composes header + scrollable message list + `ChatComposer`. Replaces the body of today's `ChatPane`. Existing `ChatPane.test.tsx` is updated to match.

**Depends on**: W.4.1, W.4.2, W.4.3, W.4.4, W.4.5.

**Test first** (RED):
- Update `apps/web-ui/src/__tests__/ChatPane.test.tsx`:
  - Existing assertions on `data-testid="chat-pane"`, `chat-form`, `chat-input`, `chat-send` stay.
  - Update the "Enter sends" assertion to "Cmd+Enter sends" (the Enter-sends behavior is dropped per spec §3.4).
  - Add: when a message has agent role `architect`, asserts `[data-testid="chat-message-agent"][data-agent="architect"]` renders.
  - Add: typing `/` opens slash popover.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/ChatColumn.tsx`. Header (with active-agent chip), scrollable message list, sticky composer.
- Update `ChatPane.tsx` to delegate its body to `ChatColumn`, keeping the same outer `data-testid="chat-pane"` and prop signature.

**Refactor**: Extract chip + active-agent indicator into `ChatColumnHeader.tsx` if the file grows.

**Commit**: `feat(web): rewrite ChatPane body as ChatColumn with new design`

**Verification**: `pnpm -F @agentic/web-ui test ChatPane ChatColumn`

---

## Phase 5 — Web Activity column

### [x] Step W.5.1: `ActivityHeader` with tab strip

**Goal**: Header that shows the title and four tabs (`All`, `Tool calls`, `Permissions`, `Errors`) with per-tab count chips.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/ActivityHeader.test.tsx`:
  - Renders with `counts={all: 12, tool: 3, perm: 1, error: 0}` and `filter="all"`.
  - Asserts 4 tab buttons exist.
  - Asserts the count chip next to each tab matches.
  - Click "Tool calls"; assert `onFilterChange("tool")` fires.
  - Asserts active tab has `aria-selected="true"`.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/ActivityHeader.tsx`. ARIA `role="tablist"` + `role="tab"` per item.

**Refactor**: None.

**Commit**: `feat(web): add ActivityHeader with tab strip and counts`

**Verification**: `pnpm -F @agentic/web-ui test ActivityHeader`

---

### [x] Step W.5.2: `LogRow` (info / status variant)

**Goal**: Pure component for a single info or status log row — `[HH:MM:SS]` + agent in agent color + message body.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/LogRow.test.tsx`:
  - Renders an info row for `architect`. Asserts timestamp, agent name, message body present, agent name in `text-agent-architect` (or equivalent class binding to the per-agent token).
  - Renders an error row. Asserts the level chip is red.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/LogRow.tsx`.

**Refactor**: None.

**Commit**: `feat(web): add LogRow component for activity log`

**Verification**: `pnpm -F @agentic/web-ui test LogRow`

---

### Step W.5.3: `ToolCallCard` (collapsible body) ✓

**Goal**: Bordered card with header row (agent + tool + result chip) and a collapsible detail body for stdout/stderr.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/ToolCallCard.test.tsx`:
  - Renders with `tool="read_file"`, `arg="/src/api.ts"`, `result="OK"`. Asserts `result-chip-ok`.
  - When `details` prop is present, asserts a toggle button. Click expands; assert details visible. Click again collapses.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/ToolCallCard.tsx`.

**Refactor**: None.

**Commit**: `feat(web): add ToolCallCard with collapsible details`

**Verification**: `pnpm -F @agentic/web-ui test ToolCallCard`

---

### [x] Step W.5.4: `ActivityColumn` composes header + filtered log

**Goal**: Replaces today's `EventList`. Reads `events: EventEnvelope[]`, applies the tab filter, dispatches per-row to `LogRow` / `ToolCallCard`.

**Depends on**: W.5.1, W.5.2, W.5.3.

**Test first** (RED):
- Update `apps/web-ui/src/__tests__/EventList.test.tsx` (rename to `ActivityColumn.test.tsx` if scope permits, but **keep** the existing `data-testid="event-list"` on the inner UL element — see compat rules):
  - With 5 events spanning info/tool/error: assert all visible under `All` tab.
  - Switch to `Tool calls`; assert only the tool events visible.
  - Switch to `Errors`; assert only the error rows.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/ActivityColumn.tsx`. Adapter from existing `EventEnvelope` types to `LogRow` / `ToolCallCard` props.
- Delete `apps/web-ui/src/components/EventList.tsx` once nothing imports it. (Update `App.tsx` in W.7.x.)

**Refactor**: None.

**Commit**: `feat(web): rewrite EventList as ActivityColumn with tab filter`

**Verification**: `pnpm -F @agentic/web-ui test ActivityColumn`

---

## Phase 6 — Web Issue column

### [x] Step W.6.1: `IssueColumn` shell — id, title, labels, description

**Goal**: New component renders the static issue header strip. No action items yet.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/IssueColumn.test.tsx`:
  - Renders with a fixture ticket (`{id: "AGT-204", title: "...", labels: ["backend","api"], body: ["para 1","para 2"], acceptance: ["a1","a2"]}`).
  - Asserts the id, title, label chips, description paragraphs, acceptance checklist all visible.
  - Asserts each label has its own `[data-testid^="issue-label-"]`.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/IssueColumn.tsx`.

**Refactor**: None.

**Commit**: `feat(web): add IssueColumn with id, title, labels, description`

**Verification**: `pnpm -F @agentic/web-ui test IssueColumn`

---

### Step W.6.2: IssueColumn — acceptance checklist with completed state

**Goal**: When `runState="completed"`, mark each acceptance item as done (filled checkbox glyph).

**Depends on**: W.6.1.

**Test first** (RED):
- Extend `IssueColumn.test.tsx`:
  - Render with `runState="running"`. Assert all checkboxes have `data-checked="false"`.
  - Render with `runState="completed"`. Assert all checkboxes have `data-checked="true"`.

**Implement** (GREEN):
- Drive checkbox state from `runState` (matching the prototype's logic).

**Refactor**: None.

**Commit**: `feat(web): mark acceptance items done when run completes`

**Verification**: `pnpm -F @agentic/web-ui test IssueColumn`

---

### Step W.6.3: IssueColumn — Action items section

**Goal**: When `runState="completed"` and `actionItems.length > 0`, render the "Action items" heading + per-item rows.

**Depends on**: W.6.2.

**Test first** (RED):
- Extend `IssueColumn.test.tsx`:
  - Render with `runState="completed"` and 3 action items. Assert each row visible with `[data-testid="action-item-{id}"]`, status icon (`✓` / `⚠` / `↗`), title, description.
  - Render with `runState="running"`. Assert no action items section in DOM.

**Implement** (GREEN):
- Extend `IssueColumn.tsx`.

**Refactor**: None.

**Commit**: `feat(web): add Action items section to IssueColumn`

**Verification**: `pnpm -F @agentic/web-ui test IssueColumn`

---

### [x] Step W.6.4: IssueColumn — derive action items from findings

**Goal**: Adapter that maps `Finding[]` → `ActionItem[]` so the existing findings stream populates the new section. Keeps `FindingsTable` triage logic alive (W.7.x decides what to do with the standalone table).

**Depends on**: W.6.3, W.0.4.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/findingsToActionItems.test.ts`:
  - Given 3 findings (one error, one warning, one info), returns 3 `ActionItem`s with `kind` mapped (`error → warning`, `warning → followup`, `info → issue`).
  - Findings already triaged (`triage !== null`) are filtered out.

**Implement** (GREEN):
- Create `apps/web-ui/src/utils/findingsToActionItems.ts`.

**Refactor**: None.

**Commit**: `feat(web): add findingsToActionItems adapter`

**Verification**: `pnpm -F @agentic/web-ui test findingsToActionItems`

---

### Step W.6.5: `SpecDialog` modal

**Goal**: Modal with title input + body textarea + Cancel / Create & run buttons. Disabled state on submit when title empty.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/SpecDialog.test.tsx`:
  - Renders with `open={true}`. Asserts title input + textarea + 2 buttons.
  - The submit button is disabled when title is empty.
  - Type "Add rate limiting"; submit becomes enabled. Click submit; `onSubmit("Add rate limiting", "")` fires.
  - Click Cancel; `onClose` fires.
  - Click backdrop; `onClose` fires.
  - Press Esc; `onClose` fires.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/SpecDialog.tsx`. Trap focus within the dialog (basic — first focusable element on open).

**Refactor**: None.

**Commit**: `feat(web): add SpecDialog modal`

**Verification**: `pnpm -F @agentic/web-ui test SpecDialog`

---

### [x] Step W.6.6: Wire "Create spec" button in IssueColumn → SpecDialog → start_ticket_run

**Goal**: Action items "Create spec" button opens `SpecDialog`. Submit calls `start_ticket_run` IPC with `{ticket: title, backend: "claude-code", model: null}`.

**Depends on**: W.6.3, W.6.5.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/IssueColumnSpecFlow.test.tsx`:
  - Mock `@tauri-apps/api/core` `invoke`.
  - Render `<IssueColumn>` with completed state.
  - Click "Create spec"; assert `SpecDialog` open.
  - Type "New spec"; click "Create & run".
  - Assert `invoke` called with `("start_ticket_run", { ticket: "New spec", backend: "claude-code", model: null })`.
  - Assert dialog closed.

**Implement** (GREEN):
- Wire dialog state in `IssueColumn` + invoke call in `onSubmit`.

**Refactor**: None.

**Commit**: `feat(web): wire Create spec to start_ticket_run`

**Verification**: `pnpm -F @agentic/web-ui test IssueColumnSpecFlow`

---

## Phase 7 — Web permission card

### [x] Step W.7.1: `PermissionCard` component

**Goal**: Inline component matching spec §3.7. Three buttons fire `onDecision("once" | "session" | "deny")`.

**Depends on**: W.0.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/PermissionCard.test.tsx`:
  - Renders with a fixture permission `{id: "p2", agent: "developer", tool: "shell", arg: "redis-cli FLUSHDB", scope: "shell.destructive", risk: "high", reason: "...", t: "14:06:02"}`.
  - Asserts the command preview block has the prefix `$ ` and the arg.
  - Asserts the risk pill shows "HIGH RISK" in red.
  - Click "Allow once"; `onDecision("once")` fires.
  - Click "Allow for session"; `onDecision("session")` fires.
  - Click "Deny"; `onDecision("deny")` fires.

**Implement** (GREEN):
- Create `apps/web-ui/src/components/PermissionCard.tsx`.

**Refactor**: None.

**Commit**: `feat(web): add PermissionCard component`

**Verification**: `pnpm -F @agentic/web-ui test PermissionCard`

---

### [x] Step W.7.2: ActivityColumn renders PermissionCard inline for `perm` events

**Goal**: When the activity log contains a permission event with a matching pending permission, render a `PermissionCard` inline at that position.

**Depends on**: W.5.4, W.7.1.

**Test first** (RED):
- Extend `ActivityColumn.test.tsx`:
  - Pass `pendingPermissions={[{id: "p1", ...}]}` and an event stream containing a `perm` event with `permId: "p1"`.
  - Assert one `PermissionCard` rendered at the perm event's position.
  - Decide on it (click "Allow once"); assert `onPermissionDecision("p1", "once")` callback fires.

**Implement** (GREEN):
- Extend `ActivityColumn.tsx`. The `perm` event variant is hypothetical (no `Event::PermissionRequest` exists yet); the test feeds a fixture envelope that the adapter recognizes.

**Refactor**: None.

**Commit**: `feat(web): render PermissionCard inline in ActivityColumn`

**Verification**: `pnpm -F @agentic/web-ui test ActivityColumn`

**Notes**: The backend `PermissionRequest` event variant ships separately. This step renders against a fixture; nothing breaks if no real perm events arrive.

---

## Phase 8 — Web App.tsx swap-in

This phase replaces today's `App.tsx` body with the new shell, removes the now-dead components, and runs the full integration test pass.

### Step W.8.1: Replace App.tsx with AppShell composition ✓

**Goal**: `App.tsx` mounts `AppShell` with `HeaderBar` + `PipelineBar` + `ChatColumn` + `ActivityColumn` + `IssueColumn`. The standalone `Stepper`, `EventList` (now `ActivityColumn`), `FindingsTable`, `PastRunsPane`, `SettingsPane`, `StartRunForm`, `ActiveRunIndicator`, `DismissableBanner`, `DiffViewer` are removed from the visible page (`PastRunsPane` and `SettingsPane` move into modals reachable from the header; `FindingsTable` becomes Action items in IssueColumn; `StartRunForm` becomes the inline run button + `SpecDialog`).

**Depends on**: W.1.3, W.2.7, W.3.1, W.4.6, W.5.4, W.6.6.

**Test first** (RED):
- Update `apps/web-ui/src/__tests__/app.test.tsx`:
  - Renders `<App />`. Assert presence of: `[data-testid="app-shell-header"]`, `[data-testid="app-shell-pipeline"]`, `[data-testid="chat-pane"]`, `[data-testid="event-list"]` (still on ActivityColumn's UL), `[data-testid="issue-column"]`.
  - Assert absence of: standalone `Stepper` (`cockpit-stepper` testid moves into pipeline bar) and standalone `findings-table` directly under main (it now lives inside `IssueColumn`'s action items section, with its existing testid still present there).

**Implement** (GREEN):
- Rewrite `apps/web-ui/src/App.tsx`:
  - Hooks (`useTauriEvents`, `useFindings`, `useChat`, etc.) stay.
  - Layout switches to `<AppShell>`.
  - Settings is wired through to `SettingsModal` in W.8.3 — for this step,
    leave `HeaderBar.onOpenSettings` as a no-op stub or local placeholder.
  - PastRuns is **not** mounted as a top-level page or behind a header
    button; it ships in W.8.2 as a tab inside `SettingsModal`.
  - DiffViewer remains accessible from a finding's detail view (out of scope here — see tech debt).

**Refactor**: Move shared state (run id, theme, etc.) into a small reducer if `App.tsx` body grows past ~120 lines.

**Commit**: `feat(web): swap App.tsx to new design shell`

**Verification**: `pnpm -F @agentic/web-ui test app`

---

### [x] Step W.8.2: Build `SettingsModal` shell with `GeneralTab` + `HistoryTab`

**Goal**: New tabbed-modal component that hosts the existing `SettingsPane`
(General tab) and `PastRunsPane` (History tab). Pure presentation in this
step — wiring into `App.tsx` is W.8.3. The header bar does **not** carry a
standalone History button; History is reachable only via this modal.

**Depends on**: W.8.1.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/SettingsModal.test.tsx`:
  - Render `<SettingsModal open={true} initialTab="general" onClose={fn} />`.
  - Assert `[role="dialog"]` is in the document with
    `[data-testid="settings-modal"]`.
  - Assert a tab strip with two tabs: `[data-testid="settings-tab-general"]`
    (active, `aria-selected="true"`) and
    `[data-testid="settings-tab-history"]` (inactive).
  - Assert the General tab body wraps the existing `SettingsPane` content
    (`[data-testid="settings-pane"]` still present).
  - Click `settings-tab-history`; assert `aria-selected` flips and
    `[data-testid="past-runs-pane"]` is now in the document while
    `settings-pane` is no longer rendered (or is hidden).
  - Click backdrop; `onClose` fires.
  - Press Esc; `onClose` fires.

**Implement** (GREEN):
- Add a `Modal` primitive component if not yet present (extract the JSX
  shape used by `SpecDialog` into `apps/web-ui/src/components/Modal.tsx`).
- Create `apps/web-ui/src/components/SettingsModal.tsx` — owns
  `activeTab` state, focus trap, backdrop / Esc dismissal.
- Create `apps/web-ui/src/components/GeneralTab.tsx` — pure wrapper around
  the existing `SettingsPane` component.
- Create `apps/web-ui/src/components/HistoryTab.tsx` — pure wrapper around
  the existing `PastRunsPane` component. Preserve the
  `data-testid="past-runs-pane"` testid on the wrapped element.

**Refactor**: Extract `Modal` if not done already.

**Commit**: `feat(web): add SettingsModal with General and History tabs`

**Verification**: `pnpm -F @agentic/web-ui test SettingsModal`

---

### [x] Step W.8.3: Wire `SettingsModal` into `App.tsx` from the header settings icon

**Goal**: Click the header's settings icon → `SettingsModal` opens. The
header has **no** History button (PastRuns is reachable only via the
History tab inside this modal).

**Depends on**: W.8.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/AppSettingsModal.test.tsx`:
  - Render `<App />`. Assert no `[data-testid="header-history"]` button is
    in the document (it must not exist).
  - Click `[data-testid="header-settings"]`. Assert
    `[data-testid="settings-modal"]` is in the document and the General
    tab is initially active (`[data-testid="settings-pane"]` visible).
  - Click `[data-testid="settings-tab-history"]`. Assert the History tab
    becomes active and `[data-testid="past-runs-pane"]` renders.
  - Click backdrop; assert modal closed.

**Implement** (GREEN):
- Edit `apps/web-ui/src/App.tsx`: add `settingsOpen` state, wire
  `HeaderBar.onOpenSettings` to set it true, render `<SettingsModal>` at
  the App root.
- Confirm `HeaderBar` does not expose a History button (remove any leftover
  `header-history` testid added in earlier drafts of W.8.1).

**Refactor**: None.

**Commit**: `feat(web): wire SettingsModal to header settings icon`

**Verification**: `pnpm -F @agentic/web-ui test AppSettingsModal`

---

### Step W.8.4: Restyle DismissableBanner and DiffViewer to new tokens

**Goal**: Both components keep their existing API (props, data-testids) but their styling switches to the new design tokens (`bg-bg-surface`, `text-fg-muted`, etc.).

**Depends on**: W.0.2.

**Test first** (RED):
- Update `apps/web-ui/src/__tests__/DismissableBanner.test.tsx` and `DiffViewer.test.tsx`:
  - Snapshot-style assertions: the warning banner's background class is `bg-status-info/10` (or equivalent), error banner is `bg-status-failed/10`. Diff added line is `bg-status-done/10`, removed line is `bg-status-failed/10`.

**Implement** (GREEN):
- Edit the two components' Tailwind class strings to reference the new tokens.

**Refactor**: None.

**Commit**: `style(web): restyle DismissableBanner and DiffViewer to new tokens`

**Verification**: `pnpm -F @agentic/web-ui test DismissableBanner DiffViewer`

---

### Step W.8.5: Delete dead components and update barrel exports

**Goal**: Remove `Stepper.tsx`, `EventList.tsx` (already deleted in W.5.4), `ActiveRunIndicator.tsx` (its content folded into `HeaderBar` and `ChatColumn`), `StartRunForm.tsx` + `StartRunFormInner.tsx` (run button + SpecDialog replace them).

**Depends on**: W.8.3.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/deadCode.test.ts`:
  - Reads each deleted file path via `fs.existsSync`. Asserts they don't exist.
  - Reads `apps/web-ui/src/App.tsx` as text. Asserts no `import` references the deleted modules.

**Implement** (GREEN):
- `git rm` each file.
- Delete the matching `__tests__/<Name>.test.tsx` files for `Stepper`, `ActiveRunIndicator`, `StartRunForm`. (Tests for `EventList` already migrated to `ActivityColumn` in W.5.4.)

**Refactor**: None.

**Commit**: `refactor(web): delete deprecated cockpit components`

**Verification**: `pnpm -F @agentic/web-ui test`

---

### CP-W: Web review checkpoint

**Checkpoint**: Stop. Hand back to user.
- Manual visual smoke test: `pnpm -F @agentic/web-ui dev` and walk through idle / running / completed states; toggle theme.
- All web tests green; no orphaned components.
- Ready to start TUI work.

---

## Phase 9 — Polish

Closes the visual + behavioral gaps surfaced during manual smoke testing
of Phases 0–8. Spec contract: `docs/redesign/spec.md` §6.8. Pipeline
mutation in this phase is **local-only** — no backend IPC changes (see
§6.8.3 for the trade-off rationale and tech-debt #7 for the deferred
persistence work).

### [x] Step W.9.1: Wire pipeline mutation handlers in `App.tsx`

**Goal**: Drag-reorder, `+`-chip insert, kebab Remove and Skip
visibly persist in the UI. Closes user gaps #2 and #3. Per spec §6.8.3,
state is **local-only**: edits live in `App.tsx`'s React tree and do
not flow back to the backend orchestrator (deferred — see tech-debt #7).

**Depends on**: W.2.7, W.2.8, W.8.1.

**Test first** (RED):

- New test `apps/web-ui/src/__tests__/AppPipelineMutation.test.tsx`:
  1. **Reorder**: render `<App />`. Assert pipeline cards order matches
     `runState.steps` (initial: architect → tdd-developer → qa →
     reviewer). Drag the architect card across the gap before qa via
     `fireEvent.dragStart` on `[data-testid="agent-card-architect"]`,
     `fireEvent.dragOver` on `[data-testid="pipeline-gap-2"]`, and
     `fireEvent.drop` on the same gap. Assert the new order is
     tdd-developer → qa → architect → reviewer in the rendered
     `[data-testid^="agent-card-"]` elements.
  2. **Insert**: render `<App />`. Click `[data-testid="pipeline-add-agent"]`,
     type "researcher" in the picker, click the researcher row. Assert
     the rendered pipeline cards now include
     `[data-testid="agent-card-researcher"]` at the end.
  3. **Insert mid-pipeline**: click `[data-testid="pipeline-insert-2"]`
     (gap between developer and qa), select "security". Assert
     `agent-card-security` appears at index 2.
  4. **Remove**: open the qa card's kebab menu, click Remove. Assert
     `[data-testid="agent-card-qa"]` no longer in DOM and the
     remaining cards are in order.
  5. **Skip**: open the reviewer card's kebab, click "Skip this run".
     Assert `[data-testid="agent-card-reviewer"]` has
     `data-skipped="true"` (new attribute) and reduced visual opacity
     (`class*="opacity-50"`).
  6. **Re-seed on new run**: simulate `activeRunId` going from
     `undefined` → `"run-1"` and `runState.steps` arriving with a
     different agent list (e.g. `["architect", "developer", "docs"]`).
     Assert the rendered pipeline reflects the new list (re-seeded),
     **discarding** any prior local edits.

**Implement** (GREEN):

- Edit `apps/web-ui/src/App.tsx`:
  - Add state: `const [pipelineAgents, setPipelineAgents] = useState<string[]>(...)`,
    `const [pipelineSkipped, setPipelineSkipped] = useState<Set<string>>(new Set())`.
  - Seed `pipelineAgents` from `runState.steps.map(s => s.agent)` via
    `useEffect` keyed on `activeRunId` (re-seed only on run-id change,
    not on every `runState` tick — to preserve user edits between runs).
  - Pass `agents={pipelineAgents}` to `<PipelineBar>` (replaces the
    current `pipelineAgents` from `usePipelineFromRunState`).
  - Pass `onReorder={(from, to) => setPipelineAgents(reorderArray(prev, from, to))}`.
  - Pass `onInsert={(at, id) => setPipelineAgents(prev => insertAt(prev, at, id))}`.
  - Pass `onRemove={(at) => setPipelineAgents(prev => prev.filter((_, i) => i !== at))}`.
  - Pass `onSkip={(at) => setPipelineSkipped(prev => new Set([...prev, prev[at]]))}` —
    the agent id at index `at` is added to the skipped set (toggle
    behavior on re-click is fine; document the contract).
- Update `PipelineBar.tsx` to render `data-skipped="true"` and
  `opacity-50 line-through` when the agent id is in the skipped set
  (new prop: `skipped?: Set<string>`).
- Helper utilities in `apps/web-ui/src/utils/arrayMove.ts`: pure
  `reorderArray(arr, from, to)` and `insertAt(arr, at, id)` — covered
  by their own unit tests.

**Refactor**: extract `usePipelineMutation` hook from `App.tsx` if the
state + handlers grow past ~50 lines.

**Files**:
- `apps/web-ui/src/App.tsx` (edit)
- `apps/web-ui/src/components/PipelineBar.tsx` (edit — add `skipped` prop)
- `apps/web-ui/src/components/AgentCard.tsx` (edit — wire skipped style)
- `apps/web-ui/src/utils/arrayMove.ts` (new)
- `apps/web-ui/src/__tests__/AppPipelineMutation.test.tsx` (new)
- `apps/web-ui/src/__tests__/arrayMove.test.ts` (new)

**Commit**:
- `test(web): add failing tests for App.tsx pipeline mutation wiring`
- `feat(web): wire pipeline reorder/insert/remove/skip in App.tsx`

**Verification**: `pnpm -F @agentic/web-ui test AppPipelineMutation arrayMove`

**Notes**: Local-only state — no backend IPC. Tech-debt #7 tracks
backend persistence; this step does NOT block on it.

---

### Step W.9.2: Add per-agent SVG icon library + render in AgentCard / AgentPicker / ChatMessage

**Goal**: Replace the placeholder white rect / `bg-bg-surface-2` / colored
circle with the 12 monoline SVG glyphs from the design hand-off. Closes
user gap #1. Spec contract: §6.8.1.

**Depends on**: W.0.4, W.2.1, W.2.5, W.4.1.

**Test first** (RED):

- New test `apps/web-ui/src/__tests__/AgentIcon.test.tsx`:
  - Renders `<AgentIcon agent="architect" />`. Asserts a single `<svg>` is
    in the document with `viewBox="0 0 20 20"`, and the SVG contains a
    `<path>` whose `d` attribute equals the `blueprint` glyph path
    (`M3 4h14v12H3zM3 8h14M7 4v12M11 12h2`).
  - Renders for each of the 12 known agent ids; asserts the path matches
    the icon-key from `AGENT_LIBRARY`.
  - Renders for `tdd-developer` (the project's local rename of
    `developer`); asserts the path matches the `code` glyph.
  - Renders for `unknown-agent`; asserts a fallback rect path is rendered
    (no crash).
  - Asserts `size` prop default is 18; passing `size={14}` sets `width`/
    `height` accordingly.

- Extend `AgentCard.test.tsx`:
  - Render with `agent="architect"`. Assert the avatar tile contains
    `<svg>` with the blueprint path (no longer the placeholder white
    rect). Assert step number `01` (when `index={0}`) renders to the
    left of the avatar with `data-testid="agent-card-step-number"`.
    Assert the agent name shown is `Architect` (from `AGENT_LIBRARY`),
    not `architect`.
  - Render with `agent="tdd-developer"`. Assert name shows `Developer`
    (alias entry); avatar SVG matches `code` glyph.

- Extend `AgentPicker.test.tsx`:
  - Render `<AgentPicker excludeIds={[]} ... />`. Assert each row's
    leading 32×32 avatar contains an `<svg>` with the matching glyph
    path (replaces the `bg-bg-surface-2` placeholder span).

- Extend `ChatMessage.test.tsx`:
  - Render `<ChatMessage kind="agent" agent="developer" ... />`. Assert
    the avatar circle contains the `code` glyph SVG (replaces the
    colored-tint round div).

**Implement** (GREEN):

- Add an alias entry for `tdd-developer` to `AGENT_LIBRARY` in
  `apps/web-ui/src/types/pipeline.ts`:
  ```ts
  { id: "tdd-developer", name: "Developer", icon: "code", desc: "Writes code & tests (TDD)" },
  ```
- Create `apps/web-ui/src/components/AgentIcon.tsx`:
  - Export a `AGENT_ICON_PATHS: Record<string, string>` constant with the
    12 paths transcribed verbatim from the hand-off `agents.jsx`.
  - The component looks up `AGENT_LIBRARY.find(a => a.id === agent)?.icon`,
    indexes into `AGENT_ICON_PATHS`, and renders an `<svg viewBox="0 0
    20 20" width={size} height={size} aria-hidden="true">` with a
    `<path>` (or `<g>` for multi-element glyphs — see `eye`, `gauge`,
    `palette`, `database`, `a11y`).
  - Falls back to a rounded-rect placeholder when the lookup fails.
- Edit `AgentCard.tsx`:
  - Replace the placeholder white rect (existing lines 94–96) with
    `<AgentIcon agent={agent} size={18} />`.
  - Add a step-number span to the left of the avatar:
    `<span data-testid="agent-card-step-number" className="text-[11px] font-semibold text-fg-subtle tabular-nums w-4 text-right">{String(index + 1).padStart(2, "0")}</span>`.
    The component now needs an `index: number` prop.
  - Use the resolved name from `AGENT_LIBRARY` for the displayed text:
    `const lib = AGENT_LIBRARY.find(a => a.id === agent); const displayName = lib?.name ?? agent;`.
- Edit `AgentPicker.tsx`:
  - Replace the `bg-bg-surface-2` placeholder span with
    `<AgentIcon agent={agent.id} size={16} />` inside a 32×32 tile with
    `bg-agent-<id>` (or fallback) accent background.
  - Swap `shadow-modal` to `shadow-popover` (per §6.8.7).
- Edit `ChatMessage.tsx`:
  - Replace the colored-tint avatar `<div>` (existing lines 71–75) with
    `<AgentIcon agent={agent} size={14} />` inside a 28×28 round tile
    using the same agent-tinted background.
- Edit `PipelineBar.tsx` to pass the `index` prop into each `<AgentCard>`.

**Refactor**: None — paths are data, component is total.

**Files**:
- `apps/web-ui/src/types/pipeline.ts` (edit — alias entry)
- `apps/web-ui/src/components/AgentIcon.tsx` (new)
- `apps/web-ui/src/components/AgentCard.tsx` (edit)
- `apps/web-ui/src/components/AgentPicker.tsx` (edit)
- `apps/web-ui/src/components/ChatMessage.tsx` (edit)
- `apps/web-ui/src/components/PipelineBar.tsx` (edit — pass index)
- `apps/web-ui/src/__tests__/AgentIcon.test.tsx` (new)
- `apps/web-ui/src/__tests__/AgentCard.test.tsx` (edit)
- `apps/web-ui/src/__tests__/AgentPicker.test.tsx` (edit)
- `apps/web-ui/src/__tests__/ChatMessage.test.tsx` (edit)

**Commit**:
- `test(web): add failing tests for per-agent SVG icon library`
- `feat(web): add AgentIcon and render glyphs in card/picker/message`

**Verification**: `pnpm -F @agentic/web-ui test AgentIcon AgentCard AgentPicker ChatMessage`

---

### Step W.9.3: ChatComposer layout polish — chip placement, plane send icon, placeholder

**Goal**: Match the hand-off `panels.jsx` chrome: chips below the input,
placeholder `Ask a question, or use /plan, /develop, /@agent…`,
paper-plane send glyph, dynamic send button background, single bordered
container wrapping textarea + icon buttons. Closes user gap #5. Spec
contract: §6.8.4.

**Depends on**: W.4.3, W.4.4, W.4.5.

**Test first** (RED):

- Extend `apps/web-ui/src/__tests__/ChatComposer.test.tsx`:
  1. **Chip placement**: render `<ChatComposer onSend={fn} />`. Get the
     DOM order of `[data-testid="chat-composer-chip-plan"]` and
     `[data-testid="chat-composer-textarea"]`. Assert the textarea
     appears **before** the first chip in document order
     (`compareDocumentPosition` returns `Node.DOCUMENT_POSITION_FOLLOWING`
     when comparing textarea → chip).
  2. **Placeholder**: assert the textarea's `placeholder` attribute is
     `"Ask a question, or use /plan, /develop, /@agent…"`.
  3. **Send glyph**: assert the send button contains an `<svg>` whose
     `<path>` `d` attribute is `M3 10l14-7-3 16-4-7-7-2z` (paper plane).
     The previous up-arrow path (`M8 14V2 M3 7l5-5 5 5`) must NOT be in
     the document.
  4. **Send button dynamic background**: with empty draft, assert the
     send button's class string contains `bg-bg-surface-2` (or has
     inline-style background `rgb(228 228 231)` — pick the impl form
     and lock it). After typing `hello`, assert the class flips to
     `bg-[#18181b]` / black.
  5. **Outer wrapper**: assert there is a single ancestor div wrapping
     both `[data-testid="chat-composer-textarea"]` and
     `[data-testid="chat-composer-send"]` with classes containing
     `border` + `rounded-xl` (or radius 12) + `p-1.5` (= 6 px). Verify
     by walking up `el.closest('[data-testid="chat-composer-input-wrapper"]')`
     and asserting it contains both children.

**Implement** (GREEN):

- Edit `apps/web-ui/src/components/ChatComposer.tsx`:
  - Restructure JSX so the chip row renders **after** the input wrapper
    (move the `<div className="flex gap-2">` of chips to be a sibling
    AFTER the input container).
  - Wrap the textarea + icon button group in a single bordered
    container with `data-testid="chat-composer-input-wrapper"`:
    ```
    <div data-testid="chat-composer-input-wrapper" className="flex items-end gap-2 rounded-xl border border-[rgb(0_0_0_/_0.1)] bg-bg-surface p-1.5 shadow-card">
      <textarea ... />
      <div className="flex items-center gap-1">
        <button data-testid="chat-composer-new-spec" ... />  {/* W.9.4 */}
        <button data-testid="chat-composer-send" ... />
      </div>
    </div>
    ```
  - Update placeholder to
    `"Ask a question, or use /plan, /develop, /@agent…"`.
  - Swap the send SVG path to `M3 10l14-7-3 16-4-7-7-2z` (paper plane).
    Drop the `stroke="currentColor"` attributes; use `fill="currentColor"`
    for the solid plane shape.
  - Compute send button bg dynamically:
    ```ts
    const sendActive = value.trim() !== "";
    // className: sendActive ? "bg-[#18181b] text-white" : "bg-bg-surface-2 text-fg-subtle"
    ```
  - Keep the existing 36×36 footprint (per spec §3.4 line 224 — README
    wins on size; only the glyph swaps).
  - Remove the textarea's individual `border` + `focus:ring` classes;
    those styles move onto the wrapper.

**Refactor**: None.

**Files**:
- `apps/web-ui/src/components/ChatComposer.tsx` (edit)
- `apps/web-ui/src/__tests__/ChatComposer.test.tsx` (edit)

**Commit**:
- `test(web): add failing tests for ChatComposer layout polish`
- `feat(web): match ChatComposer layout to design handoff`

**Verification**: `pnpm -F @agentic/web-ui test ChatComposer`

**Notes**: This step DOES NOT add the New-spec button — that's W.9.4.
The wrapper structure is set up in this step so W.9.4 can drop the
button into the icon-button group cleanly.

---

### Step W.9.4: ChatComposer "New spec" affordance

**Goal**: Add a small doc-icon button immediately to the left of the
send button. Click opens `SpecDialog`. Closes user gap #6. Spec contract:
§6.8.4 last paragraph.

**Depends on**: W.6.5 (`SpecDialog` exists), W.9.3 (input wrapper
restructure).

**Test first** (RED):

- Extend `apps/web-ui/src/__tests__/ChatComposer.test.tsx`:
  - Render `<ChatComposer onSend={fn} onCreateSpec={specFn} />`.
  - Assert `[data-testid="chat-composer-new-spec"]` is in the document,
    inside `[data-testid="chat-composer-input-wrapper"]`, and appears
    **before** `[data-testid="chat-composer-send"]` in document order.
  - Assert the button has `aria-label="Create spec"` and the SVG path
    is `M5 3h7l3 3v11H5zM12 3v3h3M7 9h6M7 12h6M7 15h4` (doc icon).
  - Click the button. Assert `specFn` is called with no args.
- Extend `ChatColumn.test.tsx`:
  - Render `<ChatColumn ... />` inside a parent that owns `specOpen`
    state. Click the new-spec button in the composer. Assert
    `[data-testid="spec-dialog"]` is in the document.
  - Type a title, click `Create & run`. Assert mock `invoke` called
    with `("start_ticket_run", { ticket: "...", backend: "claude-code", model: null })`.

**Implement** (GREEN):

- Edit `apps/web-ui/src/components/ChatComposer.tsx`:
  - Add prop `onCreateSpec?: () => void`.
  - Render the doc-icon button as the first child of the icon-button
    group (before send). Use the SVG path from the hand-off:
    `M5 3h7l3 3v11H5zM12 3v3h3M7 9h6M7 12h6M7 15h4`, stroke 1.4,
    fill none. 14×14 inside a 28×28 ghost button (`text-fg-muted
    hover:text-fg`). `aria-label="Create spec"`, `title="Create spec"`.
- Edit `apps/web-ui/src/components/ChatColumn.tsx`:
  - Add local state `const [specOpen, setSpecOpen] = useState(false)`.
  - Pass `onCreateSpec={() => setSpecOpen(true)}` to `<ChatComposer>`.
  - Render `<SpecDialog open={specOpen} onClose={() => setSpecOpen(false)} onSubmit={...} />`
    at the column root. The `onSubmit` calls `start_ticket_run` IPC
    (mirrors `IssueColumn.handleCreateSpecSubmit`).

**Refactor**: extract the `handleCreateSpecSubmit` body into a shared
`apps/web-ui/src/utils/createSpec.ts` helper if the same logic now lives
in two places.

**Files**:
- `apps/web-ui/src/components/ChatComposer.tsx` (edit)
- `apps/web-ui/src/components/ChatColumn.tsx` (edit)
- `apps/web-ui/src/utils/createSpec.ts` (optional; new if extracted)
- `apps/web-ui/src/__tests__/ChatComposer.test.tsx` (edit)
- `apps/web-ui/src/__tests__/ChatColumn.test.tsx` (edit)

**Commit**:
- `test(web): add failing tests for ChatComposer New-spec button`
- `feat(web): add New-spec doc icon to ChatComposer`

**Verification**: `pnpm -F @agentic/web-ui test ChatComposer ChatColumn`

---

### Step W.9.5: HeaderBar settings gear icon

**Goal**: Swap the unrecognizable settings SVG path with the standard
heroicons solid-cog path. Closes user gap #4. Spec contract: §6.8.5.

**Depends on**: W.1.1.

**Test first** (RED):

- Extend `apps/web-ui/src/__tests__/HeaderBar.test.tsx`:
  - Render `<HeaderBar ... />`. Get `[data-testid="header-settings"]`.
  - Assert the inner `<svg>` has `viewBox="0 0 20 20"` (was `0 0 16 16`).
  - Assert the `<path>` `d` attribute starts with `M7.84 1.804A1 1 0 018.82 1`
    (the heroicons cog path) and contains `M10 13a3 3 0 100-6` (the
    inner circle subpath).
  - Assert the previous proprietary path (starts with `M7.0 0.5`) is
    NOT in the document.

**Implement** (GREEN):

- Edit `apps/web-ui/src/components/HeaderBar.tsx`:
  - Replace the existing SVG (lines 149–158) with:
    ```jsx
    <svg viewBox="0 0 20 20" className="h-[14px] w-[14px]" fill="currentColor" aria-hidden="true">
      <path d="M7.84 1.804A1 1 0 018.82 1h2.36a1 1 0 01.98.804l.331 1.652a6.993 6.993 0 011.929 1.115l1.598-.54a1 1 0 011.186.447l1.18 2.044a1 1 0 01-.205 1.251l-1.267 1.113a7.047 7.047 0 010 2.228l1.267 1.113a1 1 0 01.206 1.25l-1.18 2.045a1 1 0 01-1.187.447l-1.598-.54a6.993 6.993 0 01-1.929 1.115l-.33 1.652a1 1 0 01-.98.804H8.82a1 1 0 01-.98-.804l-.331-1.652a6.993 6.993 0 01-1.929-1.115l-1.598.54a1 1 0 01-1.186-.447l-1.18-2.044a1 1 0 01.205-1.251l1.267-1.114a7.05 7.05 0 010-2.227L1.821 7.773a1 1 0 01-.206-1.25l1.18-2.045a1 1 0 011.187-.447l1.598.54A6.992 6.992 0 017.51 3.456l.33-1.652zM10 13a3 3 0 100-6 3 3 0 000 6z" />
    </svg>
    ```
  - Keep the wrapper button's `data-testid="header-settings"`,
    `aria-label="Settings"`, and click handler unchanged.

**Refactor**: None.

**Files**:
- `apps/web-ui/src/components/HeaderBar.tsx` (edit)
- `apps/web-ui/src/__tests__/HeaderBar.test.tsx` (edit)

**Commit**:
- `test(web): assert HeaderBar settings icon uses heroicons cog path`
- `fix(web): swap HeaderBar settings glyph to heroicons cog`

**Verification**: `pnpm -F @agentic/web-ui test HeaderBar`

**Notes**: No new icon-library dependency. The path is inlined to
match the rest of the codebase's inline-SVG convention. If the project
adopts a Lucide / Heroicons npm package later, this becomes the first
component to migrate.

---

### Step W.9.6: `StatusDot` component + use in AgentCard

**Goal**: Replace the bare uppercase status text in `AgentCard` with a
proper `StatusDot` pill (colored dot + label). Spec contract: §6.8.2.

**Depends on**: W.0.4.

**Test first** (RED):

- New test `apps/web-ui/src/__tests__/StatusDot.test.tsx`:
  - Renders `<StatusDot status="queued" />`. Asserts the rendered text
    is `Queued`, the pill has class `bg-zinc-100` and `text-zinc-500`,
    and a leading `<span>` dot is present.
  - Renders `status="active"`. Asserts label `Running`, classes
    `bg-blue-100 text-blue-700`, and the dot has `animate-pulse`.
  - Renders `status="done"`. Asserts label `Done`,
    `bg-green-100 text-green-700`.
  - Renders `status="failed"`. Asserts label `Failed`,
    `bg-red-100 text-red-700`.
  - Renders `status="skipped"`. Asserts label `Skipped`,
    `bg-zinc-100 text-zinc-400`, dot opacity reduced.
- Extend `AgentCard.test.tsx`:
  - With `status="active"`, assert a `[data-testid="status-dot"]` is
    rendered with text matching `/Running/`.
  - The previous bare uppercase status text (line 110–112 in shipped
    `AgentCard.tsx`) is removed.

**Implement** (GREEN):

- Create `apps/web-ui/src/components/StatusDot.tsx` per spec §6.8.2.
- Edit `AgentCard.tsx`: replace the
  `<span className="text-[10px] uppercase ...">{status}</span>` element
  with `<StatusDot status={status} />`.

**Refactor**: None.

**Files**:
- `apps/web-ui/src/components/StatusDot.tsx` (new)
- `apps/web-ui/src/components/AgentCard.tsx` (edit)
- `apps/web-ui/src/__tests__/StatusDot.test.tsx` (new)
- `apps/web-ui/src/__tests__/AgentCard.test.tsx` (edit)

**Commit**:
- `test(web): add failing tests for StatusDot component`
- `feat(web): add StatusDot pill and use in AgentCard`

**Verification**: `pnpm -F @agentic/web-ui test StatusDot AgentCard`

---

### Step W.9.7: IssueColumn header polish (run-state pill + section labels)

**Goal**: Add `StatusDot` next to the issue id and uppercase
"DESCRIPTION" / "ACCEPTANCE CRITERIA" section labels. Spec contract:
§6.8.6.

**Depends on**: W.6.1, W.9.6.

**Test first** (RED):

- Extend `apps/web-ui/src/__tests__/IssueColumn.test.tsx`:
  - Render with `runState="running"`. Assert
    `[data-testid="issue-column"] [data-testid="status-dot"]` is in the
    document with text matching `/Running/` (i.e. `StatusDot` rendered
    inline next to the issue id).
  - Render with `runState="completed"`. Assert the dot now reads `Done`.
  - Render with `runState="idle"`. Assert the dot reads `Queued`.
  - Assert section labels: `[data-testid="issue-section-description"]`
    with text `Description` (rendered uppercase via CSS), and
    `[data-testid="issue-section-acceptance"]` with text
    `Acceptance criteria`.

**Implement** (GREEN):

- Edit `apps/web-ui/src/components/IssueColumn.tsx`:
  - Map `runState` → `AgentStatus`:
    - `idle` → `queued`
    - `running` → `active`
    - `completed` → `done`
    - `failed` → `failed`
  - Render `<StatusDot status={mapped} />` to the right of the issue
    id (inline, in the same flex row).
  - Insert `<div data-testid="issue-section-description" className="text-[11px] font-semibold uppercase tracking-[0.05em] text-fg-muted">Description</div>`
    above the description block (only when `ticket.body.length > 0`).
  - Insert `<div data-testid="issue-section-acceptance" className="text-[11px] font-semibold uppercase tracking-[0.05em] text-fg-muted">Acceptance criteria</div>`
    above the acceptance list (only when `ticket.acceptance.length > 0`).

**Refactor**: None.

**Files**:
- `apps/web-ui/src/components/IssueColumn.tsx` (edit)
- `apps/web-ui/src/__tests__/IssueColumn.test.tsx` (edit)

**Commit**:
- `test(web): add failing tests for IssueColumn header pill and section labels`
- `feat(web): add run-state pill and section labels to IssueColumn`

**Verification**: `pnpm -F @agentic/web-ui test IssueColumn`

---

### Step W.9.8: App.tsx integration test — full polish flow

**Goal**: One end-to-end test that exercises the polish surface in `App.tsx`:
icon glyphs render in the live pipeline bar, the gear icon is the new
path, the chat composer has the new layout, and the new-spec button
opens `SpecDialog` from the chat column.

**Depends on**: W.9.1, W.9.2, W.9.3, W.9.4, W.9.5.

**Test first** (RED):

- New test `apps/web-ui/src/__tests__/AppPolish.test.tsx`:
  - Render `<App />`.
  - Assert at least one `[data-testid^="agent-card-"]` contains an
    `<svg viewBox="0 0 20 20">` (icon library rendered).
  - Assert `[data-testid="header-settings"]` `<svg>` has `viewBox="0 0 20 20"`
    (new path).
  - Assert `[data-testid="chat-composer-new-spec"]` is in the document.
  - Click `chat-composer-new-spec`. Assert
    `[data-testid="spec-dialog"]` is in the document.
  - Press Esc; assert dialog closes.
  - Assert `[data-testid="chat-composer-textarea"]` placeholder is
    `"Ask a question, or use /plan, /develop, /@agent…"`.

**Implement** (GREEN):

- N/A — this is a verification step. If a test fails, the prior step
  has a contract gap; fix it there, not here.

**Refactor**: None.

**Files**:
- `apps/web-ui/src/__tests__/AppPolish.test.tsx` (new)

**Commit**:
- `test(web): add App.tsx polish integration test`

**Verification**: `pnpm -F @agentic/web-ui test AppPolish`

---

### CP-W2: Web polish review checkpoint

**Checkpoint**: Stop. Hand back to user.

- Manual visual smoke test: `pnpm -F @agentic/web-ui dev` and walk
  through the six original gaps + the audit-flagged additions:
  1. Pipeline bar shows per-agent SVG glyphs (not white rects).
  2. Drag a card; the new order persists after drop.
  3. Click `+ Add agent`; pick one; it appears in the pipeline.
  4. Header settings icon visually reads as a gear.
  5. Chat composer chips render below the textarea; placeholder reads
     "Ask a question…"; send glyph is a paper-plane.
  6. Click the doc icon left of the send button; SpecDialog opens.
  7. (W.9.6) Status pills on agent cards read "Done"/"Running"/etc.
     (not bare `done`/`running`).
  8. (W.9.7) Issue column header shows a status pill next to the id;
     "Description" + "Acceptance criteria" labels render above sections.
- All web tests green.
- Pipeline mutation (W.9.1) is local-only by design — verify a fresh
  run replays the seeded `runState.steps`, while edits between runs
  persist visually until the next run starts.
- Tech-debt entry for backend pipeline persistence (#7) is unchanged.
- Ready to start TUI work (Phase 10).

---

## Phase 10 — TUI palette + title bar

### Step T.10.1: New `theme` module with color constants

**Goal**: Centralize all TUI colors per spec §4.1 in `crates/agentic-tui/src/theme.rs`.

**Depends on**: none.

**Test first** (RED):
- `crates/agentic-tui/src/theme.rs` doctest: asserts each constant compiles to a `Color::Rgb(...)` with the documented hex values.
- New test in `tests/theme_constants.rs`: snapshot-style `assert_eq!(theme::ACCENT, Color::Rgb(0x5e, 0xea, 0xd4))`.

**Implement** (GREEN):
- Create the module with all constants from spec §4.1.

**Refactor**: None.

**Commit**: `feat(tui): add theme module with design palette`

**Verification**: `cargo test -p agentic-tui theme`

---

### Step T.10.2: Title bar (28 px, traffic lights + centered text)

**Goal**: New widget rendering the title bar at the top of the frame.

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/title_bar.rs` using `TestBackend`:
  - Render at 80×30. Assert the top row contains `●` glyphs at columns 1, 3, 5 (or matching positions).
  - Assert the centered text matches `/agentic — \d+×\d+/`.

**Implement** (GREEN):
- Create `crates/agentic-tui/src/views/title_bar.rs`. Render via three `Span`s for traffic lights + a centered `Paragraph` for the title text.
- Integrate into `draw_app` above the existing two-pane layout — but only after the issue header / pipeline bar / tab bar are also shipped (T.11.x), so for now wire it as the topmost row of `draw_app` and shift everything else 1 row down.

**Refactor**: None.

**Commit**: `feat(tui): add title bar with traffic lights and dimensions`

**Verification**: `cargo test -p agentic-tui title_bar`

---

## Phase 11 — TUI pipeline strip + tabs

### Step T.11.1: Issue header strip

**Goal**: Render the `▰ agentic │ AGT-204 <title>` row + run-state pill on the right.

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/issue_header.rs`:
  - Render with `pipeline_state.run_label = "AGT-204"`, `pipeline_state.run_title = "Add multi-tenant rate limiting"`, `pipeline_state.elapsed_seconds = 154`.
  - Assert text `▰ agentic │ AGT-204 Add multi-tenant rate limiting` appears.
  - Assert `running 02:34` in BLUE on the right.

**Implement** (GREEN):
- Add fields to `AppState`: `run_label: Option<String>`, `run_title: Option<String>`, `run_elapsed_secs: u64`.
- Create `crates/agentic-tui/src/views/issue_header.rs`.

**Refactor**: None.

**Commit**: `feat(tui): add issue header strip`

**Verification**: `cargo test -p agentic-tui issue_header`

---

### Step T.11.2: ASCII pipeline bar — boxes + connectors

**Goal**: Render the 4-row ASCII pipeline strip per spec §4.4. Status glyphs and per-status colors per the palette.

**Depends on**: T.10.1, T.11.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/pipeline_bar.rs`:
  - Set `state.pipeline = vec![architect_done(), developer_active(), qa_queued(), reviewer_queued()]`.
  - Render at 140×40. Assert the top-row buffer contains the substring `┌─` four times and `──▶` three times.
  - Assert the middle row contains `✓ 01 Architect`, `● 02 Developer`, `○ 03 QA`, `○ 04 Reviewer`.
  - Assert the third row of card 2 ("ACTIVE") is in YELLOW (assert via `Cell::style.fg`).

**Implement** (GREEN):
- New `AgentInstance` + `AgentRunStatus` (or reuse existing `StepRunStatus` adapted) on `AppState`.
- Create `crates/agentic-tui/src/views/pipeline_bar.rs` with the box-drawing logic.

**Refactor**: None.

**Commit**: `feat(tui): add ASCII pipeline bar with status boxes`

**Verification**: `cargo test -p agentic-tui pipeline_bar`

---

### Step T.11.3: Pipeline footer hint

**Goal**: Render `[a]dd  [r]eorder  [d]rop` in DIM below the pipeline bar.

**Depends on**: T.11.2.

**Test first** (RED):
- Extend `pipeline_bar.rs` test:
  - Assert the row immediately below the boxes contains `[a]dd  [r]eorder  [d]rop`.
  - Assert it is rendered in DIM color.

**Implement** (GREEN):
- Append a 1-row hint inside `pipeline_bar.rs`.

**Refactor**: None.

**Commit**: `feat(tui): add pipeline bar hint row`

**Verification**: `cargo test -p agentic-tui pipeline_bar`

---

### Step T.11.4: Tab bar widget

**Goal**: Render `① logs   ② chat   ③ issue` row with active tab highlighted (2 px ACCENT bottom border + brighter FG).

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/tab_bar.rs`:
  - Render with `state.pane = Pane::Logs`. Assert `① logs` is in ACCENT bold; `② chat` and `③ issue` in DIM.
  - Switch state to `Pane::Chat`; assert the highlight moves.

**Implement** (GREEN):
- Replace `Pane` enum: add `Logs`, `Chat`, `Issue`. Adapter from old `Cockpit → Logs` so existing tests don't break.
- Create `crates/agentic-tui/src/views/tab_bar.rs`.

**Refactor**: Update existing `app.rs` references.

**Commit**: `feat(tui): add tab bar with three panes`

**Verification**: `cargo test -p agentic-tui tab_bar`

---

### Step T.11.5: Wire `1` / `2` / `3` keys to switch panes

**Goal**: New keys switch the active pane. Existing `Tab` no longer toggles between two panes — it cycles through three (or is removed; pick `1/2/3` only and document).

**Depends on**: T.11.4.

**Test first** (RED):
- New test `crates/agentic-tui/tests/pane_switch_keys.rs`:
  - `state.handle_key(KeyCode::Char('1'))` → `state.pane == Pane::Logs`.
  - `'2'` → `Chat`. `'3'` → `Issue`.
  - Existing `Tab` test still passes (cycles).

**Implement** (GREEN):
- Add the three branches to `AppState::handle_key`.

**Refactor**: None.

**Commit**: `feat(tui): switch panes with 1/2/3 keys`

**Verification**: `cargo test -p agentic-tui pane_switch_keys`

---

## Phase 12 — TUI body panes restyle

### Step T.12.1: Logs pane — column-aligned rows

**Goal**: Replace the current bare cockpit stepper with a logs pane: columns time | agent | level | message per spec §4.6 logs variant.

**Depends on**: T.10.1, T.11.4.

**Test first** (RED):
- New test `crates/agentic-tui/tests/logs_pane.rs`:
  - Seed `state.log` with three entries (info, tool, error).
  - Render at 140×40. Assert the time column is in DIM, the agent column in agent color, the level column in level color, and the message column in FG.

**Implement** (GREEN):
- Create `crates/agentic-tui/src/views/logs_pane.rs`. Replace the `cockpit::render` body when `pane == Logs`.

**Refactor**: None.

**Commit**: `feat(tui): add logs pane with column-aligned rows`

**Verification**: `cargo test -p agentic-tui logs_pane`

---

### Step T.12.2: Chat pane — message blocks

**Goal**: Render chat messages with system dividers, user/agent labels in cyan/green, body indented 2 cols, slash + mention highlighting.

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/chat_pane.rs`:
  - Seed `state.chat` with one user, one system, one agent message.
  - Render. Assert the system message is centered with `── … ──`, user line is in ACCENT, agent line is in GREEN.
  - Assert `/develop` token is highlighted (yellow bg).

**Implement** (GREEN):
- Add `ChatMessage` type to `app.rs` (or new `chat.rs` module).
- Create `crates/agentic-tui/src/views/chat_pane.rs`.

**Refactor**: None.

**Commit**: `feat(tui): add chat pane with message blocks`

**Verification**: `cargo test -p agentic-tui chat_pane`

---

### Step T.12.3: Issue pane — id, title, labels, description, acceptance

**Goal**: Render the issue body in monospace per spec §4.6 issue variant.

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/issue_pane.rs`:
  - Seed `state.run_label = "AGT-204"`, `state.run_title = "..."`, `state.run_labels = vec!["backend","api"]`, `state.run_body = vec!["para 1","para 2"]`, `state.run_acceptance = vec!["a1","a2"]`.
  - Render. Assert id ACCENT, title bold, label chips with 1 px borders, description paragraphs, acceptance as `[ ]`.

**Implement** (GREEN):
- Create `crates/agentic-tui/src/views/issue_pane.rs`.

**Refactor**: None.

**Commit**: `feat(tui): add issue pane`

**Verification**: `cargo test -p agentic-tui issue_pane`

---

## Phase 13 — TUI permission card + status line

### Step T.13.1: Inline permission card in logs

**Goal**: When `state.pending_perms` is non-empty and the logs pane is active, render a red-bordered permission card after the most recent log row.

**Depends on**: T.12.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/perm_card.rs`:
  - Seed a pending permission.
  - Render. Assert a red-bordered region with `⚠ PERM`, the agent name, "HIGH RISK", `$ rm -rf node_modules`, `[y] allow once  [s] session  [n] deny`.

**Implement** (GREEN):
- Add `PermissionRequest` and `pending_perms` field to `AppState`.
- Create `crates/agentic-tui/src/views/perm_card.rs`.

**Refactor**: None.

**Commit**: `feat(tui): add inline permission card`

**Verification**: `cargo test -p agentic-tui perm_card`

---

### Step T.13.2: Wire `y` / `s` / `n` keys to resolve permission

**Goal**: When a permission is pending, those keys resolve it and emit a flash message.

**Depends on**: T.13.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/perm_keys.rs`:
  - Seed `state.pending_perms = vec![p1]`.
  - `state.handle_key(KeyCode::Char('y'))`. Assert `state.pending_perms` empty and `state.flash` set to a string starting with `✓ once:`.
  - Reset; `'s'` → `✓ session:`.
  - Reset; `'n'` → `✗ denied:`.

**Implement** (GREEN):
- Add the three branches; respect existing `i` triage scoping (only handle `y/s/n` when a permission is pending; otherwise no-op).

**Refactor**: None.

**Commit**: `feat(tui): resolve pending permissions via y/s/n keys`

**Verification**: `cargo test -p agentic-tui perm_keys`

---

### Step T.13.3: Mode indicator in status line

**Goal**: Bottom-row status line with mode indicator at right (NORMAL/INSERT/COMMAND in matching colors).

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/status_line.rs`:
  - Render with `state.mode = Mode::Normal`. Assert the bottom row right-aligned text is `NORMAL` in DIM.
  - Switch to Command. Assert `COMMAND` in YELLOW + the `:` prefix on the left.
  - Add `Mode::Insert` variant. Switch to Insert. Assert `INSERT` in GREEN.

**Implement** (GREEN):
- Add `Mode::Insert` to `modes.rs`.
- Replace the existing chat pane footer logic with a global status line widget at the bottom of `draw_app`.

**Refactor**: Move the status-line rendering out of `chat::render` into `views::status_line::render`.

**Commit**: `feat(tui): add status line with NORMAL/INSERT/COMMAND mode indicator`

**Verification**: `cargo test -p agentic-tui status_line`

---

### Step T.13.4: Flash messages on the status line

**Goal**: When `state.flash` is set, the status line shows it in ACCENT for ~1.6 s, then reverts. Driven by `flash_set_at: Option<Instant>`.

**Depends on**: T.13.3.

**Test first** (RED):
- Extend `status_line.rs` test:
  - Set `state.flash = Some("✓ once: shell".into())` and `state.flash_set_at = Some(Instant::now())`.
  - Render. Assert the flash text in ACCENT.
  - Manually advance `state.flash_set_at` to `Instant::now() - Duration::from_secs(2)`. Call `state.tick()`. Assert flash cleared.

**Implement** (GREEN):
- Add `flash_set_at: Option<Instant>` and `tick(&mut self)` method on `AppState`.
- The bin's main loop calls `tick()` on every iteration before `draw`.

**Refactor**: None.

**Commit**: `feat(tui): add flash message lifecycle to status line`

**Verification**: `cargo test -p agentic-tui status_line`

---

### Step T.13.5: Help overlay (`?` toggle)

**Goal**: Pressing `?` opens a centered modal listing keybindings; Esc or any click dismisses.

**Depends on**: T.10.1.

**Test first** (RED):
- New test `crates/agentic-tui/tests/help_overlay.rs`:
  - `state.handle_key(KeyCode::Char('?'))`. Assert `state.help_open == true`.
  - Render. Assert `┌── KEYBINDINGS ──┐` in the buffer.
  - Press `Esc`; assert closed.

**Implement** (GREEN):
- Add `help_open: bool` to `AppState`.
- Create `crates/agentic-tui/src/views/help_overlay.rs`. Render conditionally above all other layers.

**Refactor**: None.

**Commit**: `feat(tui): add help overlay`

**Verification**: `cargo test -p agentic-tui help_overlay`

---

### Step T.13.6: Insert mode (`i` in chat/logs pane) + triage `i` in issue pane

**Goal**: Resolve the `i` key collision between today's "triage as ignore"
and the new "enter INSERT mode" behavior by **scoping the key by active
pane**. The dual binding is the contract:

- `pane == Logs` or `pane == Chat` → `i` enters `Mode::Insert`.
- `pane == Issue` with a selected finding → `i` triages the finding as
  `Ignore` (today's behavior preserved).
- `pane == Issue` with **no** finding selected → `i` is a no-op
  (mode stays `Normal`).

**Depends on**: T.11.5, T.13.3.

**Test first** (RED):
- Extend or new test `crates/agentic-tui/tests/insert_mode.rs`. The test
  cases must each exercise one scope of the dual binding so the
  documented contract is locked in:
  1. With `pane = Logs`, `state.mode = Normal`: press `i`. Assert
     `state.mode == Mode::Insert` and findings state is unchanged.
  2. With `pane = Chat`, `state.mode = Normal`: press `i`. Assert
     `state.mode == Mode::Insert`.
  3. With `pane = Issue` and a finding selected
     (`state.findings.selected = Some(idx)`): press `i`. Assert the
     finding's triage is `Triage::Ignore` and `state.mode` stayed
     `Normal`.
  4. With `pane = Issue` and no finding selected: press `i`. Assert
     `state.mode` stayed `Normal` and findings state is unchanged.
  5. From `Mode::Insert`, press `Esc`. Assert mode reverts to `Normal`.

**Implement** (GREEN):
- Update `AppState::handle_key` to scope the `i` key by current pane per
  the four cases above.

**Refactor**: None.

**Commit**: `feat(tui): scope i key — insert in logs/chat, triage in issue`

**Verification**: `cargo test -p agentic-tui insert_mode`

---

### Step T.13.7: Wire all TUI views into `draw_app`

**Goal**: `draw_app` now lays out, top-to-bottom: title bar (T.10.2) → issue header (T.11.1) → pipeline bar + hint (T.11.2/3) → tab bar (T.11.4) → active body pane (T.12.x) → status line (T.13.3). Help overlay renders above all.

**Depends on**: T.10.2, T.11.1, T.11.2, T.11.3, T.11.4, T.12.1, T.12.2, T.12.3, T.13.3, T.13.5.

**Test first** (RED):
- New integration test `crates/agentic-tui/tests/draw_app_layout.rs`:
  - Render at 140×40 with default state. Assert each region's expected row range:
    - Title bar at row 0.
    - Issue header at row 1.
    - Pipeline bar at rows 2–5.
    - Hint at row 6.
    - Tab bar at row 7.
    - Body 8…N-2.
    - Status line at the last row.
  - Switch pane to Chat; assert the body region's first row contains a chat-message marker.

**Implement** (GREEN):
- Rewrite `draw_app` in `crates/agentic-tui/src/lib.rs` to compose the views in order. Compute heights via `Layout::default().constraints(...)`.
- Remove the old two-pane `compute_panes` from `layout.rs` (or keep it for the body's internal split if a future step needs it).

**Refactor**: Move `cockpit::render` and `chat::render` into `logs_pane`/`chat_pane` so the new `pane` enum maps cleanly. Delete `findings::render` as a top-level pane (it lives inside `issue_pane` action items in a future step).

**Commit**: `feat(tui): compose new layout in draw_app`

**Verification**: `cargo test -p agentic-tui draw_app_layout && cargo test -p agentic-tui`

---

### Step T.13.8: Delete dead TUI views (cockpit, old chat)

**Goal**: Remove the legacy `cockpit.rs` and `chat.rs` files. Their content is now in the new pane modules.

**Depends on**: T.13.7.

**Test first** (RED):
- New test `crates/agentic-tui/tests/dead_views.rs`:
  - Compile-time: `mod cockpit;` and `mod chat;` should not be in `views/mod.rs`.
  - File-system: `crates/agentic-tui/src/views/cockpit.rs` does not exist.

**Implement** (GREEN):
- `git rm` the two files; update `views/mod.rs`.

**Refactor**: None.

**Commit**: `refactor(tui): delete legacy cockpit and chat views`

**Verification**: `cargo test -p agentic-tui`

---

### CP-T: TUI review checkpoint

**Checkpoint**: Stop. Hand back to user.
- Manual smoke: run `cargo run -p agentic-tui` and walk through 1/2/3 panes, `:add architect`, `?`, `:q`.
- All TUI tests green.
- Ready to start Tauri-specific work.

---

## Phase 14 — Tauri dense layout + chrome verification

### Step X.14.1: Wire `dense` prop through AppShell from Tauri detection

**Goal**: `App.tsx` calls `isTauriDense()` and passes `dense={true}` to `AppShell` when detected.

**Depends on**: W.3.2, W.8.1.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/AppDense.test.tsx`:
  - Stub `window.__TAURI_INTERNALS__` to a truthy object.
  - Render `<App />`. Assert the right column resolves to `280px`.
  - Reset; render again with the global unset; assert `340px`.

**Implement** (GREEN):
- Edit `App.tsx` to call `isTauriDense()` and pass through.

**Refactor**: None.

**Commit**: `feat(tauri): apply dense layout when running inside Tauri`

**Verification**: `pnpm -F @agentic/web-ui test AppDense`

---

### Step X.14.2: Verify and lock `tauri.conf.json` window decorations

**Goal**: Resolve the uncommitted `tauri.conf.json` change. Confirm `decorations: true` (default), commit any explicit changes deliberately.

**Depends on**: none.

**Test first** (RED):
- New test `crates/agentic-tauri/tests/conf_decorations.rs`:
  - Read `tauri.conf.json`. Assert `app.windows[0].decorations` is true (or absent — Tauri defaults to true). Assert `app.windows[0].width >= 1200` and `height >= 800`.

**Implement** (GREEN):
- Inspect the uncommitted change; if it removes decorations, restore them. If it merely tunes width/height, accept the change and commit.
- Add the test fixture path to `Cargo.toml` if needed.

**Refactor**: None.

**Commit**: `chore(tauri): lock window decorations and dimensions`

**Verification**: `cargo test -p agentic-tauri conf_decorations`

---

## Phase 15 — Cross-cutting cleanup + tech debt index

### Step X.15.1: Full-workspace test pass + screenshot refresh

**Goal**: Run the full test matrix and update any committed screenshots / fixtures that reflect the redesign.

**Depends on**: W.8.5, T.13.8, X.14.2.

**Test first** (RED):
- N/A — this is a verification step.

**Implement** (GREEN):
- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `pnpm -F @agentic/web-ui test`, `pnpm -F @agentic/web-ui lint`.
- Update any committed `.png` or `.txt` snapshots in `crates/agentic-tui/tests/snapshots/` to match the new TUI rendering.
- Update `MANUAL.md` screenshot references if any.

**Refactor**: None.

**Commit**: `chore: refresh snapshots and screenshots after redesign`

**Verification**: full matrix above.

---

### Step X.15.2: File tech-debt entries for deferred items

**Goal**: Per project rule (CLAUDE.md §4), every deferred item gets both a tech-debt note here AND a GitHub issue (`gh issue create --label tech-debt`). Link each issue back from this todo.

**Depends on**: X.15.1.

**Test first** (RED):
- N/A — administrative step.

**Implement** (GREEN):
- Create one GitHub issue per item below. Update this section with `(GH #N)` after creation.

**Refactor**: None.

**Commit**: `docs(redesign): file tech-debt for deferred redesign items`

**Verification**: `gh issue list --label tech-debt | grep -E "redesign|spec"` returns the new issues.

#### Tech-debt items

1. **Agent configure side-panel UI** (GH #TBD).
   - What's missing: kebab → Configure opens an empty placeholder modal.
   - Why deferred: no backend `pipeline.toml` per-agent override API yet.
   - Trigger: when core ships a `set_agent_config` IPC.

2. **Backend permission-request event** (GH #88).
   - What's missing: real-time permission events from `agentic-core` —
     `PermissionCard` currently renders against a fixture only and there
     is no IPC channel to deliver live `PermissionRequest`/`PermissionDecision`
     envelopes from the runner to the UI.
   - Why deferred: redesign scope is visual / structural; backend events
     are out of scope.
   - Trigger: when the orchestrator gains a permission-gate hook in
     `agentic-core`.

3. **Real ticket-source body in Issue column** (GH #TBD).
   - What's missing: `IssueColumn.body` is always the placeholder `["No description available — …"]`.
   - Why deferred: no `get_ticket(url)` IPC.
   - Trigger: when ticket-source integration ships.

4. **Keyboard drag-reorder for pipeline bar** (GH #89).
   - What's missing: arrow-key reorder with a roving-tabindex pattern (or
     equivalent ARIA listbox semantics via `@dnd-kit/sortable`) for the
     pipeline bar's agent cards.
   - Why deferred: HTML5 DnD covers mouse + touch and keyboard a11y wasn't
     in the design hand-off; adding it is ~1 step of additional work.
   - Trigger: before public release / WCAG 2.1 AA pass.

5. **DiffViewer access from action items** (GH #TBD).
   - What's missing: `DiffViewer` exists but no entry-point in the new shell.
   - Why deferred: out of scope; needs a per-finding "View diff" affordance.
   - Trigger: when a user reports the diff is unreachable.

6. **Real avatar API integration** (GH #TBD).
   - What's missing: header avatar is initials-on-zinc placeholder.
   - Why deferred: no identity backend.
   - Trigger: when OAuth profile fetch ships.

7. **Pipeline editing persistence** (GH #TBD).
   - What's missing: drag-reorder, insert, remove fire callbacks but state lives only in the React tree.
   - Why deferred: backend pipeline-config persistence not yet specified.
   - Trigger: when `pipeline.toml` mutability lands.

8. **Backend ChatMessage `senderAgent` field** (GH #90).
   - What's missing: Rust `ChatMessage` struct + `chat_send_message` IPC don't carry which agent answered; TS `ChatMessage.senderAgent?` is reserved but always undefined for backend-issued assistants. ChatColumn falls back to `agent="assistant"` placeholder, so per-agent tints are homogeneous instead of agent-aware.
   - Why deferred: schema change touches `agentic-core`, the chat SQLite migration, the `agentic-tauri` IPC handler, and the chat-routing layer that knows the answering agent.
   - Trigger: when chat replies are actually orchestrated by the multi-agent pipeline (architect/developer/qa/reviewer) rather than a single-LLM passthrough.

9. **Streaming-row left-border animation** (GH #91).
   - What's missing: ActivityColumn doesn't render the "subtle left-border animation matching agent color" on the currently-streaming event entry per spec §3.5 line 263. No `streamingEventId` / `isStreaming` signal flows through the column props.
   - Why deferred: streaming-state plumbing arrives with the App.tsx integration in Phase 8 (which threads the live event source). The animation class itself is trivial; the prop wiring + live-event correlation is the load-bearing work.
   - Trigger: Phase 8 App integration when ActivityColumn first sees a live event stream.

---

## Status checklist

Phase 0 — Tokens & foundation
- [ ] W.0.1 Inter via Google Fonts CDN + tokens.css
- [ ] W.0.2 Tailwind theme extension
- [ ] W.0.3 useTheme hook
- [ ] W.0.4 pipeline.ts types

Phase 1 — Web header
- [ ] W.1.1 HeaderBar idle
- [ ] W.1.2 HeaderBar running/completed
- [ ] W.1.3 HeaderBar theme toggle wiring

Phase 2 — Web pipeline bar
- [ ] W.2.1 AgentCard
- [ ] W.2.2 Connector
- [x] W.2.3 PipelineBar shell
- [ ] W.2.4 Insert chips
- [ ] W.2.5 AgentPicker popover
- [ ] W.2.6 PipelineBar + AgentPicker integration
- [x] W.2.7 Drag-reorder
- [ ] W.2.8 AgentCard kebab menu

Phase 3 — Web 3-column shell
- [ ] W.3.1 AppShell grid
- [ ] W.3.2 isTauriDense helper

Phase 4 — Web Chat column
- [ ] W.4.1 ChatMessage variants
- [ ] W.4.2 Inline token highlighter
- [x] W.4.3 ChatComposer
- [x] W.4.4 Slash popover
- [x] W.4.5 Mention popover
- [x] W.4.6 ChatColumn integration

Phase 5 — Web Activity column
- [ ] W.5.1 ActivityHeader tabs
- [ ] W.5.2 LogRow
- [ ] W.5.3 ToolCallCard
- [x] W.5.4 ActivityColumn

Phase 6 — Web Issue column
- [ ] W.6.1 IssueColumn shell
- [x] W.6.2 Acceptance completed state
- [x] W.6.3 Action items section
- [x] W.6.4 findingsToActionItems adapter
- [x] W.6.5 SpecDialog modal
- [ ] W.6.6 Create spec → start_ticket_run

Phase 7 — Web permission card
- [ ] W.7.1 PermissionCard
- [ ] W.7.2 ActivityColumn renders PermissionCard

Phase 8 — Web App.tsx swap
- [ ] W.8.1 Swap to AppShell
- [ ] W.8.2 SettingsModal + GeneralTab + HistoryTab
- [ ] W.8.3 Wire SettingsModal into App.tsx
- [x] W.8.4 Restyle DismissableBanner/DiffViewer
- [x] W.8.5 Delete dead components
- [ ] CP-W

Phase 9 — Polish
- [ ] W.9.1 Wire pipeline mutation handlers in App.tsx
- [ ] W.9.2 Per-agent SVG icon library + render in card/picker/message
- [ ] W.9.3 ChatComposer layout polish (chips below, paper-plane, placeholder)
- [ ] W.9.4 ChatComposer New-spec affordance
- [ ] W.9.5 HeaderBar settings gear icon
- [ ] W.9.6 StatusDot component + use in AgentCard
- [ ] W.9.7 IssueColumn header polish (run-state pill + section labels)
- [ ] W.9.8 App.tsx polish integration test
- [ ] CP-W2

Phase 10 — TUI palette + title bar
- [ ] T.10.1 Theme module
- [ ] T.10.2 Title bar

Phase 11 — TUI pipeline + tabs
- [ ] T.11.1 Issue header
- [ ] T.11.2 Pipeline bar boxes
- [ ] T.11.3 Pipeline hint
- [ ] T.11.4 Tab bar
- [ ] T.11.5 1/2/3 keys

Phase 12 — TUI body panes
- [ ] T.12.1 Logs pane
- [ ] T.12.2 Chat pane
- [ ] T.12.3 Issue pane

Phase 13 — TUI permission + status line
- [ ] T.13.1 Permission card
- [ ] T.13.2 y/s/n keys
- [ ] T.13.3 Status line + modes
- [ ] T.13.4 Flash lifecycle
- [ ] T.13.5 Help overlay
- [ ] T.13.6 Insert mode
- [ ] T.13.7 draw_app composition
- [ ] T.13.8 Delete dead views
- [ ] CP-T

Phase 14 — Tauri
- [ ] X.14.1 Wire dense
- [ ] X.14.2 Lock window decorations

Phase 15 — Cleanup + tech debt
- [ ] X.15.1 Full test pass + snapshots
- [ ] X.15.2 File tech-debt issues

---

## Resolved design decisions

The six open questions have been resolved by the user (2026-04-29). For
historical reference:

1. **Inter font hosting** — Load via **Google Fonts CDN**; do not commit a
   `.ttf` asset. Implemented in W.0.1 (`<link>` tags in
   `apps/web-ui/index.html`).
2. **PastRunsPane placement** — Lives inside a tabbed `SettingsModal`
   (General + History tabs); the header bar carries no separate History
   button. Implemented in W.8.2 + W.8.3.
3. **TUI `i` key conflict** — Confirmed: dual binding scoped by pane.
   Implemented in T.13.6 with explicit test cases for each scope.
4. **Backend permission event variant** — Fixture-only for this redesign;
   tech-debt entry filed (§ Tech-debt item 2). Trigger: when the
   orchestrator gains a permission-gate hook in `agentic-core`.
5. **Drag-reorder keyboard accessibility** — Deferred as a feature
   request; tech-debt entry filed (§ Tech-debt item 4). Trigger: before
   public release / WCAG 2.1 AA pass.
6. **Cross-platform TUI traffic lights** — Decorative everywhere. No
   `cfg(target_os = "...")` gating; documented in spec §4.2.
