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

### [x] Step W.9.2: Add per-agent SVG icon library + render in AgentCard / AgentPicker / ChatMessage

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

### Step W.9.4: ChatComposer "New spec" affordance ✅

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

### [x] Step T.10.1: New `theme` module with color constants

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

### [x] Step T.10.2: Title bar (28 px, traffic lights + centered text)

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

### [x] Step T.11.1: Issue header strip

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

### [x] Step T.11.3: Pipeline footer hint

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

### Step T.12.3: Issue pane — id, title, labels, description, acceptance ✓ COMPLETE

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

### Step T.13.3: Mode indicator in status line [x]

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

### Step T.13.4: Flash messages on the status line ✓ DONE

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

2. **Backend permission-request event** (GH #88 — closed by Phase P).
   - Shipped: observational permission gate per Q3.c. Backend emits
     `Event::PermissionRequest` and `Event::PermissionResolved` envelopes
     from the orchestrator's per-`ToolUseStart` hook. Web UI (Tauri) renders
     live cards via `usePermissionRequests`; user clicks dispatch
     `permission_decide` IPC; gate consumes `PermissionResolved` from the
     bus. TUI applies the same envelopes into `state.pending_perms` (P.5.1).
   - Limitations (v1, expected): tool calls already executed by the time
     the gate sees them (`--dangerously-skip-permissions` constraint).
     Session allowlist blocks future prompts but cannot un-execute the
     call that produced the prompt.
   - Follow-ups (filed):
     - **GH #105** — Real blocking permission gate (MCP/proxy intercept).
     - **GH #103** — TUI runner integration: y/s/n keys publish via bus.
     - **GH #102** — SessionAllowlist memory leak on abnormal run exit.
     - **GH #104** — Backend tracing::error logs not surfaced to Activity.

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

10. **TUI issue header: ellipsis truncation** (GH #96).
    - What's missing: `crates/agentic-tui/src/views/issue_header.rs` doesn't truncate the title with `…` on overflow per spec §4.3. `pad_width` underflows via `saturating_sub` and ratatui clips at the right edge.
    - Why deferred: proper Unicode-aware truncation needs `unicode-width` (or ratatui `Block::title` trim) + a tested boundary policy — warrants a focused sub-step rather than a review-fix tack-on.
    - Trigger: before T.11.2 (ASCII pipeline bar) lands.

11. **TUI issue header: failed-state pill color test** (GH #97).
    - What's missing: tests only cover the `running → BLUE` pill. When `AppState.run_status` lands in T.13.x, `completed → GREEN` and `failed → RED` will need test coverage.
    - Why deferred: the `run_status` field doesn't exist yet; scaffolding it without a producer is premature.
    - Trigger: alongside T.13.x runner wiring when `run_status` is added.

12. **TUI Pane::Issue body placeholder** (GH #98 — closed by T.12.3).
    - RESOLVED: T.12.3 restructured draw_app to single-pane dispatch and added views::issue_pane with the full spec §4.6 renderer (ACCENT id, bold title, ▏chips▕, description paragraphs, [ ] acceptance checklist). The tab indicator is now in sync with body content.

13. **TUI logs pane: Finding events as WARN log rows** (GH #99).
    - What's missing: `findings::render` is rendered as a separate widget below the log rows. Spec §4.6 line 476 specifies that Finding events should appear AS log rows at `LogLevel::Warn`, not as a sidebar widget.
    - Why deferred: the translation requires runner integration (T.13.x) — the event-application path must push `Finding` envelopes as `LogEntry { level: LogLevel::Warn, ... }`. The plumbing for T.13.x to consume is in place (`LogEntry`, `LogLevel::Warn`, `pub log: Vec<LogEntry>` are all public).
    - Trigger: T.13.x runner→AppState bridge. Remove `findings::render` call from `logs_pane::render` at the same time.

14. **TUI logs pane: vertical scroll for long sessions** (GH #100).
    - What's missing: `logs_pane::render` has no scroll offset. When `state.log.len() > area.height`, rows beyond the visible area are silently dropped.
    - Why deferred: T.12.1 ships the visual contract (column-aligned rows). Scroll behavior isn't in the spec for this step and isn't visible until T.13.x produces real event volume.
    - Trigger: when runner produces enough events to fill the pane — add scroll offset, j/k navigation, and a "+N earlier" indicator.

15. **TUI help overlay: mouse-click dismissal** (GH #101).
    - What's missing: `help_overlay` only dismisses on Esc. Spec §4.9 + hand-off (`tui-view.jsx:252`) specify "Esc or any click dismisses".
    - Why deferred: mouse handling in ratatui requires plumbing `MouseEvent` through the keyboard event loop, including hit-testing against the modal Rect. Non-trivial; keyboard-driven Esc covers the core flow.
    - Trigger: before T.13.8 integration milestone, OR earlier if a user reports it.

16. **Permission gate: SessionAllowlist memory leak on abnormal run exit** (GH #102).
    - What's missing: `permissions/session.rs` clears entries via the RunComplete listener task. If a run terminates abnormally (crash, cancellation that bypasses RunComplete, error path), entries for that run_id remain until process exit. Long-lived processes (Tauri) accumulate stale entries.
    - Why deferred: v1-acceptable; entries are tiny and Tauri restarts naturally on shutdown. Premature mitigation muddies the v1 contract.
    - Trigger: when the gate is observed reused across many runs without restart in production, OR when memory profiling shows SessionAllowlist as a measurable retainer. Approach: TTL, run-count LRU, or hard cap.

17. **TUI runner integration: y/s/n keys publish decisions to bus** (GH #103).
    - What's missing: T.13.2's y/s/n keys mutate `state.flash` + `state.pending_perms` locally only. They do NOT publish `Event::PermissionResolved` back through the bus, so the orchestrator's AsyncGate (P.2.x) has no way to receive a decision from the TUI side. Additionally, `app::PermissionRisk` and `events::PermissionRisk` are structurally identical; the local enum should be dropped and the wire enum re-exported when egress lands.
    - Why deferred: the TUI does not yet have a runtime that subscribes to / publishes on the agentic-core EventBus. P.5.1 ships envelope INGESTION (PermissionRequest → pending_perms; PermissionResolved → remove) but not egress.
    - Trigger: when the TUI gains a runtime (TUI as standalone agent runner, or TUI subscribes to agentic-tauri's bus via IPC). What's needed: (1) a bus handle on AppState (Arc<EventBus> or similar), (2) handle_key for y/s/n constructs an EventEnvelope::PermissionResolved with the matching request_id and publishes to bus, (3) tests that mock the bus to verify the publish happens.

18. **App.handleRunPipeline silently swallows pre-flight errors** (GH #106).
    - What's missing: the HeaderBar "Run pipeline" button calls `start_ticket_run` directly via `App.handleRunPipeline`; on rejection (e.g. binary not found) the catch is a `/* no-op */`. F.1.4 surfaced pre-flight errors via the `/plan` slash-command path (which already had `systemMessages` access), but the button path still swallows.
    - Why deferred: surfacing the error from `App.handleRunPipeline` requires hoisting `systemMessages` state from `ChatPane` to `App` or adding a callback prop — invasive restructuring for a placeholder button.
    - Trigger: Run-pipeline button graduates to a SpecDialog-driven flow (W.8.x successor), OR a global toast surface lands first.

19. **YAML frontmatter metadata flow-through in agent discovery** (GH #107).
    - What's missing: `discover_agent_with_home` falls back to a stub `Agent` when it encounters YAML `---` frontmatter (commit `5700509`). The fallback carries `system_prompt = full file content` but leaves `model`, `tools`, `allowed_questions`, `pipeline_role`, `timeout_seconds` at their defaults. Picker (`list_discoverable`) also shows `description = None` for YAML files.
    - Why deferred: Copilot CLI and Claude Code read the agent file directly from disk; the Rust `Agent` struct metadata is not load-bearing for the current pipeline. Adding a YAML parser (`serde_yaml`) touches the `agentic-core` dependency surface — that's a separate decision.
    - Trigger: when the orchestrator needs to route on `model`/`tools`/`pipeline_role` from YAML agent files (i.e. when Rust—not the subprocess—must act on metadata), or when a user requests description display for YAML files in the picker.

---

## Phase P — Backend permission stream (GH #88)

This phase wires a backend permission gate into `agentic-core` and
streams `PermissionRequest` / `PermissionResolved` envelopes onto the
existing `EventBus` so the web UI, TUI, and Tauri shell can present
real prompts instead of fixtures. The gate is **observational** in
v1: because we run the underlying CLIs with `--dangerously-skip-permissions`
(Claude) / equivalent (Copilot), tool calls have already executed by
the time the orchestrator sees `Event::ToolUseStart`. The gate
therefore annotates after the fact — allowlist hits are recorded as
`AllowOnce`, denylist hits emit a `Deny` decision plus a warning log
(advisory only, since the call already ran), and unknown patterns
emit a `PermissionRequest` envelope and await user input. A `[s]
session` decision adds the matched pattern to a per-run allowlist that
the gate consults on subsequent calls (so future identical patterns
auto-allow without prompting). A real blocking gate requires routing
tool calls through an MCP server or process proxy that can pause the
child process before it executes — that work is filed as a follow-up
tech-debt GH issue ("Real blocking permission gate (MCP/proxy
intercept)") in P.6.2 and closes the in-progress reference on
tech-debt entry 2 above.

Scope summary: 4 backend event/config sub-steps (P.1.x), 4 gate +
orchestrator sub-steps (P.2.x), 2 Tauri IPC sub-steps (P.3.x), 3 web
UI sub-steps (P.4.x), 1 TUI envelope-routing sub-step (P.5.1), 2 E2E +
cleanup sub-steps (P.6.x). Total: 16 atomic steps. Each step targets a
single TDD cycle (30 min – 2 hr) and must result in a green commit
ahead of the next step.

Tech-debt entry 2 (GH #88) is updated above to reference this phase
and stays open until P.6.1 lands; P.6.2 then files the follow-up
"real blocking gate" issue and closes #88 with a comment pointing at
the new issue.

### Step P.1.1: Add `PermissionRequest` + `PermissionResolved` event variants

**Goal**: Extend `Event` (and the persistence tag table) with two
additive variants that carry permission-gate signalling on the bus.
No producer yet — variants must round-trip through MessagePack and
through the JSON IPC layer.

**Depends on**: nothing (pure data-model addition).

**Test first** (RED):
- New unit tests in `crates/agentic-core/src/events/mod.rs` (next to
  existing variant round-trip tests):
  - `permission_request_round_trips_msgpack`: build an envelope with
    `Event::PermissionRequest { request_id: "req-01J...", agent:
    "developer".into(), tool: "Bash".into(), arg: "rm -rf
    node_modules".into(), scope: "shell.destructive".into(), risk:
    PermissionRisk::High, reason: "destructive shell".into() }`,
    encode via `rmp_serde::to_vec_named`, decode, assert equality.
  - `permission_resolved_round_trips_msgpack`: same shape with
    `Event::PermissionResolved { request_id: "req-01J...", decision:
    PermissionDecision::AllowOnce, source: PermissionSource::User }`.
  - `permission_request_serializes_to_json_kebab_case`: assert
    `serde_json::to_value(&envelope).unwrap()` contains
    `"type": "PermissionRequest"` and that nested enum fields use
    snake_case discriminants (matches existing convention — confirm
    by inspection against `Event::Finding` snapshot).
- Extend `crates/agentic-core/src/events/persist.rs` test suite: add a
  case to the existing `event_type_tag` round-trip / coverage test
  asserting both new variants tag as `"PermissionRequest"` and
  `"PermissionResolved"`.

**Implement** (GREEN):
- In `crates/agentic-core/src/events/mod.rs`:
  - Add `pub enum PermissionRisk { Low, Medium, High }` with
    `#[serde(rename_all = "snake_case")]`.
  - Add `pub enum PermissionDecision { AllowOnce, AllowSession, Deny,
    TimedOut }` with `#[serde(rename_all = "snake_case")]`.
  - Add `pub enum PermissionSource { User, AllowlistConfig,
    DenylistConfig, SessionAllowlist, Timeout }` with `#[serde(rename_all
    = "snake_case")]`.
  - Add `Event::PermissionRequest { request_id: String, agent: String,
    tool: String, arg: String, scope: String, risk: PermissionRisk,
    reason: String }`.
  - Add `Event::PermissionResolved { request_id: String, decision:
    PermissionDecision, source: PermissionSource }`.
- In `crates/agentic-core/src/events/persist.rs::event_type_tag`,
  extend the match: `Event::PermissionRequest { .. } =>
  "PermissionRequest"`, `Event::PermissionResolved { .. } =>
  "PermissionResolved"`.

**Refactor**: Confirm `CURRENT_SCHEMA_VERSION` does not need bumping
(additive variants are backward-compatible per the comment at line
14–18 of `events/mod.rs`). Document this in a `// schema: additive`
comment beside the new variants.

**Commit**: `feat(core): add PermissionRequest + PermissionResolved event variants`

**Verification**: `cargo test -p agentic-core --features all events::`

---

### Step P.1.2: `permissions.toml` config loader

**Goal**: Stand up a `PermissionsConfig` struct loaded from a
**separate** `permissions.toml` file (sibling of `pipeline.toml`, not
nested inside it). Carries `[allowlist]`, `[denylist]`, and a
`[settings]` block with `default_on_timeout` (`"deny"` | `"allow"`,
default `"deny"`).

**Depends on**: P.1.1.

**Test first** (RED):
- New module + tests `crates/agentic-core/src/pipeline/permissions/config.rs`
  (or `crates/agentic-core/src/permissions/config.rs` — choose location
  during P.2.1; pick one and stick to it):
  - `loads_minimal_config`: write
    ```toml
    [allowlist]
    patterns = ["Read(*)", "LS(*)"]

    [denylist]
    patterns = ["Bash(rm -rf /*)"]

    [settings]
    default_on_timeout = "deny"
    ```
    to a tempdir, call `PermissionsConfig::load(path)`, assert two
    allow patterns and one deny pattern parsed.
  - `defaults_when_file_missing`: call `PermissionsConfig::load` on a
    non-existent path, assert it returns `PermissionsConfig::builtin_default()`
    (no error). Defaults must contain a Claude-tool baseline (`Read`,
    `LS`, `Grep`, `Glob` all `*`-matched in allowlist) AND a Copilot-tool
    baseline (`view`, `ls`, `grep`, `find` — verify the actual Copilot
    tool names from `crates/agentic-core/src/backends/copilot_cli/parser.rs`
    during impl). High-risk Bash patterns (`rm -rf`, `sudo`, `kubectl
    delete`, `git reset --hard`, `git push --force`) ship in the denylist.
  - `rejects_invalid_pattern`: a pattern with regex syntax (`Bash(/.*/)`)
    must return `PermissionsConfigError::InvalidPattern` (we explicitly
    do not support regex per Q2).
  - `default_on_timeout_round_trips`: settings block with
    `default_on_timeout = "allow"` parses to
    `OnTimeout::Allow`; `"deny"` to `OnTimeout::Deny`; missing → defaults
    to `Deny`.

**Implement** (GREEN):
- Add `[dependencies] serde = ...` (already present), `toml = ...`
  (check `Cargo.toml`; add if missing).
- Define `PermissionsConfig`, `PermissionRule`, `OnTimeout`, error
  enum.
- `builtin_default()` populates the per-backend tool tables. Embed
  the actual tool names from
  `crates/agentic-core/src/pipeline/tool_use_observer.rs:113`
  (`Edit`, `Write`, `MultiEdit`, `create`, `str_replace`) **plus** the
  read-only / navigation tools from the Claude allow-list seen at
  `backends/claude_code/mod.rs:378` (`Read`, `Edit`, `Bash`) and the
  Copilot tool names from `backends/copilot_cli/parser.rs`. Document
  the source of each name in a `// from:` comment for grep-ability.
- Use the matcher from P.1.3 once it exists (RED here passes a stub
  that just checks the pattern parses, GREEN in P.1.3 makes
  `matches()` work).

**Refactor**: Move the file to its final location (`crates/agentic-core/src/permissions/`)
if you didn't pick that in the RED phase. Add a `mod permissions;` to
`lib.rs`.

**Commit**: `feat(core): add PermissionsConfig with builtin tool defaults`

**Verification**: `cargo test -p agentic-core permissions::config`

---

### Step P.1.3: Tool matcher (`<tool>(<arg-glob>)` and `<tool>:*`)

**Goal**: Pure matcher that takes `(tool_name, arg)` and a pattern and
returns bool. Supports two pattern shapes only:
- `<tool>(<arg-glob>)` where `<arg-glob>` uses shell glob syntax
  (`*`, `?`, `[abc]`) on the entire arg string. No anchoring needed —
  the parens are the anchor.
- `<tool>:*` matches any arg for that tool.

No regex, no negation, no captures (Q2).

**Depends on**: P.1.1.

**Test first** (RED):
- New tests `crates/agentic-core/src/permissions/matcher.rs`:
  - `tool_wildcard_matches_any_arg`: pattern `Bash:*` matches
    `("Bash", "ls -la")` and `("Bash", "")`. Does not match
    `("Read", "/tmp/x")`.
  - `arg_glob_basic`: pattern `Bash(rm -rf *)` matches `("Bash", "rm
    -rf node_modules")`. Does not match `("Bash", "ls")`.
  - `arg_glob_question_mark`: pattern `Read(/tmp/?.txt)` matches
    `("Read", "/tmp/a.txt")` but not `("Read", "/tmp/ab.txt")`.
  - `arg_glob_star_does_not_cross_quotes_no_special_handling`:
    pattern `Bash(rm * /tmp)` matches the arg as a flat string with
    no shell tokenization — document this explicitly.
  - `unknown_pattern_shape_errors`: parsing `Bash` (no parens) returns
    `PatternParseError::Malformed`. Parsing `Bash(/regex/)` parses but
    treats the slashes as literal characters.
  - `tool_name_is_case_sensitive`: pattern `bash:*` does not match
    `("Bash", ...)` (Q2 — no case folding).

**Implement** (GREEN):
- Add `glob = "..."` (or use the lighter-weight `globset` already
  potentially in the workspace — verify with
  `cargo tree -p agentic-core | rg -i glob`).
- Implement `Pattern::parse(&str) -> Result<Pattern, PatternParseError>`
  and `Pattern::matches(tool: &str, arg: &str) -> bool`.
- Wire the matcher into `PermissionRule` so `PermissionsConfig::load`
  validates patterns at parse time (P.1.2's `rejects_invalid_pattern`
  test now goes from "stub returns ok" to "matcher rejects").

**Refactor**: Document the matcher grammar at the top of `matcher.rs`
in a doc-comment block — this becomes the user-facing reference for
`permissions.toml`.

**Commit**: `feat(core): add permission tool matcher`

**Verification**: `cargo test -p agentic-core permissions::`

---

### Step P.1.4: Risk classifier table

**Goal**: Heuristic v1 risk classifier embedded in the gate. Given
`(tool, arg)` returns `PermissionRisk`. Used to populate
`Event::PermissionRequest.risk`. Per Q11, a fixed table with the
following rules in priority order:
- Match against denylist High patterns: `Bash(rm -rf *)`,
  `Bash(sudo *)`, `Bash(kubectl delete *)`, `Bash(git reset --hard*)`,
  `Bash(git push --force*)`, `Bash(* | sh)` → **High**.
- Tool family `Bash(*)` not matched above → **Medium**.
- File-write tools (`Write`, `Edit`, `MultiEdit`, `create`,
  `str_replace`) → **Medium**.
- Everything else (`Read`, `LS`, `Grep`, `Glob`) → **Low**.

**Depends on**: P.1.3.

**Test first** (RED):
- New tests `crates/agentic-core/src/permissions/risk.rs`:
  - `bash_rm_rf_is_high`: classify `("Bash", "rm -rf node_modules")`
    → `High`.
  - `bash_plain_ls_is_medium`: classify `("Bash", "ls -la")` →
    `Medium`.
  - `read_is_low`: classify `("Read", "/tmp/x")` → `Low`.
  - `write_is_medium`: classify `("Write", "/tmp/x")` → `Medium`.
  - `unknown_tool_falls_back_to_low`: classify `("CustomTool", "...")`
    → `Low`.

**Implement** (GREEN):
- `pub fn classify(tool: &str, arg: &str) -> PermissionRisk` —
  internally reuses the matcher from P.1.3 against a
  `static [(Pattern, PermissionRisk)]` table. The fact that the table
  is duplicated between this module and the user's denylist is
  intentional (Q11): the user's denylist controls the *gate decision*,
  the risk table controls the *risk pill displayed to the user*.
  Document the duplication.

**Refactor**: None.

**Commit**: `feat(core): add v1 permission risk classifier`

**Verification**: `cargo test -p agentic-core permissions::risk`

---

### Step P.2.1: `PermissionGate` trait + `ConfigGate` static implementation ✓

**Goal**: Define the gate trait that the orchestrator will call on
every `Event::ToolUseStart` it consumes. Provide a static
implementation that consults only `PermissionsConfig` (no async
prompt yet — P.2.2 adds the channel).

**Depends on**: P.1.2, P.1.3, P.1.4.

**Test first** (RED):
- New tests `crates/agentic-core/src/permissions/gate.rs`:
  - `allowlist_hit_returns_allow_once`: gate built from a config with
    `Read(*)` allowlisted; calling
    `gate.evaluate("Read", "/tmp/x")` returns
    `GateOutcome::AnnotateAllow { source: AllowlistConfig }`.
  - `denylist_hit_returns_deny`: gate with `Bash(rm -rf *)` denylisted;
    `gate.evaluate("Bash", "rm -rf node_modules")` returns
    `GateOutcome::AnnotateDeny { source: DenylistConfig }`.
  - `unknown_tool_returns_prompt`: gate with neither rule matching
    `("CustomTool", "x")` returns
    `GateOutcome::Prompt { risk: PermissionRisk::Low }` (risk via
    P.1.4).
  - `denylist_takes_precedence_over_allowlist`: pattern overlap (rare
    but the contract must be explicit) → deny wins.

**Implement** (GREEN):
- Define
  ```rust
  pub trait PermissionGate {
      fn evaluate(&self, tool: &str, arg: &str) -> GateOutcome;
  }
  pub struct ConfigGate { config: PermissionsConfig }
  pub enum GateOutcome {
      AnnotateAllow { source: PermissionSource },
      AnnotateDeny { source: PermissionSource },
      Prompt { risk: PermissionRisk },
  }
  ```
- Implement `ConfigGate::new(config)` and `evaluate`.

**Refactor**: None.

**Commit**: `feat(core): add PermissionGate trait + ConfigGate`

**Verification**: `cargo test -p agentic-core permissions::gate`

---

### Step P.2.2: Decision channel + async `evaluate_async`

**Goal**: Add an async path that, when the gate decides to prompt,
emits `Event::PermissionRequest` on the bus and waits for a matching
`Event::PermissionResolved` on a per-request `oneshot::Receiver`.
60-second timeout, configurable per-config (Q4): on timeout, emit a
synthetic `PermissionResolved { decision: TimedOut, source: Timeout }`
and resolve as `default_on_timeout`.

**Depends on**: P.2.1, P.1.1.

**Test first** (RED):
- New tests `crates/agentic-core/src/permissions/gate_async.rs`:
  - `prompt_emits_permission_request_envelope`: spawn a test bus,
    call `gate.evaluate_async("CustomTool", "x", &bus, run_id,
    step_id)`, await the request envelope on a subscriber, assert
    payload fields (request_id is a fresh ULID, agent comes from a
    constructor arg, scope is derived from tool family).
  - `decision_resolves_pending_request`: while a call to
    `evaluate_async` is awaiting, publish
    `Event::PermissionResolved { request_id: <same as emitted>,
    decision: AllowOnce, source: User }`. The future resolves to
    `GateOutcome::AnnotateAllow { source: User }`.
  - `mismatched_request_id_is_ignored`: publish a Resolved with a
    different request_id; the future stays pending until the right
    one arrives.
  - `timeout_resolves_to_deny_by_default`: use a 50 ms test timeout
    override, never publish a decision, await result;
    `GateOutcome::AnnotateDeny { source: Timeout }`. Verify a
    synthetic `PermissionResolved { decision: TimedOut, source:
    Timeout }` was published on the bus (so persist + UI see it).
  - `timeout_resolves_to_allow_when_configured`: same with
    `default_on_timeout = Allow` →
    `GateOutcome::AnnotateAllow { source: Timeout }`.
  - `cancellation_drops_pending`: spawn a cancel token, abort it
    before publishing a decision; the future returns
    `GateOutcome::AnnotateDeny { source: Cancelled }` (a new
    `PermissionSource::Cancelled` variant — add in P.1.1's variant
    set if not already there; if not, file as a tiny RED follow-up
    here).

**Implement** (GREEN):
- Maintain a `Arc<Mutex<HashMap<RequestId, oneshot::Sender<...>>>>`
  inside the async gate.
- On `evaluate_async`:
  1. Run the sync `evaluate` from P.2.1.
  2. If `Prompt`: mint request_id, register a oneshot, publish
     `Event::PermissionRequest` to the bus, `tokio::select!` between
     timeout / cancel / oneshot.
- Subscribe to bus on construction (or per-call — pick the lighter
  path; the test for `mismatched_request_id_is_ignored` is the
  contract).
- The constructor takes a `tokio::time::Duration` so tests can
  inject a 50 ms timeout. Production callers pass
  `Duration::from_secs(60)`.

**Refactor**: Extract the timeout / cancel `select!` into a
`wait_for_decision` helper.

**Commit**: `feat(core): add async permission gate with prompt + timeout`

**Verification**: `cargo test -p agentic-core permissions::gate_async`

---

### Step P.2.3: Per-run session allowlist

**Goal**: Augment the async gate with a per-run in-memory allowlist
of patterns added via `decision == AllowSession`. Cleared when the
gate observes `Event::RunComplete` for the owning run. Does not
persist to disk.

**Depends on**: P.2.2.

**Test first** (RED):
- Extend `gate_async.rs` tests:
  - `session_decision_caches_pattern_for_subsequent_calls`: first
    call to `("Bash", "ls -la")` prompts; user resolves with
    `AllowSession`. Second call to identical args returns
    `AnnotateAllow { source: SessionAllowlist }` without prompting
    (no new `PermissionRequest` envelope on the bus — assert the
    subscriber sees only the original request, not a second).
  - `session_pattern_canonicalizes_to_exact_match`: session entry
    for `("Bash", "ls -la")` does NOT match `("Bash", "ls -la
    /tmp")`. Document explicitly: session allowlist is exact-arg,
    not glob (Q2 — keep the matcher minimal here).
  - `run_complete_clears_session_allowlist`: publish
    `Event::RunComplete` on the bus; subsequent identical call
    prompts again.
  - `cross_run_isolation`: two runs with different `run_id`s share
    no session state.

**Implement** (GREEN):
- Add `Arc<Mutex<HashMap<RunId, HashSet<(String, String)>>>>` to the
  gate.
- On `Decision::AllowSession`, insert before publishing
  `PermissionResolved`.
- On `Event::RunComplete`, drop the entry for that run_id.
- The sync `evaluate` path is unchanged — only `evaluate_async`
  consults session state.

**Refactor**: Pull the session-allowlist HashMap into a struct with
`insert(run_id, tool, arg)` / `contains(run_id, tool, arg)` /
`drop_run(run_id)` — easier to reason about than a raw Mutex.

**Commit**: `feat(core): add per-run session allowlist to permission gate`

**Verification**: `cargo test -p agentic-core permissions::gate_async`

---

### Step P.2.4: Wire gate into `PipelineOrchestrator` ✓ DONE

**Goal**: The orchestrator (which already consumes the bus) now also
consults the permission gate on every `Event::ToolUseStart`. This is
the producer side of `Event::PermissionRequest` and
`Event::PermissionResolved` envelopes; downstream consumers
(`EventPersister`, the Tauri forwarder, the TUI app) just see the
envelopes flow through.

**Depends on**: P.2.3.

**Test first** (RED):
- New integration tests
  `crates/agentic-core/src/pipeline/orchestrator.rs` test module
  (or `tests/orchestrator_permissions.rs` if test-bus plumbing is
  already there):
  - `tool_use_start_with_allowlist_hit_emits_permission_resolved`:
    spawn an orchestrator with a config that allows `Read(*)`;
    publish `Event::ToolUseStart { tool_name: "Read", input:
    json!({"file_path": "/tmp/x"}), .. }`; assert exactly one
    `Event::PermissionResolved { decision: AllowOnce, source:
    AllowlistConfig }` is published with a fresh `request_id`. No
    `PermissionRequest` envelope is published (allowlist short-circuits
    the prompt path, per Q3.c — but emits a Resolved for audit-log
    parity).
  - `tool_use_start_with_denylist_hit_emits_permission_resolved_deny_plus_warn_log`:
    same with denylist hit; assert
    `Event::PermissionResolved { decision: Deny, source:
    DenylistConfig }` and a
    `tracing::warn!` entry in `tracing_subscriber::fmt::TestWriter`
    capture (use the `tracing-test` crate).
  - `tool_use_start_with_no_match_emits_permission_request`:
    config has neither match; assert one
    `Event::PermissionRequest` envelope is published (and stays
    pending — no Resolved until P.2.2's user-decision channel
    fires).
  - `non_tool_use_events_pass_through`: publishing
    `Event::TextDelta` to the bus does **not** invoke the gate (no
    Permission* envelopes emitted).

**Implement** (GREEN):
- Extend `apply_event` in `orchestrator.rs` to short-circuit
  `Event::ToolUseStart` into an async gate call — but the existing
  `apply_event` is sync. Pick one of two paths and document in the
  commit message:
  - **(a)** Spawn a per-request `tokio::spawn` for each ToolUseStart
    and let the gate call `await`. Keeps the orchestrator loop
    non-blocking but breaks ordering relative to the next event on
    the same step.
  - **(b)** Make the orchestrator loop async-aware and call
    `gate.evaluate_async(...).await` inline. Simpler but blocks
    persistence of subsequent events behind the gate's 60 s
    timeout.

  Recommendation: **(a)** — preserves the existing event-stream
  ordering invariants and matches the observational nature of the
  gate.
- The gate is constructed once at `PipelineOrchestrator::spawn` time
  and shared via `Arc<dyn AsyncPermissionGate>`.

**Refactor**: If `apply_event` grows, extract the
`Event::ToolUseStart` arm into a `handle_tool_use_start` free
function.

**Commit**: `feat(core): wire permission gate into PipelineOrchestrator`

**Verification**:
- `cargo test -p agentic-core orchestrator::permissions`
- `cargo clippy --workspace --all-features --all-targets -- -D warnings`

---

### Step P.3.1: Tauri IPC — `permission_decide` command

**Goal**: Expose a Tauri `invoke`able command that the web UI calls
when the user clicks Allow / Allow for session / Deny on a permission
prompt. The command publishes `Event::PermissionResolved` onto the
bus, which the gate then consumes.

**Depends on**: P.2.4.

**Test first** (RED):
- New tests `crates/agentic-tauri/src/commands/permissions.rs` test
  module:
  - `permission_decide_publishes_resolved_envelope`: build an
    `EventBusState`, subscribe a test consumer, call
    `permission_decide(state, request_id: "req-x", decision: "once")`,
    assert one `Event::PermissionResolved { request_id: "req-x",
    decision: AllowOnce, source: User }` envelope received.
  - `permission_decide_session_value`: same with `decision: "session"`
    → `AllowSession`.
  - `permission_decide_deny_value`: `decision: "deny"` → `Deny`.
  - `permission_decide_invalid_value_returns_err`: `decision:
    "fhqwhgads"` returns `Err("invalid decision")` and publishes no
    envelope.
  - `permission_decide_returns_quickly`: command resolves within
    50 ms (no awaiting on the gate's outcome — fire-and-forget
    publish). Use a `tokio::time::timeout` wrapper.

**Implement** (GREEN):
- Add `crates/agentic-tauri/src/commands/permissions.rs` with
  `#[tauri::command] pub async fn permission_decide(state:
  State<'_, EventBusState>, request_id: String, decision: String,
  run_id: String, step_id: Option<String>) -> Result<(), String>`.
- Map `"once" | "session" | "deny"` to enum variants; reject other
  strings.
- `state.bus.publish(EventEnvelope::now(run_id, step_id,
  Event::PermissionResolved { ... }))`.
- Register the command in `crates/agentic-tauri/src/commands/mod.rs`
  and in the builder's `.invoke_handler(...)` list.

**Refactor**: None.

**Commit**: `feat(tauri): add permission_decide IPC command`

**Verification**: `cargo test -p agentic-tauri commands::permissions`

---

### Step P.3.2: Bus forwarder transparently propagates Permission* envelopes ✓

**Goal**: Verify that the existing Tauri event-forwarder
(`subscribe_events` in `crates/agentic-tauri/src/commands/events.rs`)
routes the new envelopes to the webview without modification, and add
a regression test so refactors of the forwarder don't accidentally
filter them out.

**Depends on**: P.3.1.

**Test first** (RED):
- Extend `crates/agentic-tauri/src/commands/events.rs` test module:
  - `forwards_permission_request_envelope`: publish a
    `PermissionRequest` envelope to the bus, capture the emitted
    Tauri event, assert serialization round-trips
    `request_id`/`tool`/`arg`/`risk`.
  - `forwards_permission_resolved_envelope`: same with `Resolved`.
- Both tests use the existing `MockApp` / mock-emitter infrastructure
  (verify by searching `events.rs` for the existing pattern).

**Implement** (GREEN):
- Likely no production change required (the forwarder is
  envelope-shape-agnostic). If a serialization test fails because of
  enum representation, fix at the `serde` attribute level on the
  variants in P.1.1.
- Update `apps/web-ui/src/types/event.ts` to add the new event-type
  literals and discriminated-union members. (TypeScript-side; verify
  `EventEnvelope` is the discriminated-union root.)

**Refactor**: None.

**Commit**: `test(tauri): regression-test permission envelope forwarding`

**Verification**: `cargo test -p agentic-tauri commands::events`

---

### Step P.4.1: `usePermissionRequests` hook with id-dedup ✅

**Goal**: New React hook that subscribes to the existing
`useTauriEvents` envelope stream and yields the **current set of
unresolved** `PermissionRequest` envelopes, keyed by `request_id`. A
matching `PermissionResolved` envelope removes the request from the
set. Per Q10, the event log is the single source of truth.

**Depends on**: P.3.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/usePermissionRequests.test.ts`:
  - `tracks_a_pending_request`: feed a fixture envelope stream
    `[PermissionRequest{id:"r1"}]`; hook returns `[{id:"r1", ...}]`.
  - `removes_request_on_resolved`: stream
    `[PermissionRequest{id:"r1"}, PermissionResolved{id:"r1"}]`;
    hook returns `[]`.
  - `dedups_duplicate_request_envelopes`: stream
    `[PermissionRequest{id:"r1"}, PermissionRequest{id:"r1"}]`
    (e.g., HMR reattach + history fetch overlap); hook returns one
    entry.
  - `preserves_order_by_arrival`: two pending requests with `t1 <
    t2`; hook returns them in arrival order.
  - `clears_on_run_change`: when the upstream `useTauriEvents`
    re-keys on a new `runId`, the hook also clears (matches the
    existing behaviour — no special code, but assert it).

**Implement** (GREEN):
- `apps/web-ui/src/hooks/usePermissionRequests.ts`. Internally calls
  `useTauriEvents()` and reduces the envelope list with a small
  reducer keyed by `request_id`.
- Export type `PermissionRequest` shape from
  `apps/web-ui/src/types/permission.ts` (or extend the existing
  `pipeline.ts` interface — but pipeline.ts already has a
  `PermissionRequest` type at line 36; make sure the wire-shape from
  the backend matches and rename if the field set differs).

**Refactor**: If `pipeline.ts`'s `PermissionRequest` shape diverges
from the backend's, decide here (RED forces the field-naming choice):
keep the existing `t: number` (a UI-side hint) and add `requestId:
string` as the dedup key. Document the difference in
`apps/web-ui/src/types/permission.ts`.

**Commit**: `feat(web): add usePermissionRequests hook`

**Verification**: `pnpm -F @agentic/web-ui test usePermissionRequests`

---

### Step P.4.2: `ActivityColumn` consumes live `usePermissionRequests` ✅

**Goal**: Replace the hard-coded fixture in `ActivityColumn` (W.7.2)
with the live hook. When a real `PermissionCard` is rendered and the
user clicks a button, fire `invoke('permission_decide', { ... })`.

**Depends on**: P.4.1, W.7.2.

**Test first** (RED):
- Extend `apps/web-ui/src/__tests__/ActivityColumn.test.tsx`:
  - With the new hook mocked to return `[{ requestId: "r1", ... }]`,
    a `PermissionCard` renders.
  - Clicking "Allow once" fires `invoke('permission_decide', {
    requestId: "r1", decision: "once", runId: ..., stepId: ... })`.
    Use the existing mocked `invoke` from
    `apps/web-ui/src/__tests__/devInvokeMock.test.ts` setup.
  - Clicking "Deny" fires the same with `decision: "deny"`.
  - The card stays visible (until the backend echoes a Resolved
    envelope) — assert it does NOT immediately disappear from the
    DOM after click.
  - When the test rig then injects a `PermissionResolved` envelope,
    the card unmounts.

**Implement** (GREEN):
- Edit `apps/web-ui/src/components/ActivityColumn.tsx` to call
  `usePermissionRequests()` for the active run. Pass the array down
  to the existing `PermissionCard` rendering path.
- The `onDecision` callback wires through to
  `invoke('permission_decide', ...)`. Surface backend errors via
  `setHistoryError` (or a new `permissionError` slot on the parent
  `App.tsx` — pick during impl; tech-debt the alternative).

**Refactor**: If the prop signature on `ActivityColumn` grows past 5
props, group permission props into a `permissions: { requests, onDecide
}` object.

**Commit**: `feat(web): wire ActivityColumn permissions to live backend`

**Verification**: `pnpm -F @agentic/web-ui test ActivityColumn`

---

### Step P.4.3: `App.tsx` wires `runId` + `stepId` into the permission call

**Goal**: The existing `App.tsx` already tracks `activeRunId`; thread
it (and the most recent `stepId` from the event stream) into
`ActivityColumn` so `permission_decide` calls carry the correct
`run_id` / `step_id` pair. The backend gate uses these to look up the
right oneshot.

**Depends on**: P.4.2.

**Test first** (RED):
- Extend `apps/web-ui/src/__tests__/app.test.tsx`:
  - Render `<App />` with a fixture event stream containing a
    `StepStarted { step_id: "s1" }` followed by a
    `PermissionRequest`. Click "Allow once". Assert
    `invoke('permission_decide', ...)` was called with `runId =
    <activeRunId>, stepId: "s1"`.

**Implement** (GREEN):
- In `App.tsx`, derive `latestStepId` from the events array. Pass
  `runId={activeRunId}` and `stepId={latestStepId}` into
  `ActivityColumn`. `ActivityColumn` passes them through to the
  decide callback.

**Refactor**: None.

**Commit**: `feat(web): thread runId/stepId into permission_decide calls`

**Verification**: `pnpm -F @agentic/web-ui test app`

---

### Step P.5.1: TUI envelope-routing for Permission* (deferred runner integration)

**Goal**: Per Q9.c — wire `Event::PermissionRequest` into
`AppState::pending_perms` and `Event::PermissionResolved` to remove
the matching entry, but **do not** change T.13.2's local `y/s/n` keys
yet. When the TUI eventually gets a runtime, `pending_perms` will be
populated by the bus rather than fixtures, and the deferred runner
integration (filed as a separate tech-debt issue) will close the loop
by having `y/s/n` publish `PermissionResolved` back through the bus.

**Depends on**: P.1.1, T.13.x (already landed; verify
`AppState::apply_envelope` exists at
`crates/agentic-tui/src/app.rs:248`).

**Test first** (RED):
- Extend `crates/agentic-tui/tests/perm_card.rs` (or add
  `perm_envelope_apply.rs`):
  - `apply_permission_request_envelope_appends_to_pending_perms`:
    construct an envelope with the new variant; call
    `state.apply_envelope(&env)`; assert
    `state.pending_perms.len() == 1` and the fields map correctly
    (`agent`, `command` (= arg), `reason`, `scope`, `risk`).
  - `apply_permission_resolved_removes_matching_request`: with one
    pending request whose `request_id == "r1"` (note: the existing
    `PermissionRequest` struct doesn't carry `request_id` — see
    Refactor below), apply a
    `PermissionResolved { request_id: "r1", .. }`; assert
    `state.pending_perms.is_empty()`.
  - `unmatched_resolved_is_noop`: pending request `r1`, apply
    Resolved for `r2`; pending stays.
- Confirm existing `crates/agentic-tui/tests/perm_keys.rs` tests (the
  `y/s/n` keys) still pass — they test local-state mutations which
  don't change.

**Implement** (GREEN):
- Add `request_id: String` field to
  `crates/agentic-tui/src/app.rs::PermissionRequest` (compatibility:
  this is a TUI-internal struct, not the wire envelope; the
  `PermissionRequest` envelope variant from P.1.1 is the source of
  truth, and the TUI struct mirrors it). Update
  `crates/agentic-tui/src/views/perm_card.rs` if it has fixture data
  (currently has `command`, `agent`, `reason` — leave the renderer
  alone; just thread `request_id` through `Default::default`).
- Extend
  `apply_envelope` in `app.rs` with two new arms:
  ```rust
  Event::PermissionRequest { request_id, agent, tool, arg, scope, risk, reason } => {
      self.pending_perms.push(PermissionRequest { request_id, agent, command: arg, reason, scope, risk: map_risk(risk) });
  }
  Event::PermissionResolved { request_id, .. } => {
      self.pending_perms.retain(|p| p.request_id != request_id);
  }
  ```
  Where `map_risk` translates the wire-format
  `events::PermissionRisk` to the TUI-local `app::PermissionRisk`
  (both should be structurally identical — consider exposing the
  events one and dropping the local one in a later refactor; tech-debt
  if you don't).

**Refactor**: Tech-debt note in
`crates/agentic-tui/TODO.md` (or local todo): "TUI runner integration
for permissions: `y/s/n` keys still mutate local state only;
publishing back via the bus requires the same runtime-channel
plumbing as T.13.x runner integration. Trigger: when the TUI gains
its `agentic-core` runtime handle." File as a separate GH issue with
`tech-debt` label per CLAUDE.md §4.

**Commit**: `feat(tui): apply Permission envelopes to AppState`

**Verification**: `cargo test -p agentic-tui perm_envelope`

---

### Step P.6.1: End-to-end pipeline test (web) — Vitest ✓ / live-Rust: P.6.1.b

**Goal**: One Vitest integration test that drives the complete
loop: backend emits `PermissionRequest` → web UI renders card →
user clicks Allow once → `permission_decide` invoke → backend echoes
`PermissionResolved` → card unmounts. Mock the Tauri layer at
`invoke` and `listen` boundaries; do not spawn a real backend.

**Depends on**: P.4.3.

**Test first** (RED):
- New test
  `apps/web-ui/src/__tests__/permissionFlow.integration.test.tsx`:
  1. Mount `<App />` with a mocked event channel.
  2. Push a `PermissionRequest{ requestId: "r1" }` onto the channel.
  3. Wait for the `PermissionCard` to render.
  4. `userEvent.click(getByText("Allow once"))`.
  5. Assert the mocked `invoke` was called with
     `("permission_decide", { requestId: "r1", decision: "once", ... })`.
  6. Push a `PermissionResolved{ requestId: "r1" }` onto the
     channel.
  7. Wait for the card to unmount; assert no
     `data-testid="permission-card"` in the DOM.
- Also push a `PermissionResolved` for an unknown request and assert
  the UI does not crash.

**Implement** (GREEN):
- Should pass once P.4.3 is complete; this step exists to lock the
  integration in a single regression test rather than three
  independent unit tests. If the test fails, fix the integration —
  do not modify individual units' tests.

**Refactor**: If the test setup (mocked `invoke`/`listen`) is large,
extract a helper into `apps/web-ui/src/__tests__/setup.ts`.

**Commit**: `test(web): add permission-flow E2E integration test`

**Verification**: `pnpm -F @agentic/web-ui test permissionFlow`

**Status**: Vitest portion complete (3 tests added, all pass on first run — P.4.3
confirmed correct). Live-Rust complement is P.6.1.b below.

---

### Step P.6.1.b: Live-Rust complement test (P.6.1 follow-up)

**Goal**: A `#[ignore]`d Rust integration test in `agentic-cli` that exercises the
full backend stack with a real `ClaudeCodeBackend` subprocess against a sandbox
Python project (palindrome function), with `AsyncGate` intercepting tool calls and
emitting `PermissionResolved` envelopes onto the bus.

**Depends on**: P.6.1 (Vitest portion complete).

**Deferred from**: P.6.1 dispatch — live backend infrastructure not needed for
regression lock; the Vitest mock layer is sufficient for UI integration coverage.

**Test**: `crates/agentic-cli/tests/e2e_permissions_live.rs`
- Single `#[ignore]` test requiring `ANTHROPIC_API_KEY` + `claude` on PATH.
- Sandbox setup: tempdir, two placeholder Python files, git repo, permissions.toml.
- Asserts: ≥1 `ToolUseStart`, ≥1 `PermissionResolved`, ≥1 `AllowlistConfig`
  resolution, no panic, completes within 5 min.
- Run: `cargo test -p agentic-cli --test e2e_permissions_live -- --ignored --nocapture`
- Documented in `docs/SMOKE.md` § "Live permission gate smoke test".

**Status**: Complete. Test compiles and is properly `#[ignore]`d (0 tests run in
default `cargo test`). Clippy + fmt clean. Live run not verified (ANTHROPIC_API_KEY
absent in the agent shell session); the test itself guards with an early-return on
missing key. `claude` binary confirmed on PATH at `/Users/igorilic/.local/bin/claude`.

---

### Step P.6.2: File follow-up tech-debt issues + close GH #88

**Goal**: Per CLAUDE.md §4, file the deferred items as GH issues
with the `tech-debt` label, link them back into todo.md, and close
GH #88 with a comment pointing at this phase + the new follow-up.

**Depends on**: P.6.1.

**Test first** (RED): N/A — administrative step.

**Implement** (GREEN):
1. Create `gh issue create --label tech-debt --title "Real
   blocking permission gate (MCP/proxy intercept)" --body "..."`.
   Body mirrors the Phase P intro: observational gate ships in Phase
   P; full blocking requires the child CLI to call out to an
   MCP-compliant tool server (or a process proxy) that pauses the
   tool call before execution. Trigger: when MCP integration lands or
   when a user reports a destructive call landed before the prompt
   was visible.
2. Create `gh issue create --label tech-debt --title "TUI permission
   y/s/n: publish PermissionResolved through the bus"`. Body: T.13.2
   keys mutate local state only; need a runtime channel to publish
   `Event::PermissionResolved` back to the bus when the TUI gains
   its runtime handle. Trigger: T.13.x runner integration milestone.
3. Update tech-debt entry 2 above: change "(GH #88 — in-progress,
   see Phase P)" to "(GH #88 — closed by Phase P; follow-ups: GH
   #<blocking-gate>, GH #<tui-perm-publish>)".
4. Close GH #88 with a comment linking to the merged Phase P PR and
   the two new issues.

**Refactor**: None.

**Commit**: `docs(redesign): close GH #88 + file permission follow-ups`

**Verification**:
- `gh issue view 88 --json state -q '.state' | grep -i closed`
- `gh issue list --label tech-debt | rg -i "blocking permission|TUI permission"`
- two issues visible.

---



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
- [x] W.9.3 ChatComposer layout polish (chips below, paper-plane, placeholder)
- [ ] W.9.4 ChatComposer New-spec affordance
- [x] W.9.5 HeaderBar settings gear icon
- [x] W.9.6 StatusDot component + use in AgentCard
- [x] W.9.7 IssueColumn header polish (run-state pill + section labels)
- [ ] W.9.8 App.tsx polish integration test
- [ ] CP-W2

Phase 10 — TUI palette + title bar
- [ ] T.10.1 Theme module
- [ ] T.10.2 Title bar

Phase 11 — TUI pipeline + tabs
- [ ] T.11.1 Issue header
- [x] T.11.2 Pipeline bar boxes
- [x] T.11.3 Pipeline hint
- [x] T.11.4 Tab bar
- [x] T.11.5 1/2/3 keys

Phase 12 — TUI body panes
- [x] T.12.1 Logs pane
- [x] T.12.2 Chat pane
- [ ] T.12.3 Issue pane

Phase 13 — TUI permission + status line
- [x] T.13.1 Permission card
- [x] T.13.2 y/s/n keys
- [ ] T.13.3 Status line + modes
- [ ] T.13.4 Flash lifecycle
- [x] T.13.5 Help overlay
- [x] T.13.6 Insert mode
- [x] T.13.7 draw_app composition
- [x] T.13.8 Delete dead views
- [ ] CP-T

Phase 14 — Tauri
- [ ] X.14.1 Wire dense
- [ ] X.14.2 Lock window decorations

Phase 15 — Cleanup + tech debt
- [ ] X.15.1 Full test pass + snapshots
- [ ] X.15.2 File tech-debt issues

Phase P — Permissions (GH #88)
- [x] P.1.1 Add PermissionRequest + PermissionResolved event variants
- [x] P.1.2 permissions.toml config loader
- [x] P.1.3 Tool matcher (`<tool>(<arg-glob>)` and `<tool>:*`)
- [x] P.1.4 Risk classifier table
- [x] P.2.1 PermissionGate trait + ConfigGate static
- [x] P.2.2 Decision channel + async evaluate_async (60 s timeout)
- [x] P.2.3 Per-run session allowlist
- [x] P.2.4 Wire gate into PipelineOrchestrator
- [x] P.3.1 Tauri permission_decide command
- [x] P.3.2 Forwarder regression-test for Permission* envelopes
- [x] P.4.1 usePermissionRequests hook with id-dedup
- [x] P.4.2 ActivityColumn consumes live usePermissionRequests
- [x] P.4.3 App.tsx wires runId/stepId into permission_decide
- [x] P.5.1 TUI applies Permission envelopes to AppState (deferred runner integration)
- [x] P.6.1 End-to-end web permission-flow integration test (Vitest portion)
- [x] P.6.1.b Live-Rust complement test (palindrome sandbox + real AsyncGate)
- [x] P.6.2 File follow-up tech-debt issues + close GH #88

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

---

## Open implementation questions (Phase F)

Surfaced during planning; flag to user before tdd-developer dispatches:

1. **HeaderBar grid layout**: the right cluster currently uses
   `flex items-center gap-3.5`. The segmented control adds a 4th sibling
   (selector → run-state → settings → theme → avatar). At narrow widths
   the cluster may overflow. Acceptable tradeoff for F.1.3? (Suggested:
   yes — `flex-wrap: nowrap` + `min-w-0` on left cluster keeps brand
   truncating first.) **No code change needed unless reviewer flags.**
2. **`JiraTicketSource::fetch` shape**: returns `Ticket { title, body,
   comments, ac_field: Option<String>, url }`. Comments and `url` are
   dropped at the IPC layer (DTO is `{ key, title, body, ac }`). User
   confirmed scope; documenting here so reviewer doesn't flag as an
   omission.
3. **Env-var freshness**: F.2.1 reads `JIRA_URL` / `JIRA_EMAIL` /
   `JIRA_API_TOKEN` / `JIRA_AC_FIELD_ID` **per IPC call** (not at app
   startup). Rationale: user can `export JIRA_URL=…` in shell and re-
   click "Pull" without restarting Tauri. Negligible cost; documented
   in F.2.1 step body.
4. **`useBackend()` localStorage key**: `agentic.backend` (mirrors
   `agentic.theme`). Default value: `"claude-code"` (matches every
   existing hardcoded site).
5. **Segmented control a11y**: use `role="group"` wrapping two
   `<button aria-pressed={selected}>` elements rather than radio
   inputs — matches existing theme-toggle pattern in HeaderBar
   (`aria-pressed`).

---

## Phase F.1 — Backend selector

User-visible selector for the runtime backend (`claude-code` or
`copilot-cli`). Replaces three hardcoded `"claude-code"` call sites
with a hook-driven value. Surfaces "binary not found" pre-flight
errors as system messages in the chat column.

Implementation contract:
- `useBackend()` hook with same shape as `useTheme()` (state + setter
  + localStorage persistence under key `agentic.backend`).
- Segmented 2-button toggle in HeaderBar's right cluster, **left of
  the run-state pill**. 1-click swap; no popover.
- Default: `"claude-code"`. Persisted across reloads.
- All three call sites (`App.tsx:101`, `ChatPane.tsx:41`,
  `createSpec.ts:16`) read from the hook.
- "Binary not found" pre-flight failure is surfaced by appending the
  error message to `systemMessages` in `ChatPane`, matching the
  existing slash-error pattern.

### [x] Step F.1.1: `useBackend()` hook + localStorage persistence

**Goal**: Add a hook that owns the selected backend ID, persists it to
`localStorage` under `agentic.backend`, and exposes
`{ backend, setBackend }`. Mirrors `useTheme.ts` for consistency.

**Depends on**: none.

**Test first** (RED):
- New file `apps/web-ui/src/__tests__/useBackend.test.ts`:
  - `it("defaults to 'claude-code' when no value persisted")`: render
    via `renderHook`, assert `result.current.backend === "claude-code"`.
  - `it("reads persisted value from localStorage on mount")`: pre-seed
    `localStorage.setItem("agentic.backend", "copilot-cli")`, render,
    assert `result.current.backend === "copilot-cli"`.
  - `it("ignores invalid persisted values")`: pre-seed
    `localStorage.setItem("agentic.backend", "garbage")`, render,
    assert default `"claude-code"`.
  - `it("setBackend updates state and persists")`: render, call
    `act(() => result.current.setBackend("copilot-cli"))`, assert
    `result.current.backend === "copilot-cli"` and
    `localStorage.getItem("agentic.backend") === "copilot-cli"`.
  - Use `beforeEach(() => localStorage.clear())`.

**Implement** (GREEN):
- New file `apps/web-ui/src/hooks/useBackend.ts`:
  - `export type BackendKind = "claude-code" | "copilot-cli";`
    (re-export from `slash/types.ts` to avoid duplication; or import
    and re-export). Inspect import paths first — keep one canonical
    definition.
  - Hook signature:
    `export function useBackend(): { backend: BackendKind; setBackend: (b: BackendKind) => void }`.
  - Internal state initializer reads `localStorage.getItem("agentic.backend")`,
    validates against the union, defaults to `"claude-code"`.
  - `useEffect` writes to `localStorage` on `backend` change (mirrors
    `useTheme` pattern).

**Refactor**: None.

**Commit**: `feat(web): add useBackend() hook for backend selection state`

**Verification**: `pnpm -F @agentic/web-ui test useBackend`

**Notes**: Do **not** wire the hook into any component yet — F.1.2
handles call-site replacement, F.1.3 handles UI. This step ships only
the hook + tests.

---

### [x] Step F.1.2: Replace 3 hardcoded `"claude-code"` call sites

**Goal**: Replace the literal `"claude-code"` strings in `App.tsx:101`,
`ChatPane.tsx:41`, and `createSpec.ts:16` with values read from
`useBackend()`. The IPC payload to `start_ticket_run` must reflect the
hook's current state.

**Depends on**: F.1.1.

**Test first** (RED):
- Extend `apps/web-ui/src/__tests__/ChatPane.test.tsx` (or add a
  dedicated test there):
  - `it("uses backend from useBackend() in /plan dispatch")`: pre-seed
    `localStorage.setItem("agentic.backend", "copilot-cli")` in
    `beforeEach`. Render `ChatPane`, type `/plan #42 ticket text`, send.
    Assert the `invoke` mock was called with `start_ticket_run` and
    `{ ticket: "#42 ticket text", backend: "copilot-cli", model: null }`
    (use existing invoke mock pattern in this test file).
- New test `apps/web-ui/src/__tests__/createSpec.test.ts` (extend
  existing) or add a new case:
  - `it("threads the active backend into the IPC payload")`: pre-seed
    `localStorage`, call `createSpec("title")`, assert mock was called
    with `backend: "copilot-cli"`. **Caveat**: `createSpec` is a plain
    function — it cannot read a React hook directly. Solution: change
    its signature to accept `backend: BackendKind` as a parameter
    (callers pass `useBackend()`'s value). Update existing call sites
    in `IssueColumn.tsx` and `ChatColumn.tsx` accordingly. Update the
    existing `createSpec.test.ts` cases to pass `backend` explicitly.
- Extend `apps/web-ui/src/__tests__/app.test.tsx` to cover
  `App.tsx:101` (the `handleRunPipeline` "Untitled run" path):
  - `it("Run-pipeline button uses selected backend")`: pre-seed
    localStorage to `copilot-cli`, render `<App />`, click
    `header-run`, assert `invoke('start_ticket_run', …)` was called
    with `backend: "copilot-cli"`.

**Implement** (GREEN):
- `apps/web-ui/src/utils/createSpec.ts`: change signature to
  `createSpec(title: string, backend: BackendKind): Promise<string | undefined>`.
  Pass `backend` through to `invoke`.
- Update both `createSpec` call sites (search via
  `rg "createSpec\\(" apps/web-ui/src`):
  - `IssueColumn.tsx`, `ChatColumn.tsx` — both should call
    `useBackend()` at the component top and pass `backend` into
    `createSpec(title, backend)`.
- `apps/web-ui/src/components/ChatPane.tsx`: import `useBackend`, call
  it at the top of the component, replace the inline
  `backend ?? "claude-code"` fallback in the `slashServices.plan`
  closure with `backend ?? selectedBackend` where `selectedBackend`
  comes from the hook. (`backend` here is the explicit
  `--backend=…` flag from a `/plan` command.)
- `apps/web-ui/src/App.tsx:101`: import `useBackend`, call it, pass
  hook value into the `start_ticket_run` invoke for the
  `handleRunPipeline` path.
- Confirm no other files contain the literal `"claude-code"` outside
  of test fixtures and `slash/types.ts` (the canonical union literal)
  with `rg '"claude-code"' apps/web-ui/src`.

**Refactor**: None — all three sites should now read from a single
source of truth.

**Commit**: `refactor(web): thread useBackend() through start_ticket_run call sites`

**Verification**:
- `pnpm -F @agentic/web-ui test ChatPane createSpec app`
- `pnpm -F @agentic/web-ui typecheck`

**Notes**:
- Do **not** alter `slash/types.ts`'s `BACKENDS` union — it's the
  canonical type definition and must stay literal.
- Existing fixture-only `"claude-code"` strings in test files
  (`devInvokeMock.test.ts`, `slashParser.test.ts`, etc.) stay as-is —
  they assert exact IPC payloads.

---

### [x] Step F.1.3: HeaderBar segmented control wired to the hook

**Goal**: Add a 2-button segmented control (`Claude Code` |
`Copilot CLI`) to HeaderBar's right cluster, **left of the run-state
pill**. Clicking a segment immediately swaps the active backend via
`useBackend().setBackend`. The active segment shows pressed state via
`aria-pressed="true"` and a darker background, mirroring the
theme-toggle button's a11y pattern.

**Depends on**: F.1.1, F.1.2.

**Test first** (RED):
- New test `apps/web-ui/src/__tests__/HeaderBar.test.tsx` (or extend
  if exists):
  - `it("renders the backend segmented control with two buttons")`:
    render `<HeaderBar …minimal props… />` with localStorage default,
    assert `getByTestId("header-backend-claude-code")` and
    `getByTestId("header-backend-copilot-cli")` exist.
  - `it("marks claude-code as pressed by default")`: assert
    `header-backend-claude-code` has `aria-pressed="true"` and
    `header-backend-copilot-cli` has `aria-pressed="false"`.
  - `it("clicking copilot-cli flips the active backend")`: click,
    assert `header-backend-copilot-cli` becomes `aria-pressed="true"`,
    `header-backend-claude-code` becomes `aria-pressed="false"`, and
    `localStorage.getItem("agentic.backend") === "copilot-cli"`.
  - `it("clicking claude-code flips back")`: pre-seed copilot-cli,
    click claude-code, assert pressed flips back.
  - DOM-order assertion: assert the segmented control's container
    appears **before** `header-run-state` in the rendered DOM (use
    `compareDocumentPosition` or `Array.from(container.children)`
    indexing).

**Implement** (GREEN):
- `apps/web-ui/src/components/HeaderBar.tsx`: import `useBackend`,
  call it inside the component. Add a new wrapping `<div role="group"
  aria-label="Backend">` containing two `<button type="button">`
  elements with `data-testid="header-backend-claude-code"` and
  `header-backend-copilot-cli`. Each button:
  - `aria-pressed={backend === "claude-code"}` (resp. copilot).
  - `onClick={() => setBackend("claude-code")}` (resp. copilot).
  - Visual: pressed state uses `bg-bg-surface-2 text-fg`; unpressed
    uses `text-fg-muted hover:text-fg`. Container uses
    `rounded-md border border-border-soft p-0.5 flex` to give the
    segmented look.
  - Label: `Claude` and `Copilot` (compact — full names crowd the
    header; testids carry the canonical IDs).
- Insert the new `<div>` as the **first child** of the right-cluster
  flex container (before `<div role="status" data-testid="header-run-state">`).

**Refactor**: If button styling repeats, extract a small
`<SegmentedButton pressed onClick label testid />` local component
inside the same file (do not export). Skip if it doesn't materially
clean the JSX.

**Commit**: `feat(web): add backend segmented control to HeaderBar`

**Verification**:
- `pnpm -F @agentic/web-ui test HeaderBar`
- Manual smoke (deferred to F.1.4 reviewer triage if required):
  open the app, click each segment, observe the pill toggles.

**Notes**:
- No new dependencies. Pure Tailwind + existing tokens.
- Segmented control uses `aria-pressed` over `role="radio"` to match
  the theme-toggle pattern; lighter a11y surface.
- Snapshot test of HeaderBar exists somewhere (P.4.x era?) — re-run
  full HeaderBar suite to catch any DOM-shape change. Update snapshot
  if reviewer green-lights.

---

### [x] Step F.1.4: Surface "binary not found" pre-flight error in chat

**Goal**: When `start_ticket_run` rejects with the pre-flight error
message (e.g. `"pre-flight: \`copilot\` not found on PATH. Install …"`),
append that message to `systemMessages` in `ChatPane` so the user sees
*why* the run didn't start. Matches the existing slash-error /
mention-error pattern.

**Depends on**: F.1.2 (call sites must already be threading the hook
value, otherwise the error path isn't exercised distinctly).

**Test first** (RED):
- Extend `apps/web-ui/src/__tests__/ChatPane.test.tsx`:
  - `it("surfaces start_ticket_run pre-flight errors as system messages")`:
    mock `invoke('start_ticket_run', …)` to reject with `"pre-flight:
    \`copilot\` not found on PATH. Install …"`. Render `ChatPane`,
    type `/plan #42 do thing`, send. Assert the system-messages list
    rendered in `ChatColumn` contains a string starting with
    `"pre-flight:"` (or `"Command failed: pre-flight:"` depending on
    where in `dispatchSlashCommand` the error percolates — check
    actual flow first).
  - `it("does NOT swallow non-pre-flight errors")`: same setup but
    reject with `"Some other error"` — assert system-messages
    contains `"Some other error"` (regression guard for the existing
    error path).
- Audit `apps/web-ui/src/__tests__/app.test.tsx` for the
  `handleRunPipeline` path: it currently silently swallows errors via
  `.catch(() => {})`. Decision: do we surface there too? **Yes** —
  add a `it("Run-pipeline button surfaces pre-flight errors")` test
  that asserts a system message bubbles through `ChatPane`'s prop
  channel. May require routing the error from `App.handleRunPipeline`
  into a callback that ChatPane consumes (similar to
  `onTicketRunStarted`). If the wiring inflates scope > 30 min,
  defer the App-level handler to a tech-debt note and ship only the
  `/plan`-path coverage.

**Implement** (GREEN):
- `apps/web-ui/src/components/ChatPane.tsx`: the slash-dispatcher
  catch block already runs (`Command failed: …`). Confirm the
  pre-flight error string surfaces verbatim (the `${err}` template
  serializes the rejection reason). If the message gets swallowed by
  a generic "Command failed" prefix, refine to:
  ```
  setSystemMessages((prev) => [
    ...prev,
    String(err).startsWith("pre-flight:") ? String(err) : `Command failed: ${err}`,
  ]);
  ```
  This preserves the actionable hint without losing the prefix for
  non-pre-flight errors.
- `App.tsx:handleRunPipeline`: replace the silent `.catch(() => {})`
  with a callback that lifts the error string up to `ChatPane`'s
  `systemMessages`. Add a new prop on `ChatPane`:
  `onSystemMessage?: (msg: string) => void` — but this requires
  inverted flow. Simpler: hoist `systemMessages` state into `App` and
  pass it as a prop. **Defer this restructuring to tech-debt** if
  it costs > 30 min; instead, in F.1.4 ship the `/plan`-path
  coverage only and log a tech-debt entry for the
  `handleRunPipeline` path with the trigger "when the Run-pipeline
  button graduates from placeholder to a SpecDialog-driven flow
  (W.8.x successor)".

**Refactor**: None.

**Commit**: `feat(web): surface pre-flight errors in chat system-messages`

**Verification**: `pnpm -F @agentic/web-ui test ChatPane app`

**Notes**:
- This step may merge into F.1.2 if the existing
  `dispatchSlashCommand` already preserves the error string verbatim
  (run the new test against current `main` before committing F.1.4 —
  if it passes red→green with no change, fold into F.1.2's
  Refactor and skip the standalone commit). The judgment call is
  the tdd-developer's; document either way in the F.1.4 status
  checklist.
- Tech-debt note (if filed): "App.handleRunPipeline silently swallows
  pre-flight errors. Lift systemMessages state from ChatPane to App
  to share the surface, OR fold the Run-pipeline button into
  SpecDialog (W.8.x). Trigger: SpecDialog-driven Run flow lands."

---

### Phase F.1 status checklist

- [ ] F.1.1 useBackend() hook
- [ ] F.1.2 Replace 3 hardcoded call sites
- [ ] F.1.3 HeaderBar segmented control
- [x] F.1.4 Pre-flight error → systemMessages

---

## Phase F.2 — Jira ticket pull

Adds a "Pull from Jira" affordance to `SpecDialog` that fetches a
Jira ticket by key and pre-fills the title and body fields. Uses the
existing `JiraTicketSource` in `agentic-core`; new IPC layer wraps it
with env-var resolution and DTO mapping.

Implementation contract:
- New Tauri IPC `fetch_jira_ticket(key: String) -> Result<JiraTicketDto, String>`.
- Env vars (read **per-call**): `JIRA_URL` (bare host, e.g.
  `https://yourorg.atlassian.net`), `JIRA_EMAIL`, `JIRA_API_TOKEN`,
  optional `JIRA_AC_FIELD_ID`.
- IPC layer appends `/rest/api/3` to the `JIRA_URL` host before
  passing it to `JiraTicketSource::new`.
- DTO: `{ key: String, title: String, body: String, ac: Option<String> }`.
  Comments and `url` from the inner `Ticket` are dropped.
- Client-side regex `^[A-Z][A-Z0-9]+-\d+$` gates the button-enabled
  state (instant feedback). Server-side `parse_ref` provides
  defense-in-depth.
- Button **always visible** in SpecDialog. Disabled with tooltip
  listing missing env-var names when env is incomplete.
- On success: title field populated with ticket title; body field
  populated with `body + "\n\n## Acceptance Criteria\n" + ac` when
  `ac.is_some()`, else just `body`. **No new SpecDialog field** —
  AC is appended to the existing body textarea.
- No cache; button disables while fetch is in flight.

**Note on `body` IPC drop (GH #92)**: The body parameter is currently
captured by SpecDialog but dropped at the `start_ticket_run` IPC
boundary (see `createSpec.ts` comment). That's a separate
pre-existing bug tracked in GH #92 and NOT a Phase F.2 deliverable.
F.2's success criterion is "title + body fields are populated"; what
happens at the next IPC hop is GH #92's concern.

### [x] Step F.2.1: `fetch_jira_ticket` Tauri IPC + DTO + per-call env resolution

**Goal**: Add a new Tauri command that reads the four `JIRA_*` env
vars at call time, validates presence, constructs `JiraTicketSource`
with `${JIRA_URL}/rest/api/3` as the base URL, fetches the ticket,
and returns a flat DTO. Errors are user-facing strings.

**Depends on**: none (in-tree dependency on `agentic-core::ticket_sources::jira`).

**Test first** (RED):
- New file `crates/agentic-tauri/src/commands/jira.rs` (testable inner
  helper, IPC entry calls helper):
  - `fn missing_env_vars() -> Vec<&'static str>`: returns names of any
    of `["JIRA_URL", "JIRA_EMAIL", "JIRA_API_TOKEN"]` not set in
    process env. Exclude `JIRA_AC_FIELD_ID` (optional).
  - `fn build_base_url(jira_url: &str) -> String`: trims any trailing
    slash, appends `/rest/api/3`. Idempotent — if input already ends
    in `/rest/api/3` (or 2), don't double-append; trim and re-append
    to canonicalize on v3.
  - `fn validate_key(key: &str) -> Result<(), String>`: regex
    `^[A-Z][A-Z0-9]+-\d+$`. Error: `"invalid ticket key: \"{key}\""`.
- New test file
  `crates/agentic-tauri/src/commands/jira_tests.rs` (or `#[cfg(test)] mod tests` in `jira.rs`):
  - `missing_env_vars_returns_unset_names`: scrub the 3 vars via
    `temp_env::with_vars` (or manual save/restore), assert returns
    all 3.
  - `missing_env_vars_returns_empty_when_all_set`: set all 3, assert
    empty vec.
  - `build_base_url_appends_v3`:
    `build_base_url("https://acme.atlassian.net") == "https://acme.atlassian.net/rest/api/3"`.
  - `build_base_url_strips_trailing_slash`:
    `build_base_url("https://acme.atlassian.net/") == "https://acme.atlassian.net/rest/api/3"`.
  - `build_base_url_canonicalizes_v2_to_v3`: input ending in
    `/rest/api/2` returns `/rest/api/3`. (Document this behavior
    explicitly — `JiraTicketSource` only speaks v3.)
  - `build_base_url_idempotent_on_v3`: input ending in `/rest/api/3`
    returns same.
  - `validate_key_accepts_valid`: `PROJ-1`, `ABC-999`, `XY1-42`.
  - `validate_key_rejects_invalid`: `""`, `"proj-1"`, `"PROJ-"`,
    `"-1"`, `"PROJ-abc"`, `"PROJ"`.
- IPC integration test (using existing scripted-test harness pattern,
  see `permission_decide` tests for shape — `crates/agentic-tauri/tests/`
  if a Tauri integration harness exists). If a real HTTP server stub
  is too heavy for this step, **skip the network-level test** and
  rely on `JiraTicketSource`'s own `crates/agentic-core/tests/ticket_jira.rs`
  for the fetch path. Document the trade-off in the step body. The
  IPC's contract surface that the test must cover is:
  - `fetch_jira_ticket("PROJ-1")` with no env returns
    `Err("missing environment variables: JIRA_URL, JIRA_EMAIL, JIRA_API_TOKEN")`.
  - `fetch_jira_ticket("invalid")` with env set returns
    `Err("invalid ticket key: \"invalid\"")`.

**Implement** (GREEN):
- New file `crates/agentic-tauri/src/commands/jira.rs`:
  - `#[derive(serde::Serialize, Clone, Debug)] pub struct JiraTicketDto { key: String, title: String, body: String, ac: Option<String> }`.
  - Helpers `missing_env_vars`, `build_base_url`, `validate_key` per
    test contracts.
  - `#[tauri::command] pub async fn fetch_jira_ticket(key: String) -> Result<JiraTicketDto, String>`:
    1. Trim `key`. Call `validate_key`.
    2. Call `missing_env_vars`; if non-empty, return
       `Err(format!("missing environment variables: {}", missing.join(", ")))`.
    3. Read `JIRA_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN`,
       `JIRA_AC_FIELD_ID` (last one optional via `ok()`).
    4. Construct
       `JiraTicketSource::new(build_base_url(&jira_url), email, token, ac_field_id)`.
    5. Build `TicketRef { kind: TicketKind::Jira, reference: key.clone(), … }`.
       (Inspect `TicketRef`'s actual shape — see
       `agentic-core::events::TicketRef`. Likely also has `url`/`title`
       fields; populate minimally.)
    6. Call `source.fetch(&ticket_ref).await`. Map error to user
       string (use `format!("{e}")` — `TicketSourceError`'s Display
       impl is already user-readable; verify by reading
       `agentic-core::ticket_sources::TicketSourceError`).
    7. Return `JiraTicketDto { key, title: t.title, body: t.body, ac: t.ac_field }`.
- `crates/agentic-tauri/src/commands/mod.rs`: add `pub mod jira;`.
- `crates/agentic-tauri/src/main.rs` (or wherever `tauri::Builder::default().invoke_handler(…)` lives):
  register `commands::jira::fetch_jira_ticket` in the invoke handler
  list.

**Refactor**: If `validate_key` duplicates `agentic-core::ticket_sources::jira::parse_ref` exactly, prefer:
re-export `parse_ref` (or wrap a public version) from `agentic-core`
and call from the IPC. Reduces drift. If `parse_ref` is private and
extracting feels like scope creep, keep the local copy and file a
tech-debt note ("Unify Jira key validation between `agentic-core` and
`agentic-tauri`. Trigger: when a third caller appears.").

**Commit**: `feat(tauri): add fetch_jira_ticket IPC for Jira ticket pull`

**Verification**:
- `cargo test -p agentic-tauri commands::jira`
- `cargo clippy -p agentic-tauri --all-features --all-targets -- -D warnings`

**Notes**:
- Per-call env resolution chosen so the user can `export JIRA_URL=…`
  in their shell and re-click "Pull" without restarting the Tauri
  binary. Cost: 4 `std::env::var` calls per click — negligible.
- `JIRA_AC_FIELD_ID` (e.g. `customfield_10100`) when absent triggers
  `JiraTicketSource`'s description-parse fallback (already
  implemented). Document in a doc-comment on `fetch_jira_ticket`.
- Do **not** register the command in the capability list yet — that's
  F.2.2. The IPC will be inert until the capability is granted.

---

### [x] Step F.2.2: Tauri permission file + capability registration

**Goal**: Generate the autogenerated permission TOML and add
`allow-fetch-jira-ticket` to `capabilities/default.json`. Without
this, the webview's `invoke('fetch_jira_ticket', …)` call is rejected
by Tauri's permission gate at runtime — even though the command is
registered in the invoke handler. (P.3.1 hit this exact issue live;
documented in CLAUDE.md as a checklist item.)

**Depends on**: F.2.1.

**Test first** (RED):
- Tauri permissions are file-based and don't have a unit-test surface
  per se. The "test" is a deterministic file presence + capability
  registration check:
- New test or reused test file `crates/agentic-tauri/tests/capabilities.rs`
  (extend if exists; mirror P.3.1's coverage if it added one):
  - `it_includes_fetch_jira_ticket_capability`: parse
    `capabilities/default.json`, assert
    `permissions` array contains `"allow-fetch-jira-ticket"`.
  - `it_has_autogenerated_jira_permission_file`: assert
    `permissions/autogenerated/fetch_jira_ticket.toml` exists and
    parses as TOML containing
    `[[permission]] identifier = "allow-fetch-jira-ticket"`.
- If no such test file exists, create it. Use `std::fs::read_to_string`
  + `serde_json` / `toml::from_str`.

**Implement** (GREEN):
- New file
  `crates/agentic-tauri/permissions/autogenerated/fetch_jira_ticket.toml`,
  modeled on `start_ticket_run.toml`:
  ```toml
  # Automatically generated - DO NOT EDIT!

  [[permission]]
  identifier = "allow-fetch-jira-ticket"
  description = "Enables the fetch_jira_ticket command without any pre-configured scope."
  commands.allow = ["fetch_jira_ticket"]

  [[permission]]
  identifier = "deny-fetch-jira-ticket"
  description = "Denies the fetch_jira_ticket command without any pre-configured scope."
  commands.deny = ["fetch_jira_ticket"]
  ```
- Edit `crates/agentic-tauri/capabilities/default.json`: append
  `"allow-fetch-jira-ticket"` to the `permissions` array. Keep
  trailing comma rules consistent with the existing JSON.

**Refactor**: None.

**Commit**: `feat(tauri): grant fetch_jira_ticket capability in default window`

**Verification**:
- `cargo test -p agentic-tauri capabilities`
- `cargo build -p agentic-tauri` (catches Tauri permission-schema
  validation at compile time via `tauri-build`).

**Notes**:
- The autogenerated file's `# Automatically generated - DO NOT EDIT!`
  banner is preserved by Tauri's permission codegen; we hand-write it
  to seed the file but Tauri may regenerate it during
  `cargo build`. That's expected and fine — the content is
  deterministic.

---

### [x] Step F.2.3: TS DTO type + `useJiraFetch()` IPC client hook

**Goal**: Add a TypeScript type that mirrors `JiraTicketDto` and a
React hook `useJiraFetch()` that wraps `invoke('fetch_jira_ticket')`
with `{ fetch, isLoading, error, isAvailable }` ergonomics. The
`isAvailable` flag is a static `true` for now (env-var sniffing
happens server-side); SpecDialog will compute its own
`disabledReason` from a separate IPC introspection in F.2.4 — see
that step's "Open question" callout.

**Depends on**: F.2.2 (capability must exist before the hook can
successfully invoke).

**Test first** (RED):
- New file `apps/web-ui/src/__tests__/useJiraFetch.test.ts`:
  - `it("returns fetch, isLoading, error in initial state")`: render,
    assert `result.current.isLoading === false`, `result.current.error === null`,
    `typeof result.current.fetch === "function"`.
  - `it("invokes fetch_jira_ticket with the key and returns the DTO")`:
    mock `invoke('fetch_jira_ticket', { key: 'PROJ-1' })` to resolve
    with `{ key: "PROJ-1", title: "T", body: "B", ac: null }`. Call
    `await result.current.fetch("PROJ-1")`, assert returned value
    matches.
  - `it("sets isLoading=true while in flight")`: use a deferred
    promise to keep `invoke` pending; assert `isLoading === true`
    after `act` advances. Resolve the promise, assert `isLoading === false`.
  - `it("captures error message on rejection")`: mock invoke to
    reject with `"missing environment variables: JIRA_URL"`, call
    fetch, assert `result.current.error === "missing environment variables: JIRA_URL"`.
  - `it("clears error on subsequent successful fetch")`: reject
    once, then resolve; assert error transitions null → string → null.

**Implement** (GREEN):
- New file `apps/web-ui/src/types/jira.ts`:
  ```ts
  export type JiraTicketDto = {
    key: string;
    title: string;
    body: string;
    ac: string | null;
  };
  ```
- New file `apps/web-ui/src/hooks/useJiraFetch.ts`:
  ```ts
  // pseudocode for the planner — tdd-developer writes the actual code
  export function useJiraFetch(): {
    fetch: (key: string) => Promise<JiraTicketDto>;
    isLoading: boolean;
    error: string | null;
  } {
    // useState for isLoading + error
    // fetch wraps invoke<JiraTicketDto>('fetch_jira_ticket', { key })
    //   try/catch sets/clears error and toggles isLoading
  }
  ```
  Re-throw on error so callers can chain `.catch` for UI feedback,
  but also store the error string in state for display.

**Refactor**: None.

**Commit**: `feat(web): add useJiraFetch() hook + JiraTicketDto type`

**Verification**: `pnpm -F @agentic/web-ui test useJiraFetch`

**Notes**:
- The `isAvailable` flag was originally proposed but **omitted** here
  — env-var presence is a server-side concern, surfaced via the
  error message ("missing environment variables: …"). SpecDialog
  parses that error to populate the disabled-tooltip in F.2.4.
- This avoids adding a second IPC just to introspect env vars.

---

### [x] Step F.2.4: SpecDialog Jira-pull row + tooltip + validation

**Goal**: Add a new row **above the title input** in `SpecDialog`
containing a key input + "Pull from Jira" button. Wire to
`useJiraFetch()`. Behavior:
- Client-side regex `^[A-Z][A-Z0-9]+-\d+$` gates the button-enabled
  state (button is disabled until a valid key is typed).
- On click: invoke `fetch_jira_ticket(key)`. While in flight,
  disable the button.
- On success: populate `title` state with `dto.title`; populate
  `body` state with `dto.body + (dto.ac ? "\n\n## Acceptance Criteria\n" + dto.ac : "")`.
- On error: append the error string to a local `pullError` state and
  render below the row. Pre-flight "missing environment variables: X, Y"
  errors are surfaced verbatim so the user knows what to set.
- Button is **always visible**. When the latest fetch failed with
  "missing environment variables: …", show that string as the
  button's `title` attribute (tooltip). Once the user fixes env vars
  and the next call succeeds (or returns a different error), the
  tooltip clears.

**Depends on**: F.2.3.

**Test first** (RED):
- Extend `apps/web-ui/src/__tests__/SpecDialog.test.tsx` (or create if
  absent — check first):
  - `it("renders the jira-pull row above the title input")`: open
    dialog, assert DOM order: `spec-dialog-jira-key-input`,
    `spec-dialog-jira-pull-button`, then `spec-dialog-title-input`.
  - `it("disables the pull button for invalid keys")`: type
    `"proj-1"` (lowercase) into key input, assert
    `spec-dialog-jira-pull-button` has `disabled` attribute.
  - `it("enables the pull button for valid keys")`: type `"PROJ-1"`,
    assert button is not disabled.
  - `it("populates title and body on successful pull")`: mock invoke
    to resolve with `{ key: "PROJ-1", title: "Fix bug", body: "Steps:\n1. …", ac: "Given X, when Y, then Z" }`.
    Type valid key, click button. Assert
    `spec-dialog-title-input` has value `"Fix bug"` and
    `spec-dialog-body-textarea` has value
    `"Steps:\n1. …\n\n## Acceptance Criteria\nGiven X, when Y, then Z"`.
  - `it("appends only body when ac is null")`: same setup but
    `ac: null`. Body textarea has value `"Steps:\n1. …"` (no AC
    section).
  - `it("renders the missing-env error inline")`: mock invoke to
    reject with `"missing environment variables: JIRA_URL, JIRA_EMAIL"`.
    Click pull button. Assert dialog renders an element with
    text matching `/missing environment variables: JIRA_URL, JIRA_EMAIL/`
    (use a `data-testid="spec-dialog-jira-pull-error"`).
  - `it("disables the button while fetch is in flight")`: deferred
    promise mock; click button; assert disabled state during
    flight; resolve; assert re-enabled.

**Implement** (GREEN):
- Edit `apps/web-ui/src/components/SpecDialog.tsx`:
  - Add `import { useJiraFetch } from "../hooks/useJiraFetch";`.
  - Add local state:
    `const [jiraKey, setJiraKey] = useState("")`,
    `const [pullError, setPullError] = useState<string | null>(null)`.
  - Call `const { fetch: fetchJira, isLoading: pulling } = useJiraFetch();`.
  - Compute `const keyValid = /^[A-Z][A-Z0-9]+-\d+$/.test(jiraKey);`.
  - New JSX block at the top of the body div (before the title
    input), with two siblings: `<input data-testid="spec-dialog-jira-key-input">`
    and `<button data-testid="spec-dialog-jira-pull-button">`.
    Button:
    `disabled={!keyValid || pulling}`,
    `title={pullError?.startsWith("missing environment variables") ? pullError : undefined}`,
    `onClick` handler that calls `fetchJira(jiraKey).then(dto => { setTitle(dto.title); setBody(dto.body + (dto.ac ? '\\n\\n## Acceptance Criteria\\n' + dto.ac : '')); setPullError(null); }).catch(e => setPullError(String(e)))`.
  - Render `pullError` inline below the row (only when truthy):
    `<p data-testid="spec-dialog-jira-pull-error" className="text-[11px] text-red-600">{pullError}</p>`.

**Refactor**: If the new row's classNames bloat the JSX, extract a
small inline component within the file. Skip if cosmetic.

**Commit**: `feat(web): add Jira ticket pull to SpecDialog`

**Verification**:
- `pnpm -F @agentic/web-ui test SpecDialog`
- `pnpm -F @agentic/web-ui typecheck`

**Notes**:
- Body field is shipped to the backend; per CLAUDE.md note, the
  body-drop bug at the IPC layer is GH #92's concern and explicitly
  out of scope for F.2.
- The button's `title` attribute is a basic browser tooltip — no
  custom popover library. Sufficient for the disabled-state hint.
- Pull-error rendering is intentionally simple (single `<p>`); a
  styled banner can be a follow-up tech-debt if reviewer flags.
- AC field formatting: literal `\n\n## Acceptance Criteria\n` prefix
  matches the body markdown convention in the existing AC fixtures.

---

### [x] Step F.2.5: SpecDialog → IPC integration smoke test

**Goal**: An end-to-end-flavored test that renders the full
SpecDialog flow from "user types a Jira key" → "title and body
populated" → "user clicks Create & run" with a mocked `invoke` that
returns realistic shapes for both `fetch_jira_ticket` and (the
existing) `start_ticket_run`. Acts as a regression guard for
F.2.1–F.2.4 wiring.

**Depends on**: F.2.4.

**Test first** (RED):
- New test file (or section in `SpecDialog.test.tsx`)
  `apps/web-ui/src/__tests__/SpecDialogJiraIntegration.test.tsx`:
  - `it("end-to-end: user pulls from Jira, edits, submits")`:
    1. Mount `<SpecDialog open onClose={…} onSubmit={onSubmit} />`.
    2. Mock `invoke('fetch_jira_ticket', { key: 'PROJ-42' })` →
       `{ key: "PROJ-42", title: "Refactor X", body: "Why\n…", ac: "AC text" }`.
    3. Type `PROJ-42` into key input. Click pull button.
    4. Wait for title input value === `"Refactor X"`.
    5. Append text to body via additional typing.
    6. Click `spec-dialog-submit`.
    7. Assert `onSubmit` was called once with the populated title and
       the augmented body containing `"## Acceptance Criteria"`.
  - `it("end-to-end: pull error does not block manual entry")`:
    1. Mock `invoke('fetch_jira_ticket', …)` → reject with
       `"missing environment variables: JIRA_URL"`.
    2. Type invalid env path; click pull. Assert pull-error rendered.
    3. Type a title manually. Click submit. Assert `onSubmit`
       received the manually-typed title (i.e., pull failure does not
       leave the dialog in a stuck state).

**Implement** (GREEN): No production code changes expected — F.2.4
already wired the flow. If the integration test exposes a missed
edge case, the fix lands here as a small refinement to
`SpecDialog.tsx`.

**Refactor**: None.

**Commit**: `test(web): integration test for SpecDialog jira-pull flow`

**Verification**: `pnpm -F @agentic/web-ui test SpecDialogJiraIntegration`

**Notes**:
- Keep this test fast (< 200ms). If the deferred promise pattern
  introduces flake, prefer `await waitFor(...)` over fixed timeouts.
- This step intentionally shipped as `test(web): …` not `feat(web): …`
  — it's a regression guard, not new functionality.

---

### Phase F.2 status checklist

- [x] F.2.1 fetch_jira_ticket IPC + DTO + helpers
- [x] F.2.2 Permission TOML + capability registration
- [x] F.2.3 useJiraFetch() hook + JiraTicketDto type
- [x] F.2.4 SpecDialog jira-pull row + validation
- [x] F.2.5 SpecDialog → IPC integration smoke test

---

## Phase G — Backend-scoped agent discovery + Copilot end-to-end

**Goal**: Today's `discover_agent(repo_root, name)` blends Claude Code and
Copilot project conventions into a single hard-coded search list, with no
backend awareness. When a user runs the Copilot backend, the chat
pre-flight still complains about missing files in `.claude/agents/` even
when they have valid `.github/agents/` files. Phase G makes discovery
backend-aware, promotes `BackendKind` to `agentic-core`, and adds a live
Copilot smoke test mirroring the existing claude-code one.

**Triage outcomes baked in (from architect / user Q&A)**:
- Q1=b: drop the legacy `<repo>/agents/` path entirely. `.agentic/agents/`
  remains the universal first-priority override (backend-agnostic).
- Q2=a: breaking change — `discover_agent` gains a `BackendKind` argument.
  All callers (CLI `ticket_run.rs`, Tauri `pre_flight_check`) and the
  ~12 invocations across the discovery test suite are updated in the
  same step.
- Q3=a: promote `BackendKind` to `agentic-core` (`backends::BackendKind`).
  CLI + Tauri private enums collapse to a thin wrapper (`clap::ValueEnum`
  derive in CLI; `serde::Deserialize` derive in Tauri) — or the wrappers
  are deleted if the core enum can carry both derives behind feature
  flags.
- Q4=a: error message lists every path checked **for the current
  backend** (verbose, unambiguous).
- Q5=b: H.x DnD bug fixed inline — no GH issue.
- Q6=a: H.1 RED test code-only against the real `usePipelineMutation`
  hook. Existing test at `AppPipelineMutation.test.tsx:130` already
  passes; agent must add a NEW failing test that reproduces the live
  failure mode, or document a hypothesis if JSDOM cannot reproduce
  HTML5-DnD-only bugs.
- Q7=merge: single Phase G with 5 sub-steps + Phase H with 2.

### Open implementation questions (defer to user before G.1 dispatch)

1. **`BackendKind` derive macros across crates**: the core enum needs
   `serde::{Serialize, Deserialize}` (for Tauri IPC) + `clap::ValueEnum`
   (for the CLI). Adding `clap` as a `agentic-core` dependency is
   undesirable — `clap` is a binary-tier crate, not a library
   dependency. Three options:
   a. Keep the CLI's local `BackendKind` as a thin wrapper deriving
      `ValueEnum` and `From<core::BackendKind>` / `Into`. Tauri uses
      the core enum directly with `serde`.
   b. Gate `clap::ValueEnum` behind a `clap` feature in `agentic-core`
      (compiles cleanly without it for non-CLI consumers).
   c. Both crates keep their own enums but each impls
      `From<&core::BackendKind>` + a parser that delegates to core.
   **Recommendation**: option (a) — minimum coupling, no new feature
   gates. Confirm before G.1.

2. **Tauri pre-flight backend wiring**: G.3 changes the signature to
   accept `BackendKind`, which is already the case. The change is the
   *discovery call* now also takes `BackendKind`. Confirm: should the
   error message in `pre_flight_check` enumerate the four searched
   paths verbatim (e.g. ``checked: .agentic/agents/architect.md,
   .claude/agents/architect.md, $HOME/.claude/agents/architect.md``)
   or just say "checked 3 locations under <root>"? Q4=a implies
   verbatim — confirm format.

3. **G.4 Copilot live test scope**: the existing
   `e2e_permissions_live.rs` is 200+ lines of palindrome scaffolding
   (git init, fixture files, permission gate exercise). For Copilot
   we need a leaner version that just confirms `start_ticket_run`-like
   wiring drives a real `CopilotCliBackend` for a one-step pipeline.
   Confirm: full palindrome parity, or a minimal "execute one
   reviewer agent against an empty sandbox and assert events flow"
   test? **Recommendation**: minimal — the claude test already covers
   permission-gate semantics; the Copilot one is about backend
   plumbing, not gate behaviour.

4. **H.1 reproducibility risk**: the existing reorder test
   (`AppPipelineMutation.test.tsx:130`) uses `fireEvent.dragStart /
   dragOver / drop` and passes against `main`. If the live failure is
   browser-only (HTML5 DnD effects, pointer-events CSS, z-index
   stacking, native-vs-synthetic event mismatch), there may be no
   deterministic JSDOM RED test. In that case H.1's Test-first step
   becomes "diagnostic write-up and hypothesis", and H.2 lands the
   fix + a tighter unit test that *does* fail before the fix. Confirm
   this is acceptable as a fallback before dispatching tdd-developer
   on H.1.

---

### Step G.1: Promote `BackendKind` to `agentic-core`

**Goal**: One canonical `BackendKind` enum lives in
`agentic-core::backends::BackendKind`. CLI + Tauri callers reuse it
(directly or via thin wrappers — see open question 1). Eliminates
the two near-identical private copies that today drift independently
(today: CLI uses `Copy + Clone`, Tauri uses `Deserialize` only).

**Depends on**: triage decisions above (specifically open question 1).

**Test first** (RED):
- New test file
  `crates/agentic-core/tests/backend_kind.rs`:
  - `it("BackendKind::id_str returns the stable backend id")`:
    `assert_eq!(BackendKind::ClaudeCode.id_str(), "claude-code");`
    and `assert_eq!(BackendKind::CopilotCli.id_str(), "copilot-cli");`.
  - `it("BackendKind::parse round-trips id_str values")`:
    `BackendKind::parse("claude-code").unwrap() == BackendKind::ClaudeCode`,
    `BackendKind::parse("copilot-cli").unwrap() == BackendKind::CopilotCli`,
    and `BackendKind::parse("scripted").is_err()` returns an error
    string containing both valid options.
  - `it("BackendKind serializes via serde to its id_str")`:
    `serde_json::to_value(&BackendKind::ClaudeCode).unwrap() == json!("claude-code")`
    (use `#[serde(rename_all = "kebab-case")]` to match Tauri's
    existing IPC contract).
  - `it("BackendKind deserializes from kebab-case strings")`:
    `serde_json::from_value::<BackendKind>(json!("claude-code")).unwrap() == BackendKind::ClaudeCode`.
- Migrate the existing CLI test
  `crates/agentic-cli/src/main.rs:516-521` to assert that the
  CLI wrapper round-trips correctly into `core::BackendKind` (or
  delete the test if the wrapper is gone).

**Implement** (GREEN):
- Add `pub enum BackendKind { ClaudeCode, CopilotCli }` to
  `crates/agentic-core/src/backends/mod.rs`. Derive
  `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`
  with `#[serde(rename_all = "kebab-case")]`.
- Implement `id_str(self) -> &'static str` and
  `pub fn parse(raw: &str) -> Result<Self, String>` on the new
  enum (mirrors today's CLI/Tauri impls).
- Re-export from `agentic-core/src/lib.rs` next to `BackendId`.
- Update CLI `crates/agentic-cli/src/main.rs:21-36`:
  - Keep a thin local `enum BackendKind` deriving `ValueEnum` with
    the same two variants, plus
    `impl From<BackendKind> for agentic_core::BackendKind`. Local
    `id_str` delegates.
  - Tests at `:520-521` updated to assert via the core enum's
    `id_str` after conversion.
- Update Tauri `crates/agentic-tauri/src/commands/ticket.rs:41-65`:
  - Delete the local `BackendKind` enum entirely.
  - `start_ticket_run` parses `backend: String` via
    `agentic_core::BackendKind::parse(&backend)` directly.
  - All `BackendKind::ClaudeCode | BackendKind::CopilotCli` matches
    in `pre_flight_check` and the `start_ticket_run` body switch
    onto the core enum (no signature change visible).
- `pre_flight_check(ws_root, &BackendKind)` keeps the same signature;
  the type is now the core type.

**Refactor**:
- If both old enums collapse cleanly, also collapse the
  `id_str().to_string()` calls into a `Display` impl on
  `core::BackendKind`. Optional — skip if tests pass without it.

**Files**:
- `crates/agentic-core/src/backends/mod.rs` (new enum)
- `crates/agentic-core/src/lib.rs` (re-export)
- `crates/agentic-core/tests/backend_kind.rs` (new)
- `crates/agentic-cli/src/main.rs` (wrapper + delegation)
- `crates/agentic-tauri/src/commands/ticket.rs` (delete local enum)

**Commit**: `refactor(core): promote BackendKind to agentic-core`

**Verification**:
```
cargo test -p agentic-core --test backend_kind
cargo test -p agentic-cli --lib
cargo test -p agentic-tauri --lib
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

---

### [x] Step G.2: Backend-scoped `discover_agent`

**Goal**: Discovery searches only the paths relevant to the active
backend, with `.agentic/agents/` as the universal first override.
Drops the legacy `<repo>/agents/` path.

**Depends on**: G.1 (consumes `core::BackendKind`).

**Test first** (RED): the existing
`crates/agentic-core/tests/agents_discovery.rs` (9 `#[test]` fns,
12 `discover_agent*` invocations) is rewritten to thread
`BackendKind` through every call. Each test asserts the new search
order:

  - **ClaudeCode order**: (1) `<repo>/.agentic/agents/`, (2)
    `<repo>/.claude/agents/`, (3) `$HOME/.claude/agents/`.
  - **CopilotCli order**: (1) `<repo>/.agentic/agents/`, (2)
    `<repo>/.github/agents/`, (3) `$HOME/.copilot/agents/`.
  - Legacy `<repo>/agents/` is **never** consulted.

Specific test rewrites (each test passes a `BackendKind` matching its
intent — DO NOT downgrade tests just to compile; each must assert
the right backend's intent):
- `agentic_agents_dir_wins_over_claude_github_and_legacy` →
  `agentic_dir_wins_for_claude_code` (drop the legacy and github
  arms; assert `.agentic/` beats `.claude/` for ClaudeCode) +
  `agentic_dir_wins_for_copilot_cli` (assert `.agentic/` beats
  `.github/` for CopilotCli).
- The legacy `<repo>/agents/` test (line ~140-155 if present) is
  deleted entirely; replace with
  `legacy_repo_agents_dir_is_ignored` that writes a file there
  and asserts `discover_agent` returns `AgentNotFound`.
- `home_claude_agents_dir_resolves_when_repo_empty` → splits into
  `home_claude_resolves_for_claude_code` (writes to
  `$HOME/.claude/agents/`, calls discovery with `ClaudeCode`,
  expects success) and
  `home_claude_ignored_for_copilot_cli` (same setup, calls discovery
  with `CopilotCli`, expects `AgentNotFound`).
- `home_copilot_agents_dir_resolves_when_repo_empty` → mirror
  twin: `home_copilot_resolves_for_copilot_cli` +
  `home_copilot_ignored_for_claude_code`.
- `default_discover_agent_resolves_real_home_without_panicking` →
  parameterise the backend (call once with each variant); both
  must not panic.
- `agent_not_found_lists_searched_paths` → assert the returned
  `searched: Vec<PathBuf>` contains exactly **3** entries (not 6)
  and they are the backend's three paths in order.

**Implement** (GREEN):
- `crates/agentic-core/src/agents/discovery.rs`:
  - `pub fn discover_agent(backend: BackendKind, repo_root: &Path, name: &str) -> Result<Agent>`
  - `pub fn discover_agent_with_home(backend: BackendKind, repo_root: &Path, home: Option<&Path>, name: &str) -> Result<Agent>`
  - `fn candidate_paths(backend: BackendKind, repo_root: &Path, home: Option<&Path>, name: &str) -> Vec<PathBuf>`:
    - Always start with `<repo_root>/.agentic/agents/<name>.md`.
    - For `ClaudeCode`: append `<repo_root>/.claude/agents/<name>.md`,
      then (if home) `<home>/.claude/agents/<name>.md`.
    - For `CopilotCli`: append `<repo_root>/.github/agents/<name>.md`,
      then (if home) `<home>/.copilot/agents/<name>.md`.
    - Drop the legacy `<repo_root>/agents/<name>.md` and the
      cross-backend home paths.
  - Update the doc comment to spell out the new search order per
    backend.
- Update both callers:
  - `crates/agentic-cli/src/ticket_run.rs:235` —
    `discover_agent(ws_root, &pipeline_step.agent)` →
    `discover_agent(ctx.backend_kind, ws_root, &pipeline_step.agent)`
    (the `PipelineRunContext` already carries the backend kind via
    G.1's wrapper conversion; if not, add a field).
  - `crates/agentic-tauri/src/commands/ticket.rs:266` —
    `agentic_core::discover_agent(ws_root, name)` →
    `agentic_core::discover_agent(*backend_kind, ws_root, name)`.

**Refactor**:
- If `candidate_paths` becomes a `match backend { … }` block more
  than ~15 lines, extract per-backend helpers
  (`claude_paths`, `copilot_paths`) — optional cosmetic.

**Files**:
- `crates/agentic-core/src/agents/discovery.rs`
- `crates/agentic-core/tests/agents_discovery.rs` (rewrite all 9 tests)
- `crates/agentic-cli/src/ticket_run.rs`
- `crates/agentic-tauri/src/commands/ticket.rs`

**Commit**: `feat(core): scope agent discovery by backend kind`

**Verification**:
```
cargo test -p agentic-core --test agents_discovery
cargo test -p agentic-cli
cargo test -p agentic-tauri
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

---

### Step G.3: Pre-flight error lists searched paths

**Goal**: When pre-flight discovery fails, the chat error message
lists every path checked **for the active backend** so the user
knows exactly where to drop the file. Today's message just says
`agent not found under <ws_root>`, which is not actionable when the
user has files at `.github/agents/` and the backend is ClaudeCode
(they can't tell why discovery isn't seeing them).

**Depends on**: G.2.

**Test first** (RED):
- New test file (or section in the existing
  `crates/agentic-tauri/tests/pre_flight_check.rs` if it exists; if
  not, create `crates/agentic-tauri/tests/pre_flight.rs`):
  - Pre-flight test infrastructure may need extracting
    `pre_flight_check` from `commands/ticket.rs` into a `pub(crate)`
    helper module so tests can call it directly. If that's a
    significant refactor, add it as the first sub-action of GREEN.
  - `it("error message for missing claude-code agent lists the three claude paths")`:
    Empty tmpdir, empty home tmpdir (via env override or with a
    test-only `pre_flight_check_with_home`). Call with
    `BackendKind::ClaudeCode`. Assert the returned `Err(String)`
    contains all three of:
    `.agentic/agents/architect.md`, `.claude/agents/architect.md`,
    `<home>/.claude/agents/architect.md`. Assert it does **not**
    contain `.github/agents/` or `.copilot/`.
  - `it("error message for missing copilot-cli agent lists the three copilot paths")`:
    same setup with `BackendKind::CopilotCli`. Asserts the message
    contains `.agentic/`, `.github/`, and `<home>/.copilot/` paths,
    and does **not** contain `.claude/`.
  - `it("error message still includes the install hint for the backend binary if missing")`:
    regression guard for the existing binary-on-PATH check. Set
    `CLAUDE_CODE_BIN=/nonexistent/path/to/claude`, call pre-flight,
    assert message contains `Install Claude Code`.

**Implement** (GREEN):
- Modify `pre_flight_check` in
  `crates/agentic-tauri/src/commands/ticket.rs` (~line 263-274):
  - On `discover_agent(...).is_err()`, instead of returning a
    pre-baked `format!`, downcast to the actual `CoreError` to
    extract the `searched: Vec<PathBuf>` field. Format each path
    on its own line (or comma-separated if a single line is
    preferable for the chat surface).
  - New error string template:
    `"pre-flight: agent `{name}` not found for backend `{backend}`.
    Checked:\n  - {path1}\n  - {path2}\n  - {path3}\n
    Run `agentic-cli init{flag}` to scaffold the four required agents."`
    where `{flag}` is `--copilot` for CopilotCli and empty for
    ClaudeCode.
- Apply the same change to any pre-flight in
  `crates/agentic-cli/src/ticket_run.rs` (audit: there may not be
  one — confirm during implementation).
- Pull `pre_flight_check` into a `pub(crate)` testable helper if
  needed.

**Refactor**:
- If the error formatter is more than ~10 lines, extract a free
  function `format_agent_not_found_error(name, backend, searched)`
  for testability.

**Files**:
- `crates/agentic-tauri/src/commands/ticket.rs` (or new helper module)
- `crates/agentic-tauri/tests/pre_flight.rs` (new)

**Commit**: `feat(tauri): pre-flight lists searched agent paths per backend`

**Verification**:
```
cargo test -p agentic-tauri
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

---

### Step G.4: Live Copilot end-to-end smoke test

**Goal**: Mirror the existing `e2e_permissions_live.rs` (claude-code)
with a Copilot equivalent, so we have an `#[ignore]`d smoke test that
exercises the real `CopilotCliBackend` against a one-step pipeline.
Skipped when `copilot --version` is unavailable (mirrors the claude
test's binary guard). Confirms G.1+G.2's wiring against the actual
binary the user has installed.

**Depends on**: G.2 (backend-scoped discovery), G.3 optional but
preferred (better failure messages while debugging).

**Test first** (RED):
- New test file
  `crates/agentic-cli/tests/e2e_copilot_live.rs`:
  - `#[ignore = "live: requires `copilot` CLI on PATH; run via:
    cargo test -p agentic-cli --test e2e_copilot_live -- --ignored --nocapture"]`
  - `#[tokio::test] async fn copilot_one_step_pipeline_runs_against_real_copilot_cli()`.
  - Step 1: Skip guard.
    `Command::new("copilot").arg("--version").output()` —
    on `Err` or non-zero status, `eprintln!` and `return`.
    (Use the same shape as
    `e2e_permissions_live.rs:30-56` for consistency.)
  - Step 2: Sandbox tmpdir + `.agentic/agents/reviewer.md` agent
    file (writes a minimal valid TOML+frontmatter agent that asks
    Copilot to print "ok"; the agent body is intentionally tiny
    to keep the smoke test fast and deterministic).
  - Step 3: git init the sandbox (Copilot may require it; mirror
    the claude test's approach).
  - Step 4: Construct a one-step `Pipeline` with the `reviewer`
    agent and `BackendKind::CopilotCli`.
  - Step 5: Use `execute_pipeline(...)` (the same helper the claude
    test uses) with a `BackendFactory` that constructs
    `CopilotCliBackend::default()`.
  - Step 6: Assert the run completes (status `Completed` or
    `Failed`; we accept either — this is a smoke test for *plumbing*,
    not a quality gate). Assert at least one `Event::StepStarted`
    and one `Event::StepCompleted` envelope was emitted via the
    bus. If the run fails because Copilot rejects the prompt,
    `eprintln!` the summary so the user can see why and the test
    still completes (don't `panic!`) — the goal is "did our wiring
    drive the binary?", not "did Copilot succeed?".

**Implement** (GREEN):
- The test itself IS the deliverable — the production code
  shouldn't need changes if G.1+G.2 are correct.
- If the test reveals a wiring gap (e.g., `BackendFactory` isn't
  parameterised by `BackendKind` and silently constructs
  `ClaudeCodeBackend` for both branches), fix it here. Likely
  candidates: `crates/agentic-cli/src/main.rs:409-414` switch arms
  or a missing `impl From<BackendKind> for Box<dyn Backend>`.
- Update `crates/agentic-cli/Cargo.toml` `[dev-dependencies]` only
  if a new test-only crate is needed (e.g., `tempfile` should
  already be there — confirm).

**Refactor**:
- If most of `e2e_permissions_live.rs` and the new file are
  duplicated, extract a `tests/common/live_smoke.rs` mod with the
  shared sandbox + git-init helper. Optional — only if duplication
  is more than ~30 lines.

**Files**:
- `crates/agentic-cli/tests/e2e_copilot_live.rs` (new)
- `crates/agentic-cli/tests/common/` (only if shared helpers are
  extracted)
- `crates/agentic-cli/src/main.rs` (only if a wiring gap is
  uncovered)

**Commit**: `test(cli): add live e2e smoke test for copilot backend`

**Verification**:
```
# Default (skipped when binary missing — should pass):
cargo test -p agentic-cli --test e2e_copilot_live
# Live (manual, user has copilot CLI on PATH):
cargo test -p agentic-cli --test e2e_copilot_live -- --ignored --nocapture
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

---

### Step G.5: Documentation update

**Goal**: Bring `docs/SMOKE.md`, `docs/redesign/spec.md`, and
`crates/agentic-cli/src/init.rs` help text in sync with the new
backend-scoped discovery + dropped legacy path.

**Depends on**: G.2, G.3, G.4.

**Test first** (RED): Lightweight — a doc-link / contract test:
- New test in
  `crates/agentic-cli/tests/init.rs` (extend existing file):
  - `it("init --copilot writes to .github/agents/")`: regression
    guard that confirms `agentic-cli init --copilot` creates
    `.github/agents/architect.md` (etc.) — assert the four agent
    filenames exist after running init in a tmpdir.
  - `it("init (default) writes to .claude/agents/")`: same shape
    for the default flag.
  - `it("init does NOT write to legacy <repo>/agents/")`: regression
    guard for the dropped path.

**Implement** (GREEN):
- `docs/SMOKE.md`:
  - Add a "Live Copilot smoke test" section that mirrors the
    existing claude one, pointing at the new
    `e2e_copilot_live.rs` invocation.
  - Update the agent-discovery search-order section (if present)
    to spell out the per-backend paths and the dropped legacy
    path.
- `docs/redesign/spec.md` §"Agent discovery" (or wherever the
  search order is documented): rewrite to two enumerations
  (ClaudeCode vs CopilotCli) sharing `.agentic/` as the universal
  first override.
- `crates/agentic-cli/src/init.rs` doc comment + `--help` text:
  remove any mention of `<repo>/agents/` legacy path. Confirm
  `--copilot` and `--global` flag descriptions align with G.2's
  semantics (no behavioural change expected, just doc accuracy).
- `crates/agentic-cli/src/main.rs:84-94` Init doc comment: same
  cleanup.
- README at repo root (if it documents agent paths): same update.

**Refactor**: None — pure docs + assertion regressions.

**Files**:
- `docs/SMOKE.md`
- `docs/redesign/spec.md`
- `crates/agentic-cli/src/init.rs`
- `crates/agentic-cli/src/main.rs` (Init doc comment only)
- `crates/agentic-cli/tests/init.rs` (extend)
- `README.md` (if it mentions agent paths)

**Commit**: `docs(redesign): document backend-scoped agent discovery + copilot smoke`

**Verification**:
```
cargo test -p agentic-cli --test init
# Manual: skim docs/SMOKE.md and docs/redesign/spec.md for
# remaining "<repo>/agents/" references; rg should find none.
rg -n "repo>/agents|legacy agents" docs/ crates/agentic-cli/src/
```

---

### Phase G status checklist

- [x] G.1 promote BackendKind to agentic-core
- [ ] G.2 backend-scoped `discover_agent`
- [x] G.3 pre-flight error lists searched paths
- [x] G.4 live copilot smoke test
- [x] G.5 docs + init regression tests

---

## Phase H — Pipeline DnD reorder regression fix

**Goal**: User reports that drag-and-drop reorder of pipeline cards
produces wrong ordering in the live UI (Tauri shell + Vite dev). The
existing unit test at
`apps/web-ui/src/__tests__/AppPipelineMutation.test.tsx:130` simulates
`fireEvent.dragStart / dragOver / drop` against the real
`usePipelineMutation` hook and **passes**. So the bug is either:
(a) a JSDOM ↔ HTML5 native DnD divergence the synthetic events
don't reach, (b) a stale closure / ref bug in `PipelineBar`'s
`useDragReorder` that only manifests after multiple drags, or (c) a
wiring gap between `App.tsx`'s `onReorder={onReorder}` and the
hook's `setPipelineAgents` (e.g., a memoisation that captures stale
state).

**Triage outcomes baked in**:
- Q5=b: no GH issue — fixing inline.
- Q6=a: code-only RED test against the real
  `usePipelineMutation` hook in the existing test file. If unable
  to reproduce in JSDOM, document the hypothesis in the plan (this
  step) and let GREEN verify in dev mode.

### Step H.1: RED test reproducing live failure mode (or hypothesis if JSDOM cannot)

**Goal**: Write a failing test that reproduces the live failure, OR
a diagnostic write-up explaining why JSDOM can't see it. Either way,
H.2 has a pinpoint target.

**Depends on**: nothing (independent of Phase G).

**Test first** (RED):
- New test cases in
  `apps/web-ui/src/__tests__/AppPipelineMutation.test.tsx` (in the
  same `describe("App pipeline mutation — W.9.1")` block, after the
  existing `it("reorder: drag architect(0) to gap-3 …")`):
  - **Hypothesis A — multi-drag stale state**:
    `it("reorder: two consecutive drags both apply (state isn't stale)")`.
    Render `<App />`. Drag architect(0) → gap-3 (assert order
    becomes `[tdd-developer, qa, architect, reviewer]`). Then
    immediately drag reviewer (now at index 3) → gap-1 (assert
    order becomes `[tdd-developer, reviewer, qa, architect]`).
    If the second drag doesn't apply (the visible order stays at
    the post-first-drag state), the bug is a stale closure in
    `useDragReorder` capturing `dragFromIndex` from the first
    invocation.
  - **Hypothesis B — adjusted-index off-by-one for backwards drag**:
    `it("reorder: drag tdd-developer(1) to gap-0 moves it to index 0")`.
    Render `<App />`. Drag the second card to gap-0. Expected:
    `[tdd-developer, architect, qa, reviewer]`. The current
    `PipelineBar:43` logic computes
    `adjusted = dragFromIndex < rightIndex ? rightIndex - 1 : rightIndex`,
    so `from=1, gap=0` → `1 < 0 ? -1 : 0 = 0` ✓. But the inverse
    `from=0, gap=2` (forward drag *exactly past* the next card) is
    untested and may be where the bug lives.
  - **Hypothesis C — gap-after-self drag is a no-op or wrong**:
    `it("reorder: drag architect(0) to gap-1 (the gap immediately after itself) is a no-op")`.
    `from=0, gap=1` → `adjusted = 0 < 1 ? 0 : 1 = 0`. Assert order
    is unchanged.
- Add a comment block at the top of the new test cases explaining
  which hypothesis each guards against. The agent's RED report
  documents which (if any) of the three actually fails on `main`.
- **Fallback if all three pass**: the agent must add a doc comment
  block at the bottom of the test describe explaining that the
  bug is not reproducible in JSDOM, and propose a console-level
  reproduction recipe for the user to run in `pnpm tauri dev`
  (e.g., "drag X then Y then Z; expected vs. actual"). H.2 then
  becomes a hypothesis-driven fix without a unit-level RED.

**Implement** (GREEN): None at H.1 — this step is RED-only. The
commit is `test(web): …` and adds the failing test (or the
hypothesis comment block).

**Refactor**: None.

**Files**:
- `apps/web-ui/src/__tests__/AppPipelineMutation.test.tsx`

**Commit**: `test(web): reproduce pipeline DnD reorder regression (or document hypothesis)`

**Verification**:
```
pnpm -F @agentic/web-ui test AppPipelineMutation
# RED case: at least one new test fails.
# Fallback: tests pass + comment block added; surface to user
# for manual repro in `pnpm tauri dev`.
```

---

### Step H.2: Fix the regression

**Goal**: Land the fix that turns H.1's RED green. Commit message
references the symptom (which hypothesis fired) and the root cause.

**Depends on**: H.1.

**Test first** (RED): the test from H.1 — already in place. No new
test in H.2 unless the fix uncovers a related case.

**Implement** (GREEN): Driven by H.1's findings. Most likely
candidates listed in priority order so the agent investigates them
first:
1. **`apps/web-ui/src/components/PipelineBar.tsx:18-54`
   `useDragReorder`**: the `setDropGapIndex(null)` reset on drop
   (line ~46) and the `dragFromIndex` clear may race with a
   second drag's `dragstart` event if React batches them. Switch
   to `useRef` for `dragFromIndex` to avoid the stale-closure
   read inside `getGapHandlers().onDrop`.
2. **`apps/web-ui/src/utils/arrayMove.ts:12`
   `reorderArray`**: re-audit the `splice(from,1)` then
   `splice(to,0,removed)` semantics for the case where
   `from < to`. Today's logic: removing first shifts subsequent
   indices left by 1, so the destination index `to` already
   accounts for the removal. Verify this is consistent with
   `PipelineBar`'s `adjusted = from < right ? right-1 : right`
   — there may be a double-correction.
3. **`apps/web-ui/src/hooks/usePipelineMutation.ts:44-46`
   `onReorder`**: the closure captures `setPipelineAgents` (stable
   via React) and calls `reorderArray(prev, from, to)`. Stable
   today — but if `App.tsx:161` ever wraps it in `useCallback`
   with stale deps, the indices passed in could be from a stale
   render. Check the App-level wiring.
4. **CSS pointer-events / drop-zone hit-testing**: only relevant
   if H.1's fallback fires. Inspect `.flex h-[84px]` container
   and `data-drop-active` styling.

**Refactor**:
- If the fix is a `useRef` swap, also add a brief inline comment
  explaining the closure-staleness reason (so a future reader
  doesn't "simplify" it back to `useState`).

**Files** (likely subset, exact set determined by H.1 findings):
- `apps/web-ui/src/components/PipelineBar.tsx`
- `apps/web-ui/src/utils/arrayMove.ts`
- `apps/web-ui/src/hooks/usePipelineMutation.ts`

**Commit**: `fix(web): pipeline DnD reorder regression — <root cause>`

**Verification**:
```
pnpm -F @agentic/web-ui test AppPipelineMutation
pnpm -F @agentic/web-ui test arrayMove PipelineBar
pnpm -F @agentic/web-ui test
# Manual: pnpm tauri dev — drag pipeline cards in several
# sequences, verify visible order matches expected.
```

---

### Phase H status checklist

- [ ] H.1 RED test (or hypothesis write-up)
- [ ] H.2 fix the regression

---

## Phase I — User-owned pipeline (dynamic agents)

**Goal**: Today the pipeline is a 4-element hard-coded constant
(`architect → tdd-developer → qa → reviewer`) baked into
`PipelineConfig::builtin_default`, mirrored in
`apps/web-ui/src/types/run.ts::DEFAULT_AGENTS`, and asserted in
`pre_flight_check`'s for-loop. Phase I makes the pipeline **discovered
and user-controlled**:

1. Backend lists every agent file it can find for the active backend
   (project precedence over home, both shown to the user with a `source`
   tag). The four-name hardcoded list dies.
2. The pipeline that runs is whatever ordered list of agent names the
   user (or the chat / web UI / CLI flag) hands the IPC. No implicit
   fallback list.
3. `start_ticket_run` IPC takes `agents: Vec<String>` as a **required**
   arg; CLI gains a `--agents <name>[,<name>...]` flag with no default.
   No agents flag → CLI errors with an actionable message.
4. The web UI persists the user's chosen pipeline per-project in
   `localStorage` and replays it on next start.
5. `agentic-cli init` keeps existing semantics (skip-if-exists,
   `--force` to overwrite) but is **no longer required to run** —
   the pipeline is happy with zero, one, or N agents in any backend
   directory; the user just has to point at the ones they want.

**Triage outcomes baked in (from architect/user Q&A, all 12 confirmed):**

1. `AgentInfo` exposes `name`, `description`, `source`. No `model` or
   other frontmatter hints leak. (`description` comes from the existing
   `Agent.description` frontmatter field.)
2. Pipeline persistence = **localStorage per-project** for v1. Keys are
   namespaced by workspace id (the existing `stable_workspace_id`
   exposed via the `RunSummary` flow / a new lightweight IPC). File-
   based `pipeline.toml` persistence is deferred (see "Out of scope").
3. `start_ticket_run` signature change is breaking: the new IPC takes
   `agents: Vec<String>` as a required arg. **All callers updated in
   the same step that breaks the contract** — no parallel "legacy
   default" code path.
4. CLI `run --ticket` requires a non-empty `--agents` flag (or a future
   `pipeline.toml` entry). No implicit "all agents alphabetically"
   fallback. Empty `--agents` errors with: ``--agents is required:
   pass --agents architect,tdd-developer or run `agentic-cli list-agents`
   to see what's discoverable``.
5. Phase H DnD work folds into Phase I — the picker / reorder UX must
   keep working as the agent list grows past 4. (We do **not** restart
   Phase H; the existing H.1/H.2 stay independent.)
6. Backward compat for tests: tests update **in the same step that
   breaks the contract**. No "legacy default agents" shim in test
   helpers.
7. **TUI ripple is out of scope for Phase I** — the TUI's pipeline
   strip currently renders `RunState.steps` (already dynamic, fed by
   the events stream) but the keyboard shortcuts (`:add`, `:rm`) are
   no-ops. Wiring them to the new IPC + persistence lands with GH #103
   when that lands. Noted in "Out of scope" + a tech-debt entry.
8. `agentic-cli init` template versioning: **skip-if-exists** is the
   default (already implemented); `--force` overwrites. No new flag —
   just verify the existing `--force` still does the right thing
   against an enlarged template list, if any.
9. **Partial agent list runs**: a project with only `architect.md`
   discoverable can run a 1-agent pipeline. `start_ticket_run`'s
   orchestrator is generic over `agents.len() ∈ [1, ∞)`. `qa_fix_loop`
   coupling (today: tdd-developer ↔ qa, hard-coded by name) is dropped
   — see "Out of scope" item on per-step retry semantics.
10. `AgentInfo.path` is **server-side only**. The Tauri DTO exposed
    over IPC carries `name` + `description` + `source` ("project" |
    "home"). Path leakage would let the webview render absolute paths
    from the user's machine — avoid.
11. Discovery merges by agent name with **project precedence over
    home**. UI shows the agent's `source` ("project" | "home") so the
    user knows whether they're editing a repo file or a global one.
12. `RunStep.role` field is **removed entirely**. The DB-row already
    only has `agent_name` (`crates/agentic-core/src/db/steps.rs:15`),
    so the cleanup is in the events / Tauri DTO surface and any TUI /
    web-ui type that grew a `role` mirror. After Phase I, the only
    identifier is `agent: String` (+ `index: u32` where ordering
    matters).

### Out of scope (Phase I)

- **TUI dynamic-list integration**. The TUI already renders pipeline
  cards from `RunState.steps`, so the new agents-from-IPC path will
  flow through events without a TUI change. But the keyboard
  shortcuts (`:add <agent>`, `:rm <agent>`, `[a]dd`, `[r]eorder`,
  `[d]rop` from the pipeline strip footer) are still no-ops at
  state-mutation level. Wiring them to a new mutation IPC + the
  same localStorage-or-equivalent persistence layer is **deferred to
  GH #103**. Phase I gets a tech-debt entry that explicitly mirrors
  GH #103 + a manual-verify checklist (run a 2-agent pipeline from
  the CLI, watch the TUI render only those 2 cards).
- **File-based `pipeline.toml` persistence**. The existing
  `<repo>/.agentic/pipeline.toml` parser
  (`crates/agentic-core/src/pipeline/config.rs`) stays as-is and is
  **not** consumed by Phase I — `start_ticket_run` no longer falls
  back to `PipelineConfig::load(...).default_pipeline()`. A future
  phase reconciles the two surfaces (Q2 deferred); the trigger is
  "user wants the pipeline to survive a `localStorage.clear()`".
  Tech-debt entry filed.
- **Per-step retry semantics**. The current `qa_fix_loop_cap` field
  on `PipelineStep` couples `tdd-developer` and `qa` by literal name
  in `PipelineSm::new`
  (`crates/agentic-core/src/pipeline/sm.rs:60-62`). Phase I drops the
  coupling: a generic N-agent pipeline runs each step once in order,
  no automatic retry. If a user wants the old QA-retry loop, they
  re-list `tdd-developer` after `qa` in the agents array (manual). A
  proper "retry policy per step" surface lives in v2 if anyone
  misses it.

### Migration notes (breaking changes shipped in this phase)

- **`start_ticket_run` IPC adds a required `agents: string[]` field.**
  Every JS caller updates in the same step that breaks the Rust
  signature. Affected JS files (full list found via
  `rg -l 'start_ticket_run' apps/`):
  - `apps/web-ui/src/components/ChatPane.tsx:41`
  - `apps/web-ui/src/utils/createSpec.ts:15`
  - `apps/web-ui/src/utils/devInvokeMock.ts:118`
  - `apps/web-ui/src/__tests__/permissionFlow.integration.test.tsx`
  - `apps/web-ui/src/__tests__/devInvokeMock.test.ts`
  - `apps/web-ui/src/__tests__/app.test.tsx`
  - `apps/web-ui/src/__tests__/AppPolish.test.tsx`
  - `apps/web-ui/src/__tests__/ChatColumn.test.tsx`
  - `apps/web-ui/src/__tests__/createSpec.test.ts`
  - `apps/web-ui/src/__tests__/ChatPane.test.tsx`
  - `apps/web-ui/src/__tests__/IssueColumnSpecFlow.test.tsx`
  Affected Rust callers / tests:
  - `crates/agentic-tauri/src/commands/ticket.rs:50` (the IPC
    definition itself)
  - `crates/agentic-tauri/tests/ticket_ipc.rs`
  - `crates/agentic-cli/src/main.rs:73-90` (Run subcommand args)
  - `crates/agentic-cli/src/ticket_run.rs::cmd_run_ticket`
  - `crates/agentic-cli/src/ticket_run.rs` test scaffolding
  - `crates/agentic-cli/tests/e2e_permissions_live.rs`
  - `crates/agentic-cli/tests/e2e_copilot_live.rs`
  - `crates/agentic-cli/tests/execute_pipeline_file_changes.rs`
- **`RunStep.role` is removed everywhere.** The DB row
  (`crates/agentic-core/src/db/steps.rs::Step.agent_name`) is the
  source of truth. Any DTO that grew a parallel `role` field
  collapses onto `{ agent: String, index: u32 }`. Search anchor:
  `rg -n '\\brole\\b' crates/ apps/web-ui/src` — every match outside
  `chat::ChatMessage.role` ("user"/"assistant") is in scope.
- **`agentic-cli init` is no longer a precondition for running a
  pipeline.** The pre-flight check in
  `crates/agentic-tauri/src/commands/ticket.rs::pre_flight_check`
  switches from "every name in `["architect", "tdd-developer", "qa",
  "reviewer"]` resolves" to "every name in the **user-supplied**
  `agents` list resolves". Discovery's
  `CoreError::AgentNotFound` already lists searched paths and points
  at `agentic-cli init`, so the actionable hint stays. `init` itself
  is unchanged in behaviour (skip-if-exists default; `--force`
  overwrites).

### Step I.1: List discoverable agents (core, with `source` tag)

**Goal**: Add an `agentic_core::agents::list_discoverable` API that
returns every agent file resolvable for a `BackendKind` under a given
repo root + home, **deduplicated by name with project precedence over
home**, each tagged with `source: AgentSource::{Project, Home}`. The
existing `discover_agent` lookup-by-name stays unchanged; the new API
is the iteration surface.

**Depends on**: nothing inside Phase I (assumes Phase G's
`BackendKind`-aware discovery already landed, which it has).

**Test first** (RED):
- New test file `crates/agentic-core/tests/agents_list.rs`:
  - `it("list_discoverable returns project file with source = Project")`:
    write `<root>/.claude/agents/architect.md` with valid frontmatter.
    Call `list_discoverable(BackendKind::ClaudeCode, root, Some(home), &paths)` (no
    home file). Assert `[AgentInfo { name: "architect", description: "...",
    source: AgentSource::Project }]`.
  - `it("list_discoverable returns home file with source = Home when project absent")`:
    write only `<home>/.claude/agents/qa.md`. Assert
    `[AgentInfo { name: "qa", description: "...", source: AgentSource::Home }]`.
  - `it("list_discoverable: project precedence wins over home (single AgentInfo, source = Project)")`:
    write the same `architect.md` to **both** `<root>/.claude/agents/`
    and `<home>/.claude/agents/`, with **different** descriptions.
    Assert exactly 1 result; `source == Project`; `description`
    matches the project file. Order does not matter for this assertion
    (sort by name in the impl for determinism).
  - `it("list_discoverable returns empty for empty backend dirs")`:
    no agent files anywhere → `Ok(vec![])` (not an error).
  - `it("list_discoverable: agentic universal override beats both backend dirs")`:
    write `architect.md` to `<root>/.agentic/agents/` and
    `<root>/.claude/agents/` with different descriptions. Assert
    `source == Project` and the description matches the
    `.agentic/agents/` file.
  - `it("list_discoverable: copilot-cli only sees .github + ~/.copilot")`:
    write `qa.md` to `<root>/.claude/agents/`. Call with
    `BackendKind::CopilotCli`. Assert empty result (the claude file
    is invisible to copilot).
  - `it("list_discoverable: malformed frontmatter file is skipped, not fatal")`:
    write a valid `architect.md` and a malformed `qa.md` (missing
    closing `+++`). Assert the result contains only `architect`;
    no error returned, but a `tracing::warn!` event is emitted (use
    `tracing-test` or assert the function doesn't error).
- Add `pub struct AgentInfo { pub name: String, pub description: String,
  pub source: AgentSource }` and `pub enum AgentSource { Project, Home }`
  to `crates/agentic-core/src/agents/mod.rs` (in the test, import via
  `agentic_core::{AgentInfo, AgentSource}`). The test serves as the
  spec for the public types.

**Implement** (GREEN):
- Add `AgentInfo` + `AgentSource` to `crates/agentic-core/src/agents/mod.rs`,
  re-exported from `lib.rs` next to `Agent`. Derive `Debug`, `Clone`,
  `PartialEq`, `Eq`, `Serialize`, `Deserialize` (the IPC layer in I.5
  reuses the same struct).
- New module `crates/agentic-core/src/agents/list.rs` with:
  ```rust
  pub fn list_discoverable(
      backend: BackendKind,
      repo_root: &Path,
      home: Option<&Path>,
  ) -> Result<Vec<AgentInfo>>
  ```
  Walks the backend-scoped directories in priority order:
  - `<root>/.agentic/agents/` (universal override → `Project`)
  - `<root>/.{claude|github}/agents/` (→ `Project`)
  - `<home>/.{claude|copilot}/agents/` (→ `Home`)
  For each `*.md` in each dir, calls `parse_agent` to extract the
  frontmatter; on parse error, logs `tracing::warn!` and skips the
  file (one bad agent doesn't poison the whole list). Dedup by name,
  keeping the first match (priority order = project beats home). Sort
  results alphabetically by name for determinism.
- Re-export from `crates/agentic-core/src/agents/mod.rs::pub use list::list_discoverable;`
  and from `crates/agentic-core/src/lib.rs`.

**Refactor**: None this step.

**Files**:
- `crates/agentic-core/src/agents/mod.rs` (add types, re-export)
- `crates/agentic-core/src/agents/list.rs` (new)
- `crates/agentic-core/src/lib.rs` (re-export `AgentInfo`,
  `AgentSource`, `list_discoverable`)
- `crates/agentic-core/tests/agents_list.rs` (new)

**Commit**: `feat(core): list discoverable agents with project/home source tag`

**Verification**:
```
cargo test -p agentic-core --test agents_list
cargo clippy -p agentic-core --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

### Step I.2: Drop the hard-coded 4-agent assumption from pre-flight + tests

**Goal**: `pre_flight_check` switches from
``for name in ["architect", "tdd-developer", "qa", "reviewer"]`` to
``for name in agents.iter()``. The 4-name literal dies in this step.
This is the smallest possible change that lets I.4 add the IPC arg
without a temporary "use the old list when none provided" branch.

**Depends on**: I.1 (uses the same crate; not a hard dep but
keeps PR-sized changes coherent).

**Test first** (RED):
- Existing test
  `crates/agentic-tauri/tests/ticket_ipc.rs::pre_flight_check_*`
  (search for the test that today asserts all four names are
  required) — keep the assertion, but parameterise it on the
  agents-list arg.
- New test cases in `crates/agentic-tauri/tests/ticket_ipc.rs`:
  - `it("pre_flight_check passes for a 1-agent list when only that agent file exists")`:
    write only `<root>/.claude/agents/architect.md`. Call
    `pre_flight_check_with_home(root, &ClaudeCode, &["architect"], home)`.
    Assert `Ok(())`.
  - `it("pre_flight_check fails on first missing agent in the user list, not on the literal four")`:
    write `architect.md` only. Call with `&["architect", "reviewer"]`.
    Assert error message names `reviewer` (not `tdd-developer`).
  - `it("pre_flight_check rejects an empty agents list with an actionable error")`:
    Assert `Err` whose message contains `"agents list is empty"` or
    similar.

**Implement** (GREEN):
- Change `pre_flight_check` and `pre_flight_check_with_home` in
  `crates/agentic-tauri/src/commands/ticket.rs` to take
  `agents: &[String]` (or `&[&str]`) instead of relying on the
  hard-coded `["architect", "tdd-developer", "qa", "reviewer"]`.
- Empty `agents` slice → return `Err` with the message
  ``"pre-flight: agents list is empty — pass at least one agent in
  start_ticket_run.agents"``.
- Update the **two** internal callers (the IPC body and any tests
  that fed it via the public `pre_flight_check_with_home` re-export).
  At this step, the IPC body still hard-codes the four names *at
  the call site* — I.4 replaces that with the user-supplied list.
- Existing tests that asserted "all four required" become "all
  agents in the supplied slice required". No legacy default fallback.

**Refactor**:
- Rename the internal `for name in ["architect", ...]` loop variable
  from `name` to `agent_name` so the function reads naturally as
  "for each agent_name in agents".

**Files**:
- `crates/agentic-tauri/src/commands/ticket.rs`
- `crates/agentic-tauri/tests/ticket_ipc.rs`

**Commit**: `refactor(tauri): pre_flight_check takes user-supplied agents list`

**Verification**:
```
cargo test -p agentic-tauri --test ticket_ipc
cargo clippy -p agentic-tauri --all-features --all-targets -- -D warnings
```

---

### Step I.3: `execute_pipeline` accepts an arbitrary agent list (decouple qa-fix-loop)

**Goal**: Remove the `qa_fix_loop_cap` literal-name coupling in
`PipelineSm::new` and switch `execute_pipeline` from "consume a
pre-built `Pipeline` whose steps were assembled from
`PipelineConfig::builtin_default`" to "consume a `Pipeline` built
from the user-supplied agent name list with default per-step config".
The pipeline is N agents in order, each run once, no implicit retry.

**Depends on**: I.2 (lets us flip the CLI/Tauri callers in I.4 / I.5
without a stale half-state).

**Test first** (RED):
- New test file `crates/agentic-core/tests/pipeline_dynamic.rs` (or
  add cases to the existing `pipeline_toml.rs` if name-conflict
  coverage stays clean):
  - `it("PipelineSm runs a 1-agent pipeline to completion with no qa retry")`:
    build `Pipeline { steps: vec![PipelineStep { agent: "reviewer", ... }] }`.
    Drive `Start → StepPassed`. Assert state machine goes
    `Pending → Running → Completed` in one StepPassed cycle.
  - `it("PipelineSm runs a 5-agent pipeline including duplicate tdd-developer")`:
    build steps `[architect, tdd-developer, qa, tdd-developer, reviewer]`.
    All `StepPassed`. Assert each `step_index` increments to N=5.
    Asserts the SM doesn't trip on the duplicate name.
  - `it("PipelineSm: qa step failure does NOT trigger a retry into tdd-developer (post-Phase-I)")`:
    `[architect, tdd-developer, qa, reviewer]`. Drive
    `Start → StepPassed (architect) → StepPassed (tdd-developer) →
     StepFailed (qa)`. With `stop_on_failure: false` on qa (the
     existing builtin default), assert the SM advances to reviewer
     **without** rolling back step_index to tdd-developer (today it
     does). The new contract: qa-fix-loop is no longer automatic.
  - `it("PipelineSm: qa step failure on stop_on_failure: true terminates with Failed")`:
    same pipeline but flip qa's `stop_on_failure` to true. Assert
    terminal state is `Failed`.
- Existing tests in `crates/agentic-core/tests/pipeline_toml.rs` and
  `crates/agentic-core/src/pipeline/sm.rs::tests::*` that asserted
  the old qa-fix-loop semantics get **rewritten** in this same step
  (no parallel "legacy default" path). Tests that today expect
  `qa_retries` to bump on `StepFailed(qa)` flip to expecting linear
  advancement.

**Implement** (GREEN):
- Delete `qa_fix_loop_cap` from `PipelineStep` (the field) **and**
  from `PipelineSm`'s state. Delete the special-case in
  `handle_step_failed` that compares the running step's agent name
  to `"qa"` and rolls back. The SM becomes a linear N-step advancer
  with `StepFailed` resolving via `stop_on_failure` only.
- Update the TOML deserialiser tests
  (`crates/agentic-core/tests/pipeline_toml.rs`) — any test that
  parsed a TOML containing `qa_fix_loop_cap = N` either drops the
  field or the test is removed entirely (the field no longer exists).
- Update `PipelineConfig::builtin_default` in
  `crates/agentic-core/src/pipeline/config.rs` to drop the
  `qa_fix_loop_cap: Some(3)` line on tdd-developer. The builtin
  pipeline still ships as-is (4 agents, sensible defaults) so the
  TOML parser tests survive.
- Add a new free function or `PipelineConfig::from_agents(agents:
  &[String]) -> Pipeline` that builds a `Pipeline` from a string
  slice with `stop_on_failure = true` and other fields at default.
  This is what I.4 / I.5 will call.

**Refactor**:
- Update doc comments on `PipelineStep` to drop the qa-fix-loop
  references; replace with "retry policy is out of scope (v2)".
- Update `crates/agentic-core/src/pipeline/sm.rs` doc comment that
  mentions tdd-developer ↔ qa coupling.

**Files**:
- `crates/agentic-core/src/pipeline/config.rs`
- `crates/agentic-core/src/pipeline/sm.rs`
- `crates/agentic-core/tests/pipeline_dynamic.rs` (new)
- `crates/agentic-core/tests/pipeline_toml.rs` (rewrites)

**Commit**: `refactor(core): pipeline accepts arbitrary agent list, drop qa-fix-loop`

**Verification**:
```
cargo test -p agentic-core --test pipeline_dynamic
cargo test -p agentic-core --test pipeline_toml
cargo test -p agentic-core
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

---

### Step I.4: CLI `run --ticket --agents <list>` (no implicit fallback)

**Goal**: `agentic-cli run --ticket "..." --agents architect,tdd-developer,qa,reviewer`
runs the pipeline with that agent list. **Empty / missing
`--agents` errors with an actionable message** (no fallback to
"all agents alphabetically" or any other implicit default).

**Depends on**: I.3 (uses `from_agents`).

**Test first** (RED):
- Add a test to `crates/agentic-cli/src/main.rs` (or a new
  `crates/agentic-cli/tests/cli_args.rs` for cleaner separation):
  - `it("run --ticket --agents foo,bar parses to agents = [foo, bar]")`:
    use clap's `try_parse_from` to assert the resulting subcommand
    has `agents == vec!["foo".to_string(), "bar".to_string()]`.
  - `it("run --ticket without --agents errors with a clear message")`:
    `try_parse_from(&["agentic-cli", "run", "--ticket", "fix bug"])`
    asserts an error whose message contains `--agents`.
  - `it("run --ticket --agents '' errors (whitespace-only list)")`:
    parse succeeds (clap gets an empty string), but
    `cmd_run_ticket` errors with the same actionable message
    (`"--agents is required: pass --agents architect,tdd-developer
    or run agentic-cli list-agents to see what's discoverable"`).
- Update existing `cmd_run_ticket` integration tests
  (`crates/agentic-cli/tests/e2e_permissions_live.rs`,
  `crates/agentic-cli/tests/e2e_copilot_live.rs`,
  `crates/agentic-cli/tests/execute_pipeline_file_changes.rs`) to
  pass an explicit agent list. No legacy default helper.

**Implement** (GREEN):
- Add to `crates/agentic-cli/src/main.rs::Command::Run`:
  ```rust
  /// Comma-separated agent names in pipeline order. Required for
  /// --ticket; not used with --scripted.
  #[arg(long, value_delimiter = ',', requires = "ticket")]
  agents: Vec<String>,
  ```
  No `default_value` — empty `Vec` means "user didn't pass it".
- In `run_command`, the `Run { ticket: Some(_), agents, .. }` branch
  validates `agents`:
  - If `agents.is_empty()` (or every entry is whitespace-only),
    bail with the actionable message.
  - Otherwise pass the list to `cmd_run_ticket`.
- `cmd_run_ticket` signature gains `agents: Vec<String>`. Inside,
  it builds the pipeline via `PipelineConfig::from_agents(&agents)`
  (from I.3) instead of `PipelineConfig::load(...).default_pipeline()`.
- Drop the `PipelineConfig::load` call from `cmd_run_ticket` —
  Phase I no longer reads `pipeline.toml` (see Out of scope item).
  A future phase reconciles.

**Refactor**:
- If `cmd_run_ticket`'s arg list grows past 5, group into a struct
  (matches the `PipelineRunContext` pattern already in
  `ticket_run.rs`).

**Files**:
- `crates/agentic-cli/src/main.rs`
- `crates/agentic-cli/src/ticket_run.rs`
- `crates/agentic-cli/tests/e2e_permissions_live.rs`
- `crates/agentic-cli/tests/e2e_copilot_live.rs`
- `crates/agentic-cli/tests/execute_pipeline_file_changes.rs`
- `crates/agentic-cli/tests/cli_args.rs` (new, optional)

**Commit**: `feat(cli): require --agents on run --ticket; no implicit default`

**Verification**:
```
cargo test -p agentic-cli
cargo clippy -p agentic-cli --all-features --all-targets -- -D warnings
```

---

### [x] Step I.5: `start_ticket_run` IPC takes `agents: Vec<String>` (breaking)

**Goal**: The Tauri IPC signature changes to:
```rust
async fn start_ticket_run(
    bus_state, db_state,
    ticket: String,
    backend: String,
    agents: Vec<String>,
    model: Option<String>,
) -> Result<String, String>
```
Every JS caller in the Migration notes section above updates in this
same step. No legacy default fallback.

**Depends on**: I.2, I.3, I.4 (the upstream surfaces are all in their
new shape; this step is the IPC + JS sync).

**Test first** (RED):
- Update / add tests in `crates/agentic-tauri/tests/ticket_ipc.rs`:
  - `it("start_ticket_run with empty agents errors before seeding rows")`:
    invoke with `agents: vec![]`. Assert `Err` whose message
    contains `"agents list is empty"`. Assert no `runs` row was
    inserted (DB count stays 0 / stays at the pre-call value).
  - `it("start_ticket_run with a 2-agent list seeds and dispatches")`:
    write 2 valid agent files. Invoke with `agents: vec!["architect",
    "reviewer"]`. Assert the returned run_id is non-empty; assert
    the run row is seeded with the matching agents (the orchestrator
    publishes `StepStarted` per name).
  - `it("start_ticket_run rejects names that don't resolve")`:
    invoke with `agents: vec!["nope"]`. Assert `Err` with the
    pre-flight error format from I.2 (lists searched paths).
- Update the JS test surface (Migration notes lists every caller).
  Each caller passes a fixture agents array; the existing devInvoke
  mock learns the new arg. **No "default to DEFAULT_AGENTS when
  agents is undefined"** shim — TS callers all pass the array.
- New JS test
  `apps/web-ui/src/__tests__/startTicketRunAgents.test.ts`:
  - `it("invoke('start_ticket_run', ...) requires an agents array")`:
    via the dev mock, asserts the mock rejects when `agents` is
    missing. (Mirror the Rust validation in the dev path so
    integration tests catch regressions.)

**Implement** (GREEN):
- Update `start_ticket_run` in
  `crates/agentic-tauri/src/commands/ticket.rs:50` to accept
  `agents: Vec<String>`. Validate: trim whitespace per entry,
  dedup-preserving-order is **not** done (a duplicate
  `tdd-developer` is a legitimate user choice — see I.3 test).
  Empty after trim → error.
- Replace the `pipeline_config = PipelineConfig::load(...)` block
  with `PipelineConfig::from_agents(&agents)`. Drop the
  `pipeline.toml` read entirely (deferred; see Out of scope).
- Replace the hard-coded `["architect", ...]` in `pre_flight_check`
  call with the `agents` slice (the function already accepts
  `&[String]` from I.2).
- Update the Tauri capabilities / permission JSON if `start_ticket_run`'s
  arg shape requires it (it shouldn't — the permission is name-based,
  not arg-based).
- Update every JS caller (full list in Migration notes). Each caller
  passes either its own user-supplied list (chat / start-form) or
  reads from the persistence layer landing in I.6 (placeholder
  `[]` literal acceptable in this step; I.6 wires real data).

**Refactor**:
- The existing inline `BackendKind::parse(&backend)?` line moves
  ahead of the `agents` check so a malformed backend doesn't get
  masked by an empty-agents complaint.

**Files**:
- `crates/agentic-tauri/src/commands/ticket.rs`
- `crates/agentic-tauri/tests/ticket_ipc.rs`
- `apps/web-ui/src/components/ChatPane.tsx`
- `apps/web-ui/src/utils/createSpec.ts`
- `apps/web-ui/src/utils/devInvokeMock.ts`
- `apps/web-ui/src/__tests__/permissionFlow.integration.test.tsx`
- `apps/web-ui/src/__tests__/devInvokeMock.test.ts`
- `apps/web-ui/src/__tests__/app.test.tsx`
- `apps/web-ui/src/__tests__/AppPolish.test.tsx`
- `apps/web-ui/src/__tests__/ChatColumn.test.tsx`
- `apps/web-ui/src/__tests__/createSpec.test.ts`
- `apps/web-ui/src/__tests__/ChatPane.test.tsx`
- `apps/web-ui/src/__tests__/IssueColumnSpecFlow.test.tsx`
- `apps/web-ui/src/__tests__/startTicketRunAgents.test.ts` (new)

**Commit**: `feat(tauri,web): start_ticket_run takes agents list (breaking)`

**Verification**:
```
cargo test -p agentic-tauri
pnpm -F @agentic/web-ui test
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

---

### [x] Step I.6: New `list_agents` IPC + Tauri DTO (no path leakage)

**Goal**: Expose the agent inventory to the webview via a new IPC
`list_agents(backend: String) -> Result<Vec<AgentInfoDto>, String>`.
The DTO carries `name`, `description`, `source` only — no
`path`. Maps `agentic_core::AgentInfo` (which has the path
internally if we want it) to a webview-safe shape.

**Depends on**: I.1.

**Test first** (RED):
- New test `crates/agentic-tauri/tests/list_agents_ipc.rs`:
  - `it("list_agents returns Project-tagged entries for repo files")`:
    write 2 agents in `<root>/.claude/agents/`. Invoke
    `list_agents("claude-code")`. Assert 2 entries, each with
    `source: "project"`. Assert no `path` field is serialised
    (parse the result as `serde_json::Value` and assert
    `entry.get("path").is_none()`).
  - `it("list_agents returns Home-tagged entries when only ~/.claude has them")`:
    same fixture but in `<home>/.claude/agents/`. Invoke. Assert
    `source: "home"`.
  - `it("list_agents on unknown backend errors")`:
    invoke with `"scripted"`. Assert `Err` whose message lists the
    valid backends.
  - `it("list_agents returns alphabetically-sorted entries")`:
    write `reviewer.md` then `architect.md`. Assert the result is
    `[architect, reviewer]`.
- New JS test
  `apps/web-ui/src/__tests__/useDiscoverableAgents.test.ts`:
  - `it("loads agents on mount and exposes { agents, loading, error }")`:
    mock `invoke("list_agents", ...)` to return a fixture array.
    Render a wrapper component that calls the new
    `useDiscoverableAgents(backend)` hook. Assert the hook's state
    progresses `loading: true → loading: false, agents: [...]`.

**Implement** (GREEN):
- Add `crates/agentic-tauri/src/commands/agents.rs` with:
  ```rust
  #[derive(Serialize)]
  pub struct AgentInfoDto {
      pub name: String,
      pub description: String,
      pub source: String, // "project" | "home"
  }

  #[tauri::command]
  pub async fn list_agents(backend: String) -> Result<Vec<AgentInfoDto>, String>;
  ```
  Maps `agentic_core::AgentInfo { name, description, source }` to
  the DTO. Resolves `repo_root` and `home` exactly like
  `start_ticket_run` does.
- Register the command in `crates/agentic-tauri/src/main.rs`'s
  `tauri::generate_handler!` list.
- Add the permission entry in
  `crates/agentic-tauri/permissions/...` (or wherever
  `start_ticket_run` is whitelisted) — copy the same shape.
- Add a JS hook `apps/web-ui/src/hooks/useDiscoverableAgents.ts`:
  ```ts
  export function useDiscoverableAgents(backend: BackendId): {
    agents: AgentInfo[];
    loading: boolean;
    error: string | null;
    refresh: () => void;
  };
  ```
  TS type `AgentInfo` matches the Rust DTO field-for-field. Lives
  in `apps/web-ui/src/types/agent.ts` (new file).

**Refactor**: None.

**Files**:
- `crates/agentic-tauri/src/commands/agents.rs` (new)
- `crates/agentic-tauri/src/commands/mod.rs` (re-export)
- `crates/agentic-tauri/src/main.rs` (register handler)
- `crates/agentic-tauri/permissions/<file>.toml` (whitelist)
- `crates/agentic-tauri/tests/list_agents_ipc.rs` (new)
- `apps/web-ui/src/types/agent.ts` (new)
- `apps/web-ui/src/hooks/useDiscoverableAgents.ts` (new)
- `apps/web-ui/src/__tests__/useDiscoverableAgents.test.ts` (new)

**Commit**: `feat(tauri,web): list_agents IPC + useDiscoverableAgents hook`

**Verification**:
```
cargo test -p agentic-tauri --test list_agents_ipc
pnpm -F @agentic/web-ui test useDiscoverableAgents
```

---

### Step I.7: Per-project pipeline persistence (`localStorage`)

**Goal**: The web UI persists the user's chosen agent list per
workspace id under
`localStorage["agentic.pipeline.<wsId>"]` and loads it on mount
into the `pipelineAgents` state that drives `PipelineBar` +
`start_ticket_run`. New users with no saved pipeline see an empty
pipeline and must build one (no implicit fallback to a hard-coded
list).

**Depends on**: I.5 (the IPC accepts `agents`), I.6 (the picker
needs the agent inventory).

**Test first** (RED):
- New test
  `apps/web-ui/src/__tests__/pipelinePersistence.test.tsx`:
  - `it("saves pipelineAgents to localStorage on change, keyed by workspace id")`:
    render `<App />` with a fixed `wsId`. Mutate the pipeline (drag
    or picker). Assert
    `localStorage.getItem("agentic.pipeline.<wsId>")` parses to the
    new array.
  - `it("hydrates pipelineAgents from localStorage on mount")`:
    `localStorage.setItem("agentic.pipeline.<wsId>", JSON.stringify(["architect"]))`.
    Render `<App />`. Assert `data-testid="pipeline-bar"` shows
    1 card with name "architect".
  - `it("ignores corrupt JSON gracefully and starts with an empty pipeline")`:
    `localStorage.setItem("agentic.pipeline.<wsId>", "{not json}")`.
    Render. Assert pipeline is empty + no thrown error visible to
    the user. (Optionally assert a console warn.)
  - `it("isolates per-workspace: switching wsId loads a different list")`:
    seed two keys with different lists. Render with wsId A → assert
    list A. Re-render with wsId B → assert list B.
- A test that asserts **no implicit DEFAULT_AGENTS fallback**:
  - `it("first-run with no localStorage entry shows an empty pipeline (no DEFAULT_AGENTS)")`:
    clear localStorage, render `<App />`. Assert `pipelineAgents.length === 0`
    and `data-testid="pipeline-empty-state"` is visible (a small
    "Add an agent to get started" prompt — see UI delta below).

**Implement** (GREEN):
- New hook `apps/web-ui/src/hooks/usePipelinePersistence.ts`:
  ```ts
  export function usePipelinePersistence(wsId: string | null): {
    pipelineAgents: string[];
    setPipelineAgents: (next: string[]) => void;
  };
  ```
  - Reads `localStorage["agentic.pipeline.<wsId>"]` on mount /
    when `wsId` changes. Parses JSON → `string[]`. On parse error,
    returns `[]` and clears the key.
  - Writes back via `setPipelineAgents`. No debouncing (the user
    isn't going to spam mutations and the cost is one
    `JSON.stringify` per change).
  - When `wsId` is `null` (no active workspace), returns
    `{ pipelineAgents: [], setPipelineAgents: noop }`.
- Replace the `useState(DEFAULT_AGENTS)` (or equivalent) in
  `App.tsx` with `usePipelinePersistence(wsId)`. Resolve `wsId`
  via the existing run-summary / workspace surface (the
  `RunSummary` already carries `workspace_id`; if it doesn't yet,
  add a tiny `get_workspace_id()` IPC — flag this as a sub-decision
  in "Open implementation questions").
- Update the `PipelineBar` empty-state UX: when `pipelineAgents.length
  === 0`, render a single dashed-border tile with text
  "Add an agent to get started" and the existing `+ Add agent`
  affordance. Add `data-testid="pipeline-empty-state"`.
- Wire `setPipelineAgents` through `usePipelineMutation` so the
  drag/insert/remove handlers in `PipelineBar` continue to work.

**Refactor**:
- Delete `DEFAULT_AGENTS` from `apps/web-ui/src/types/run.ts` if
  no other file consumes it (Phase I retires it). Update
  `emptyRunState` and `deriveRunState` to take a required
  `agents: string[]` (callers always have one in the post-Phase-I
  world).

**Files**:
- `apps/web-ui/src/hooks/usePipelinePersistence.ts` (new)
- `apps/web-ui/src/__tests__/pipelinePersistence.test.tsx` (new)
- `apps/web-ui/src/App.tsx`
- `apps/web-ui/src/components/PipelineBar.tsx` (empty-state)
- `apps/web-ui/src/types/run.ts` (drop `DEFAULT_AGENTS`)
- `apps/web-ui/src/hooks/usePipelineFromRunState.ts` (consumes
  required `agents`)
- `apps/web-ui/src/utils/derivePipelineSeed.ts` (consumes required
  `agents`)
- Touched test files where `emptyRunState()` was called without
  args: pass an explicit list per test.

**Commit**: `feat(web): per-project pipeline persistence in localStorage`

**Verification**:
```
pnpm -F @agentic/web-ui test pipelinePersistence
pnpm -F @agentic/web-ui test
```

**Notes**: If `RunSummary` doesn't already expose `workspace_id`,
introduce a tiny `get_workspace_id()` IPC in this step — surface
the question to the user before dispatch. **Open implementation
question 1 below.**

---

### Step I.8: AgentPicker consumes `useDiscoverableAgents` (kill the hardcoded library)

**Goal**: The existing `AgentPicker` component (today seeded from
`AGENT_LIBRARY` constant or from `data.js`-derived hardcoded list)
switches to consuming `useDiscoverableAgents(backend)`. Each row
shows the agent's `description` (from frontmatter) and a small
`source` chip ("project" / "home"). Filter excludes agents already
in the current pipeline.

**Depends on**: I.6, I.7.

**Test first** (RED):
- New / updated tests in
  `apps/web-ui/src/__tests__/AgentPicker.test.tsx`:
  - `it("shows discoverable agents with their description and source chip")`:
    mock `useDiscoverableAgents` → `[{name:"architect", description:"plan",
    source:"project"}, {name:"qa", description:"verify", source:"home"}]`.
    Render `<AgentPicker open onSelect={fn} pipelineAgents={[]} />`.
    Assert `architect` row contains text `plan` and a chip
    `data-testid="agent-source-chip-architect"` with text `project`.
    Assert `qa` row's chip says `home`.
  - `it("filters out agents already in pipelineAgents")`:
    same mock, but `pipelineAgents={["architect"]}`. Assert the
    rendered list contains `qa` only.
  - `it("renders a loading state while the hook is fetching")`:
    mock the hook with `loading: true`. Assert
    `data-testid="agent-picker-loading"` is visible.
  - `it("renders an empty state when no agents are discoverable")`:
    mock `agents: []`. Assert `data-testid="agent-picker-empty"`
    is visible with copy `"No agents discovered. Run \`agentic-cli init\` or \`agentic-cli init --copilot\`."`.

**Implement** (GREEN):
- Update `apps/web-ui/src/components/AgentPicker.tsx`:
  - Drop the `AGENT_LIBRARY` import / constant.
  - Accept a `backend: BackendId` prop (or read it from the existing
    `BackendContext` if one exists — check before adding a prop).
  - Call `useDiscoverableAgents(backend)`.
  - Render `loading`, `error`, `empty` branches.
  - Each row gets a small chip element to the right of the name
    showing the source. Style: 1px border + 10px text, the
    `border-soft` class for `home`, `border` for `project`
    (subtle visual differentiation).
- Delete the `AGENT_LIBRARY` constant (typically in
  `apps/web-ui/src/data/agentLibrary.ts` or inlined; trace via
  `rg AGENT_LIBRARY apps/web-ui/src`).

**Refactor**:
- If `AgentPicker` previously imported icons / accent colors per
  agent name from a hardcoded map, switch to a deterministic hash
  (e.g., a colour-from-name helper) so unknown names render
  reasonably.

**Files**:
- `apps/web-ui/src/components/AgentPicker.tsx`
- `apps/web-ui/src/__tests__/AgentPicker.test.tsx`
- `apps/web-ui/src/data/agentLibrary.ts` (delete or empty out)
- Any caller of `AGENT_LIBRARY` (search via `rg`) updated to use
  the hook instead.

**Commit**: `feat(web): AgentPicker consumes useDiscoverableAgents`

**Verification**:
```
pnpm -F @agentic/web-ui test AgentPicker
pnpm -F @agentic/web-ui test
```

---

### Step I.9: Drop `RunStep.role` everywhere

**Goal**: Surface the audit committed to in triage outcome 12.
Search anchor: `rg -n '\\brole\\b' crates/ apps/web-ui/src`. Every
match outside `chat::ChatMessage.role` ("user"/"assistant"
conversational role, unrelated to pipelines) collapses onto
`{ agent: String, index: u32 }`.

**Depends on**: I.5 (the IPC has stabilised on the new agent-list
shape).

**Test first** (RED):
- Audit step. The agent runs `rg -n '\\brole\\b' crates/ apps/web-ui/src`,
  documents every hit in the commit body, and:
  - Adds an assertion test in
    `crates/agentic-tauri/tests/ticket_ipc.rs::ipc_dto_shape`:
    `it("RunStep DTO has agent + index, no role")`: invoke
    `start_ticket_run`, capture the first `StepStarted` envelope,
    assert `serde_json::to_value(&step_dto)` does **not** contain
    a `"role"` key.
  - Adds a TS test in
    `apps/web-ui/src/__tests__/runState.test.ts`:
    `it("StepInfo has no role field")`: assert the type definition
    in `types/run.ts` does not export a `role` member (compile-time
    check via `// @ts-expect-error` against an attempt to read
    `step.role`).
  - For every `role` field found that maps to pipeline-step semantics
    (not conversational chat), the agent flips it to `agent` (if
    not already present) and removes the `role` field. Both the
    field and any serde / ts mirroring updates in the same step.

**Implement** (GREEN):
- Drive by the audit. Likely zero work in
  `crates/agentic-core/src/db/steps.rs` (already `agent_name`).
- Likely target areas based on the codebase shape:
  - Any `RunStep` / `StepRow` DTO in `crates/agentic-tauri` (none
    found at audit time, but verify post-refactor PRs haven't added
    one).
  - `apps/web-ui/src/types/run.ts::StepInfo` (already only `agent`).
  - `crates/agentic-tui/src/run.rs::StepRow` (already only
    `agent`).
- The most likely real hits (if any) live in the events-translation
  layer or the IPC-derived TS types. The audit drives the diff.

**Refactor**:
- Update doc comments anywhere the word "role" was used loosely to
  mean "the agent in this pipeline step". Replace with "agent".

**Files**:
- Whatever the audit surfaces. List in the commit body.
- `crates/agentic-tauri/tests/ticket_ipc.rs` (DTO shape assertion)
- `apps/web-ui/src/__tests__/runState.test.ts` (type assertion)

**Commit**: `refactor: drop RunStep.role; agent + index is the only step identity`

**Verification**:
```
cargo test --workspace
pnpm -F @agentic/web-ui test
rg -n '\brole\b' crates/ apps/web-ui/src | rg -v 'chat::ChatMessage|chat\\.rs|\\.role.*user|\\.role.*assistant|conversational'
# expect zero matches outside the chat conversational role
```

---

### Step I.10: Integration smoke — end-to-end 2-agent pipeline

**Goal**: One last integration step that exercises every Phase I
contract together: list discoverable agents → pick a 2-agent
subset → persist the choice → run the pipeline → verify the run
publishes exactly 2 `StepStarted` envelopes in order.

**Depends on**: I.1 through I.9.

**Test first** (RED):
- New integration test
  `apps/web-ui/src/__tests__/dynamicPipeline.integration.test.tsx`:
  - `it("end-to-end: list → pick 2 → persist → run → 2 StepStarted events")`:
    1. Mock `invoke("list_agents", ...)` to return
       `[{name:"architect",...}, {name:"reviewer",...}, {name:"qa",...}]`.
    2. Render `<App />` with empty localStorage.
    3. Open the AgentPicker, click `architect` then `reviewer`
       (in that order).
    4. Assert `pipelineAgents` is `["architect", "reviewer"]` and
       `localStorage["agentic.pipeline.<wsId>"]` matches.
    5. Click "Run pipeline".
    6. Assert `invoke` was called with
       `start_ticket_run, { agents: ["architect", "reviewer"], ... }`.
    7. Drive 2 `StepStarted` envelopes through the dev mock and
       assert both pipeline cards advance through `running` →
       `passed` per the events.
- Optional: a Rust-side end-to-end smoke in
  `crates/agentic-cli/tests/cli_dynamic_pipeline.rs` that runs
  `agentic-cli run --ticket "x" --agents architect` against a
  scaffolded fixture and asserts a 1-step `RunComplete` envelope
  (use the existing `e2e_permissions_live.rs` scaffolding pattern).

**Implement** (GREEN): No new production code expected. This step
is the canary that I.1-I.9 land coherently. If it fails, the
diagnosis points at the seam between two earlier steps; loop back
to that step's RED+GREEN with the failing assertion as the new
contract.

**Refactor**: None.

**Files**:
- `apps/web-ui/src/__tests__/dynamicPipeline.integration.test.tsx` (new)
- `crates/agentic-cli/tests/cli_dynamic_pipeline.rs` (new, optional)

**Commit**: `test: end-to-end dynamic pipeline integration smoke`

**Verification**:
```
pnpm -F @agentic/web-ui test dynamicPipeline.integration
cargo test -p agentic-cli --test cli_dynamic_pipeline
cargo test --workspace --all-features
pnpm -F @agentic/web-ui test
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

### Phase I status checklist

- [x] I.1 list discoverable agents (core, with `source` tag)
- [x] I.2 pre-flight takes user-supplied agents list
- [ ] I.3 execute_pipeline accepts arbitrary list, drop qa-fix-loop
- [ ] I.4 CLI `--agents` flag (no implicit fallback)
- [ ] I.5 `start_ticket_run` IPC adds `agents` (breaking)
- [x] I.6 `list_agents` IPC + `useDiscoverableAgents` hook
- [x] I.7 per-project pipeline persistence in localStorage
- [x] I.8 AgentPicker consumes `useDiscoverableAgents`
- [ ] I.9 drop `RunStep.role` everywhere
- [ ] I.10 end-to-end dynamic pipeline integration smoke

### Phase I open implementation questions (raised mid-write — triage before I.1 dispatch)

1. **`wsId` source for the localStorage key (Step I.7)**: the
   `usePipelinePersistence` hook needs a stable workspace id. The
   Rust side has `agentic_cli::ticket_run::stable_workspace_id`,
   but it's not exposed over IPC today. Options:
   a. Add a thin `get_workspace_id() -> String` IPC that hashes
      the current `cwd` (or the resolved `AGENTIC_WORKSPACE_ROOT`)
      via the existing helper. Pro: one source of truth. Con:
      one more IPC call on mount.
   b. Stuff `workspace_id` into the existing `RunSummary` shape
      (it already has `workspace_id` server-side per
      `crates/agentic-core/src/db/runs.rs`). The webview reads it
      from the most recent `list_runs` result, falls back to a
      sentinel `"default"` when no runs yet. Pro: zero new IPC.
      Con: empty workspace pre-first-run uses the sentinel; a
      later run shifts the key.
   c. Use a UI-supplied workspace nickname (the user types it).
      Skip the `wsId` rabbit hole entirely. Pro: zero IPC. Con:
      user friction on first use.
   **Recommendation**: option (a), one `get_workspace_id` IPC,
   confirmed before I.7 dispatches. Ranked second: option (b) —
   acceptable if (a) feels heavy.

2. **Where `AgentPicker` reads `backend` (Step I.8)**: today's
   `BackendContext` (if it exists; check via `rg
   BackendContext apps/web-ui/src`) may already make `backend`
   ambient. If not, the picker takes a new `backend` prop and
   `App.tsx` threads it down. Confirm before I.8.

3. **`list_agents` permissions / capabilities entry (Step I.6)**:
   the Tauri app may use a permission whitelist (`capabilities/`
   directory). If so, the new `list_agents` command needs an
   entry in the same place `start_ticket_run` is whitelisted.
   The agent verifies pre-implementation; if no whitelist is in
   use, this is a no-op.
