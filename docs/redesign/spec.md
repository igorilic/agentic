# Agentic — UI Redesign

Source: design hand-off at `/Users/igorilic/open-source/design_handoff_agentic_pipeline/`
Generated: 2026-04-29
Status: Ready for implementation
Scope: visual + structural redesign across three surfaces (web, Tauri, TUI).
The product spec (`/spec.md`, v0.1) is the **product** contract; this is
the **UI** contract layered on top.

---

## 1. Overview

### Goal

Replace the current ad-hoc UI on each of the three Agentic surfaces with
the Catalyst-derived design system delivered in the hand-off bundle.
Every surface keeps its current job (drive a single pipeline run; show
the structured event stream; let the user triage findings; talk to the
backend), but reorganizes how it is presented.

### Surfaces

| Surface | Path | Stack | Outcome |
|---|---|---|---|
| Web app | `apps/web-ui/` | React 18 · TS · Vite · Tailwind 3 · Vitest · RTL | New 132 px top chrome (header + pipeline bar) + 3-column workspace (chat \| activity \| issue), light/dark theme |
| Tauri desktop | `crates/agentic-tauri/` (wraps `apps/web-ui`) | Tauri 2.x · macOS native chrome | Same as web app but **dense layout** (right column 280 px), traffic-light chrome from OS |
| Terminal TUI | `crates/agentic-tui/` | Rust 2024 · ratatui · crossterm | xterm-style title bar, ASCII pipeline strip, tab bar (`① logs / ② chat / ③ issue`), inline permission card, NORMAL/INSERT/COMMAND mode line, `?` help overlay |

### Non-goals

- IPC contract is unchanged. `start_ticket_run`, `cancel_run`,
  `mention_agent`, `triage_finding`, `list_runs`, the `agentic://event`
  envelope stream, every event type in `agentic-core::events` —
  **untouched**.
- No new backend behavior. The redesign consumes the existing event
  stream and `RunSummary` shape; it does not add fields or invent new
  event types.
- No new agents or pipeline configuration UI. The `+ Add agent` /
  `agent picker` UI **renders** but is wired to a no-op for now (a
  future follow-up will surface real `pipeline.toml` editing).
- No real avatar API, no real GitHub/Jira issue fetch. The "issue
  ticket" panel is populated from the active run's `ticket_label` /
  `ticket_url` (already on `RunSummary`) and a placeholder body until
  the backend exposes a richer ticket DTO.
- No new dependencies for backend work. UI-only deps (font, optional
  drag-reorder lib, headless UI primitives) are flagged in §6.

### Migration strategy

**Swap-in-place.** No feature flag.

The existing Tailwind 3 setup, Vitest suite, RTL conventions, and
`data-testid` selectors stay. Each step lands a new component (or
restyles an existing one) and updates its tests in the same commit. The
old component is deleted in the same step it is replaced. This keeps the
test suite green at every commit boundary.

The two narrow "compat" rules:

1. Existing `data-testid` values stay on the **same DOM element** they
   currently mark (e.g. `chat-pane`, `chat-form`, `chat-input`,
   `chat-send`, `event-list`, `findings-table`, `past-runs-pane`,
   `start-ticket-form`, `triage-fix-{id}`, `cockpit-stepper` once it
   moves to the pipeline bar). Selectors used by the broader
   integration tests (`apps/web-ui/src/__tests__/*.test.tsx`) do not
   change. New testids are added; none are removed without renaming
   the matching assertion in the same step.
2. The shape of the props passed into top-level components
   (`useTauriEvents`, `useFindings`, `useChat`) does not change. The
   redesign rewires *consumption* of those hooks; it does not rename
   their inputs/outputs.

For TUI, the snapshot tests in `crates/agentic-tui/tests/` get
regenerated as each redesign step lands — the `TestBackend` outputs are
visual contracts and must change.

---

## 2. Design source of truth

The hand-off lives at:
`/Users/igorilic/open-source/design_handoff_agentic_pipeline/`

| File | Authority |
|---|---|
| `README.md` | **Normative** — pixel values, color tokens, typography, interactions, state shape. |
| `colors_and_type.css` | Source of truth for design-token values (zinc OKLCH scale, semantic vars, radii, shadows, type scale, Inter font). |
| `data.js` | Reference fixture shapes for run / log / chat / permissions / action items. We do **not** copy these into the production code — we adapt our existing `EventEnvelope` / `RunSummary` types to render what they show. |
| `webapp.jsx`, `pipeline-bar.jsx`, `panels.jsx` | Demo prototypes — informational only; production rebuilds them as React components in our own conventions (Tailwind classes, no inline styles, real hooks). |
| `tauri-frame.jsx`, `tui-view.jsx`, `agents.jsx` | Same — demo only, structural reference. |
| `Agent Pipeline UI.html` | Canvas with all artboards. Useful for "what does the dark-mode pipeline bar look like?" cross-checks. |

Where the README and the JSX disagree, the README wins.

---

## 3. Surface A: Web app

The web app is a single full-viewport flex column:

```
┌────────────────────────────────────────────────────┐  48 px  Header bar
├────────────────────────────────────────────────────┤  84 px  Pipeline bar
│                                                    │
│   Chat (1fr)   │   Activity (1fr)   │   Issue       │  flex 1 Workspace (3 cols)
│                │                    │   (340/280px) │
└────────────────────────────────────────────────────┘
```

### 3.1 Header bar (48 px)

**Visual contract**

- Height 48 px, border-bottom `--border-soft` (`rgb(0 0 0 / 0.05)`), bg `--bg-surface`.
- 18 px horizontal padding, 14 px gap between groups.
- **Left group**: 26 × 26 rounded-square (`radius 6`) tile, bg `#18181b`, white inline-SVG diamond glyph; brand "Agentic" (Inter / 14 / 600); slug `/ AGT-204` (Inter / 11 / `--fg-subtle`).
- **Right group**: run-state badge → settings icon → theme toggle → 28 × 28 round avatar.

**Run-state badge variants** — three exclusive states, derived from
`deriveRunState(events).overall` plus the pinned `activeRunId`:

- `idle` (no active run): primary "Run pipeline" button, bg `#18181b`, white text, 12 / 600. Click opens `StartRunFormDialog` (replaces today's inline `StartRunForm`).
- `running`: pill bg `rgb(219 234 254)`, fg `rgb(29 78 216)`, 6 × 6 pulsing dot, label `Pipeline running · MM:SS`. Stop button alongside (calls `cancel_run` IPC, same as today).
- `completed`: pill bg `rgb(220 252 231)`, fg `rgb(21 128 61)`, static dot, label `Completed · MM:SS`. Re-run button alongside.

The 28 × 28 avatar is a placeholder div (initials, bg `--zinc-200`) — no real avatar API at this stage.

**Component shape**

```tsx
<HeaderBar
  brand="Agentic"
  ticketSlug={activeTicket?.label ?? null}      // e.g. "AGT-204"; null when idle
  runState={runStateBadge}                       // "idle" | "running" | "completed"
  elapsedMs={runElapsedMs}                       // null when idle
  theme={theme}                                  // "light" | "dark"
  onThemeToggle={...}
  onOpenSettings={...}                           // opens existing SettingsPane in modal
  onRunPipeline={...}                            // opens StartRunFormDialog
  onStopRun={...}                                // calls cancel_run
  onRerun={...}                                  // re-opens StartRunFormDialog with same ticket prefilled
/>
```

**Behavior**

- Theme toggle persists to `localStorage["agentic.theme"]` and toggles `data-theme` on `<html>`.
- Settings icon opens a **tabbed Settings modal** (see §3.9) — at minimum a
  "General" tab (the existing `SettingsPane` content) and a "History" tab
  (the existing `PastRunsPane`). The header bar **does not** carry a
  separate "History" button; past runs are reachable only via the Settings
  modal's History tab.
- Run/Stop/Re-run buttons fire the same IPC calls today's UI does — no new backend surface.

**Accessibility**

- Buttons have visible labels or `aria-label`.
- Theme toggle reflects state via `aria-pressed`.
- Run-state badge has `role="status"` and `aria-live="polite"` so screen readers announce transitions.

### 3.2 Pipeline bar (84 px)

**Visual contract**

- Height 84 px, bg `--bg-surface`, border-bottom `--border-soft`.
- Horizontal flex row of agent cards joined by chevron connectors.
- Each card is 44 × 44 avatar tile + name (13 / 600) + status label (10 px uppercase, `--fg-muted`, letter-spacing `0.05em`).
- Card border 1 px `--border` (radius 10). Active card overlay: 1 px ring of agent accent, soft tinted bg `rgba(<accent>, 0.06)`.
- Status ring colors: `done` `#10b981`, `active` `#f59e0b` (with pulse), `queued` `#e4e4e7`, `failed` `#ef4444`.
- Connector: horizontal 1 px line `#d4d4d8` with chevron arrow. Active hand-off uses an animated dashed line.
- Insert affordance: 16 × 16 `+` chip in each gap (opacity 0 → 1 on hover); end cap is a dashed-border `+ Add agent` button.

**Component shape**

```tsx
<PipelineBar
  agents={pipelineAgents}                        // string[] — currently DEFAULT_AGENTS
  activeIndex={activeStepIndex}                  // -1 when idle/completed
  statuses={statusByAgent}                       // Record<agentName, AgentStatus>
  onReorder={...}                                // (from, to) => void
  onInsert={...}                                 // (atIndex, agentId) => void; no-op MVP
  onRemove={...}                                 // (atIndex) => void; no-op MVP
  onSkip={...}                                   // (atIndex) => void; no-op MVP
/>
```

`AgentStatus` derived from existing `StepStatus` (`pending → queued`, `running → active`, `passed → done`, `failed → failed`, `needs_triage → done` with warning glyph, `skipped → skipped`).

**Behavior**

- Card click expands a configure popover (out of scope; renders an empty placeholder modal that closes on outside click).
- Card kebab (⋯) menu: Remove / Skip / Configure — all no-op for MVP, gated behind `data-testid` so future tests can drive them.
- Drag-reorder: HTML5 drag-and-drop only (no library). Drop target is the gap between cards; show 2 px vertical accent bar at drop position.
- Insert chip / `+ Add agent` end cap: opens `AgentPicker` popover anchored to the chip, 320 px wide. Picker is a search-filtered list of `AGENT_LIBRARY` (12 agents from `agentic-core` config, hardcoded fallback to the 12 from `data.js` if the core list isn't yet exposed).
- ESC / outside click dismisses the picker.

**Accessibility**

- Card has `role="button"`, `aria-label="<agent> — <status>"`.
- Drop targets are reachable via keyboard (Tab to card, then arrow keys to move; future polish — MVP can skip if it complicates the step).
- `+ Add agent` button has `aria-haspopup="dialog"` and the popover has `role="dialog"`.

### 3.3 Agent picker (popover)

- 320 px wide, white surface (popover bg in dark mode), radius 12, shadow `--shadow-lg`, 1 px border `rgb(0 0 0 / 0.08)`.
- Search input (Inter / 13) + scroll list of agents not already in the pipeline.
- Each row: 32 × 32 avatar tile (icon + accent bg per agent) + name (13 / 600) + 1-line description (11 px, `--fg-muted`).
- Hover row tint `rgba(0, 0, 0, 0.04)`.

### 3.4 Workspace — Column 1: Chat

**Visual contract**

- `1fr` width on default; 1 px right border `--border-soft`.
- Header (12–14 px padding): "Chat with pipeline" (13 / 600) + active-agent chip ("Developer is responding") on the right.
- Message list: 16 px gap, vertical scroll. Per message:
  - User: avatar + "Erica" + timestamp + body. Body 14 / 1.5, `--fg`.
  - Agent: 28 × 28 round agent avatar with accent tint + agent name (13 / 600) + timestamp + body. Body in tinted bubble (`rgba(<accent>, 0.04)` bg + 3 px left accent border of agent color).
  - System: centered, 11 px, `--fg-subtle`, no bubble. e.g. `── Architect handed off to Developer ──`.
  - Slash commands and `@mentions` rendered as highlighted tokens: 2 px radius, light-yellow bg `rgba(253, 230, 138, 0.4)`.
- Composer (sticky bottom):
  - Quick-pick chip row: `Plan` `Brainstorm` `Develop` `Spec` (1 px border, 6 px radius, 12 px text). Click inserts the slash and focuses the textarea.
  - Textarea: 1 px border `rgb(0 0 0 / 0.1)`, radius 12, padding 10 / 14, focus ring 2 px `#18181b` offset 2 px.
  - Right-aligned 36 × 36 black square Send button with white arrow icon.
  - **Slash popover**: typing `/` opens a 280 px popover above the textarea showing matching commands; arrow keys + Enter select; Esc dismisses. Uses `SLASH_COMMAND_LIBRARY` from `slash/library.ts` for prefix matching; `parseSlashCommand` from `slash/parser.ts` is the syntax parser invoked on submission, not by the popover.
  - **Mention popover**: typing `@` opens a 240 px popover with the agent picker shape. Reuses `parseMention` from `mention/parser.ts`.

**Component shape (rewires existing `ChatPane`)**

```tsx
<ChatColumn
  messages={chatMessages}                   // existing useChat hook output
  systemMessages={systemMessages}           // existing local state
  mentionMessages={mentionMessages}         // existing useMentionEvents hook
  activeAgent={activeAgentName}             // null when idle
  activeRunId={activeRunId}                 // for ActiveRunIndicator integration
  activeRunStartedAtMs={...}
  onSend={...}                              // existing slashServices wiring
  onCancelActiveRun={...}
/>
```

**Behavior**

- All slash command parsing + dispatch logic stays in `slash/parser.ts` and `slash/dispatcher.ts`.
- All mention parsing stays in `mention/parser.ts`.
- Cmd/Ctrl+Enter sends; Enter alone inserts a newline (today's `Enter` sends — this is a behavior change, called out in step W.3.4).
- Quick-pick chip → inserts `/<command> ` and focuses the textarea (uses existing `setDraft` state).
- Active-run pill (today's `ActiveRunIndicator`) collapses into the column header chip — the standalone strip is removed (its content now lives in the header bar's run-state badge).

### 3.5 Workspace — Column 2: Activity log

**Visual contract**

- `1fr` width; 1 px right border `--border-soft`.
- Header: "Activity" (13 / 600) + tab strip → `All` · `Tool calls` · `Permissions` · `Errors`. Active tab: 2 px bottom border `#18181b`. Inactive: `--fg-muted`.
- Filter chip count: small 16 × 16 pill next to each tab showing the count.
- Log entries — vertical list, 8 px gap. Three kinds:
  - **Info / status**: monospace 12 px, `[HH:MM:SS]` + agent (in agent color) + message.
  - **Tool call card**: 1 px border, radius 8, padding 10 / 12. Header row: agent + tool name + result chip (`OK` green / `error` red). Optional collapsible body for stdout/stderr (max 200 px scroll).
  - **Permission card** — see §3.7.
- Active streaming entry has subtle left-border animation matching its agent color.

**Component shape (replaces today's `EventList`)**

```tsx
<ActivityColumn
  events={events}                           // existing useTauriEvents output
  filter={"all" | "tool" | "perm" | "error"}
  onFilterChange={...}
  pendingPermissions={...}                  // derived from events; future-wired
  onPermissionDecision={(permId, decision) => void}
/>
```

**Mapping** existing event types → log row variants:

| Existing event | Variant |
|---|---|
| `RunStarted` / `RunComplete` / `StepStarted` / `StepComplete` | info / status |
| `ToolCall` (today's `tool` shape) | tool-call card |
| `Finding` | inline finding row in **All / Errors** filter (not a separate column anymore) |
| `PermissionRequest` (future) | permission card |
| `TextDelta` | filtered out (chat only, not activity) |

`Finding`s previously rendered in the standalone `FindingsTable` only show up in "Action items" inside Column 3 once the run is `completed`.

### 3.6 Workspace — Column 3: Issue ticket

**Visual contract**

- 340 px wide (web) / 280 px wide (Tauri dense). Min-width 280 px.
- Padding 18 px, 14 px section gap, vertical scroll.
- Header strip: issue ID (`AGT-204`, 11 / 700, `--fg-subtle`), title (15 / 700).
- Labels row: chips, 1 px border, radius 4, 11 px, color-coded.
- Description block: 13 / 1.5 prose paragraphs.
- Acceptance criteria: monospace `[ ]` / `[x]` checklist (rendered via `<ul>` with `role="list"`, monospace marker, no native bullets).
- **Action items** (only when `runState === "completed"`):
  - Heading "Action items" (12 px uppercase / 700, `--fg-muted`).
  - Per item: status icon (`✓` done / `⚠` warning / `↗` follow-up), title (13 / 600), 1-line description (12 px, `--fg-muted`).
  - "Create spec" primary button at bottom → opens **Spec dialog** (§3.8).

**Component shape (new)**

```tsx
<IssueColumn
  ticket={activeTicket}                     // { id, title, labels[], body[], acceptance[] } — derived from RunSummary.ticket_label / ticket_url + a placeholder body
  runState={runState}
  actionItems={derivedActionItems}          // from completed-run findings
  onCreateSpec={...}                        // opens SpecDialog
/>
```

**Data sources at MVP**

- `ticket.id` and a placeholder title come from `RunSummary.ticket_label`.
- `ticket.body` is `["No description available — ticket source integration ships in a future phase."]` until the backend adds a `get_ticket` IPC. This is acceptable and called out in §7.
- `acceptance` is empty until the same backend work lands.
- `actionItems` is built from `findings` filtered to `triage === null` (i.e. user hasn't triaged yet) once `runState === "completed"`.

### 3.7 Permission request card (shared with TUI conceptually)

**Visual contract** (web variant — see §4 for TUI variant)

- 1 px border `#fca5a5` + 3 px left accent same color.
- Bg `rgba(252, 165, 165, 0.06)`, radius 10, padding 12 / 14.
- Header row: ⚠ red icon + "{agent} requests permission" (13 / 600) + risk pill (`HIGH RISK` red / `MEDIUM` amber / `LOW` zinc) right-aligned.
- Command preview block: bg `#000`, monospace 12, fg `#a7f3d0`, prefix `$ `, padding 8 / 12, radius 4.
- Reason + scope: 11 px `--fg-muted`; scope rendered in pill (`shell.destructive`, `fs.write`, etc.).
- Action row: `Allow once` (primary) · `Allow for session` (ghost) · `Deny` (red text, ghost).

**Component shape (new)**

```tsx
<PermissionCard
  permission={pending}                      // { id, agent, tool, arg, scope, risk, reason, t }
  onDecision={(decision: "once" | "session" | "deny") => void}
/>
```

**Behavior**

- Decision fires a callback (no IPC at MVP — the backend integration ships when permission events land in the event stream). The callback collapses the card and appends a synthetic info log entry.
- A `PermissionRequest` event type does not exist yet in `agentic-core::events`. The redesign **renders** the card from a deterministic fixture in non-prod builds and from a new optional event variant when the backend ships it. **No backend changes in this redesign.**

### 3.8 Spec dialog (modal)

- Centered, 560 px wide × auto, max 80vh, radius 14, shadow `--shadow-lg + 0 25px 50px -12px rgb(0 0 0 / 0.4)`.
- Backdrop `rgba(0, 0, 0, 0.4)`, click to dismiss.
- Header: doc icon + "New spec" + helper "Spec will be handed to the Architect." (11 px, `--fg-muted`).
- Body: title input + 8-row markdown textarea.
- Footer: `Cancel` (ghost) + `Create & run` (primary; disabled when title empty).

**Behavior**

- `Create & run` calls `start_ticket_run` IPC with the title as the ticket label and the body as the synthesized prompt — no new IPC.

### 3.9 Settings modal (tabbed)

The header's settings icon opens a single tabbed modal that owns
configuration *and* run history. There is no separate History button on
the header.

**Visual contract**

- Centered modal, 720 px wide × auto, max 80 vh, radius 14, shadow
  `--shadow-modal`. Backdrop `rgba(0, 0, 0, 0.4)`, click to dismiss.
- Header row: title `Settings`, close (`×`) on the right.
- Tab strip directly under the header: `General` · `History`. Active tab
  has a 2 px bottom border in `--fg`; inactive tabs in `--fg-muted`.
- Tab body fills the rest of the modal with internal scroll if content
  overflows.

**Components**

- `SettingsModal` — shell component owning `open`, `activeTab` state and
  the backdrop / focus-trap concerns. Reuses the same `Modal` primitive
  that hosts `SpecDialog`.
- `GeneralTab` — wraps the existing `SettingsPane` content unchanged
  (preferences, backend selection, theme, etc.).
- `HistoryTab` — wraps the existing `PastRunsPane` unchanged. The
  `data-testid="past-runs-pane"` selector stays on its outer element.

**Component shape**

```tsx
<SettingsModal
  open={settingsOpen}
  initialTab="general"            // "general" | "history"
  onClose={() => setSettingsOpen(false)}
/>
```

**Behavior**

- Tab switch updates `activeTab` state; URL is *not* mutated (the modal
  is transient).
- Esc, backdrop click, and `×` button all close the modal.
- Focus moves to the first focusable element in the active tab when it
  changes.

**Accessibility**

- Modal has `role="dialog"`, `aria-labelledby` pointing at the title.
- Tab strip uses `role="tablist"` with each tab as `role="tab"` + the
  active tab `aria-selected="true"`. Tab panels have `role="tabpanel"`
  and `aria-labelledby` referencing their tab.

---

## 4. Surface B: Terminal TUI

### 4.1 Palette

Constants in `crates/agentic-tui/src/theme.rs` (new module):

```rust
pub const BG: Color       = Color::Rgb(0x0d, 0x0e, 0x10);
pub const FG: Color       = Color::Rgb(0xe6, 0xe6, 0xe6);
pub const DIM: Color      = Color::Rgb(0x7d, 0x7d, 0x8a);
pub const BORDER: Color   = Color::Rgb(0x2a, 0x2b, 0x30);
pub const ACCENT: Color   = Color::Rgb(0x5e, 0xea, 0xd4); // cyan
pub const BLUE: Color     = Color::Rgb(0x7d, 0xd3, 0xfc);
pub const YELLOW: Color   = Color::Rgb(0xfd, 0xe6, 0x8a);
pub const GREEN: Color    = Color::Rgb(0xa7, 0xf3, 0xd0);
pub const RED: Color      = Color::Rgb(0xfc, 0xa5, 0xa5);
pub const PURPLE: Color   = Color::Rgb(0xc4, 0xb5, 0xfd);
pub const HEADER_BG: Color= Color::Rgb(0x16, 0x17, 0x1b);
```

Dark only — no light theme for TUI.

### 4.2 Title bar (28 px / row 0)

- Fixed-height 1-row band with HEADER_BG style.
- Mac-style traffic lights at left: three `●` glyphs in red / amber / green.
  These are **decorative on every platform** (macOS, Linux, Windows) — they
  are a visual flourish, not OS chrome, and never wire to window controls.
  No `cfg(target_os = "...")` gating; render the same three glyphs
  everywhere.
- Centered text `user@host — agentic — {cols}×{rows}` in DIM.
  `user@host` from `whoami` + `hostname()` (cached at start). `cols` / `rows` from the live frame area.

### 4.3 Issue header (1 row + 1 row of padding)

- `▰ agentic │ AGT-204 <issue title>` aligned left, `● running 02:34` aligned right.
- `▰ agentic` in ACCENT bold; `│` in DIM; `AGT-204` in FG; title in DIM with ellipsis on overflow.
- Run state pill at right with pulsing `●` (toggles every render frame to fake the pulse).

### 4.4 ASCII pipeline bar (4 rows)

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ ✓ 01 Plan   │──▶ │ ● 02 Dev    │──▶ │ ○ 03 QA     │
│ DONE        │    │ ACTIVE      │    │ QUEUED      │
└─────────────┘    └─────────────┘    └─────────────┘
[a]dd  [r]eorder  [d]rop
```

- Each card minimum width 13 cols. Active card uses YELLOW border + tinted bg
  (achievable in ratatui via `Style::default().bg(Color::Rgb(...)).fg(...)`).
- Status glyphs: done = `✓` GREEN, active = `●` YELLOW, queued = `○` DIM, failed = `✗` RED.
- Connector `──▶` in BORDER.
- Hint footer in DIM 1 row below the cards.

### 4.5 Tab bar (1 row)

- `① logs   ② chat   ③ issue` left-aligned; `? for help` right-aligned in DIM.
- Active tab: 2 px bottom border in ACCENT (rendered as `─` row directly under it), brighter FG.

### 4.6 Body pane (flex)

Only one of three panes visible at a time, switched by `1` / `2` / `3` keys.

- **Logs**: rows of `HH:MM:SS  agent      LEVEL  message`. Columns 8 / 16 / 8 / rest. Tool calls render `tool_name("arg") → result` with tool_name BLUE and result DIM. Tab also accepts the existing `Finding` events as `WARN` rows.
- **Chat**: message blocks with `── system ──` dividers; user/agent name on its own line in ACCENT/GREEN; body indented 2 cols. Slash commands and `@mentions` highlighted with `Style::bg(rgba(253,230,138,0.1))` (approximated as a single-cell rgb in 256-color terminals).
- **Issue**: issue ID in ACCENT bold; title bold; label chips with 1 px border (rendered as `▏…▕`); description paragraphs; acceptance checklist as `[ ]`.

### 4.7 Permission card (inline in logs)

```
┌─ ⚠ PERM  developer requests permission                HIGH RISK ─┐
│ $ rm -rf node_modules                                            │
│ Cleaning stale build artifacts (scope: shell.destructive)        │
│ [y] allow once    [s] session    [n] deny                        │
└──────────────────────────────────────────────────────────────────┘
```

- 1 px RED border + 3 px RED left accent (single column of `┃`).
- Command on `Color::Black` bg with GREEN fg, prefix `$ ` in DIM.
- Hotkeys: `[y]` GREEN, `[s]` GREEN, `[n]` RED, all bold; labels in FG.

### 4.8 Status / command line (bottom row)

- Single row, HEADER_BG.
- In NORMAL: hint text in DIM (`Press : for command · ? for help · 1/2/3 to switch panes · y/s/n on permission`). Mode indicator at right: `NORMAL` in DIM.
- In COMMAND: `:` in ACCENT bold + cursor + buffer; placeholder hint when buffer empty (`add <agent>  ·  rm <agent>  ·  help  ·  q`); mode indicator `COMMAND` in YELLOW.
- In INSERT (chat compose): mode indicator `INSERT` in GREEN.
- Flash messages (`✓ once: shell "rm -rf"`) override the hint for ~1.6 s in ACCENT color.

### 4.9 Help overlay (toggled by `?`)

- Centered modal, ACCENT border, HEADER_BG fill.
- `┌── KEYBINDINGS ──┐` + table of key → description.
- Esc or any click dismisses.

### 4.10 Component shape — TUI

The existing `Pane` enum (`Cockpit`, `Chat`) becomes `Pane::{Logs, Chat, Issue}`. The
`AppState` struct gains:

```rust
pub struct AppState {
    pub pane: Pane,                    // Logs | Chat | Issue (replaces Cockpit)
    pub mode: Mode,                    // Normal | Insert | Command — Insert is new
    pub pipeline: Vec<AgentInstance>,  // ordered, with status (mirrors web State)
    pub active_idx: i32,               // -1 when idle/completed
    pub log: Vec<LogEntry>,            // existing run.rs feeds this
    pub chat: Vec<ChatMessage>,        // new — fed from a future `chat_envelope` channel
    pub pending_perms: Vec<PermissionRequest>,
    pub flash: Option<Flash>,          // (text, expires_at)
    pub help_open: bool,
    pub findings: FindingsState,       // unchanged
    // existing: focus, cockpit_ratio, last_status, current_diff, diff_scroll_offset
}
```

The redesign **adds** fields; it does not break the existing `apply_envelope` /
`handle_key` contract. New keys (`1`, `2`, `3`, `i`, `y`, `s`, `n`, `?`) are
handled in addition to the existing `:`, `Tab`, `[`, `]`, `j`, `k`, `f`, `t`, `i`.
Conflict on `i`: today `i` triages "ignore"; in the new design `i` enters
INSERT mode. We resolve by **scoping** triage shortcuts to the issue/findings
pane only — `i` in chat or logs panes enters INSERT mode; in issue pane it
triages the selected finding.

### 4.11 Behavior changes

| Key | Action |
|---|---|
| `1` / `2` / `3` | Switch to logs / chat / issue pane |
| `:` | COMMAND mode (existing) |
| `i` | INSERT mode in chat/logs; triage selected = ignore in issue pane |
| `y` / `s` / `n` | Resolve topmost pending permission (allow once / session / deny) |
| `?` | Toggle help overlay |
| `Esc` | Close overlay; exit INSERT or COMMAND |
| `:add <agent>` | Append agent to pipeline (no-op for backend; updates state for visual demo) |
| `:rm <agent>` | Remove agent from pipeline (same caveat) |
| `:help` | Open help overlay |
| `:q` | Quit (existing) |

---

## 5. Surface C: Tauri desktop

The Tauri shell loads `apps/web-ui` unchanged. Deltas from §3:

### 5.1 Dense layout

- The web app accepts a `dense: boolean` prop on its root, derived at runtime from `import.meta.env.TAURI === "1"` or the presence of `window.__TAURI_INTERNALS__`.
- When dense, the workspace grid template flips from `1fr 1fr 340px` to `1fr 1fr 280px`.
- Header padding compresses from 18 px → 14 px horizontal; pipeline bar from 12 px → 10 px vertical.

### 5.2 Window chrome

- macOS: rely on the OS for traffic lights. Confirm `tauri.conf.json` window decorations match: `windows[0].decorations: true` (default; verify), `titleBarStyle: "Visible"` (default).
- The currently uncommitted `tauri.conf.json` change adjusts the window size — verify it does not also disable decorations. If it does, restore `decorations: true` in the same step.
- Linux/Windows: ship today's standard-decoration window. No bespoke chrome.
- The web app's "fake" header bar at 48 px is **inside** the OS window — Tauri's title bar sits above it and is not part of the redesign.

### 5.3 No new IPC

Every IPC call the redesign issues already exists in `crates/agentic-tauri/src/commands/`. No new permissions in `capabilities/` and no new commands in `permissions/`.

---

## 6. Cross-cutting concerns

### 6.1 Design tokens

A single source of truth: `apps/web-ui/src/styles/tokens.css` (new), imported once from `index.css`. It mirrors `colors_and_type.css`:

```css
:root {
  /* zinc OKLCH scale (50–950) */
  --zinc-50:  oklch(0.985 0 0);
  --zinc-100: oklch(0.967 0.001 286.375);
  /* … through 950 */

  /* semantic light */
  --bg-page: var(--zinc-100);
  --bg-surface: #ffffff;
  --bg-surface-2: var(--zinc-50);
  --fg: var(--zinc-950);
  --fg-muted: var(--zinc-500);
  --fg-subtle: var(--zinc-400);
  --border-soft: rgb(0 0 0 / 0.05);
  --border: rgb(0 0 0 / 0.10);
  --border-strong: rgb(0 0 0 / 0.15);

  /* type / radii / shadows */
  --font-sans: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  --radius-md: 0.375rem;
  --radius-lg: 0.5rem;
  --radius-xl: 0.75rem;
  --shadow-card: 0 1px 2px rgb(0 0 0 / 0.04);
  --shadow-popover: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
  --shadow-modal: 0 25px 50px -12px rgb(0 0 0 / 0.4);
}

:root[data-theme="dark"] {
  --bg-page: var(--zinc-950);
  --bg-surface: var(--zinc-900);
  --bg-surface-2: var(--zinc-800);
  --fg: #ffffff;
  --fg-muted: var(--zinc-400);
  --fg-subtle: var(--zinc-500);
  --border-soft: rgb(255 255 255 / 0.05);
  --border: rgb(255 255 255 / 0.10);
  --border-strong: rgb(255 255 255 / 0.15);
}
```

`tailwind.config.js` extends `theme.colors` with `bg-page`, `bg-surface`,
`bg-surface-2`, `fg`, `fg-muted`, `fg-subtle`, `border-soft`,
`border-strong`, plus per-status (`status-done`, `status-active`,
`status-queued`, `status-failed`, `status-info`) and per-agent
(`agent-architect`, `agent-developer`, `agent-qa`, `agent-reviewer`)
keys, all referencing `var(--…)`.

### 6.2 Inter font

- Load `Inter` from **Google Fonts CDN** — no self-hosted asset, no `.ttf`
  committed to the repo.
- In `apps/web-ui/index.html`, inside `<head>`:
  - Preconnect: `<link rel="preconnect" href="https://fonts.googleapis.com">` and
    `<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>`.
  - Stylesheet:
    `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:opsz,wght@14..32,100..900&display=swap">`.
- The `opsz`-variable file from Google Fonts also covers the
  "Inter Display" face used for big numerals — no separate asset is needed;
  the CSS token (`--font-display`) keeps pointing at `Inter` and the browser
  resolves the optical-size axis automatically.
- The Tailwind `theme.fontFamily.sans` token still points to `Inter`.
- This is a **new external request, not a new npm dep and not a committed
  asset.** No bundler config changes needed.
- Fallback stack (`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`)
  is applied at the `:root` level so rendering is graceful when the CDN is
  blocked or slow.

### 6.3 Theme toggle

- New hook `useTheme()` in `apps/web-ui/src/hooks/useTheme.ts`.
- Reads `localStorage["agentic.theme"]` ("light" | "dark") on mount; falls back to `prefers-color-scheme` media query.
- Sets `data-theme` on `<html>` whenever the value changes.
- Exposes `(theme, setTheme, toggle)`.

### 6.4 State shape (web)

Adopted from README §State Management. New file
`apps/web-ui/src/types/pipeline.ts`:

```ts
export type RunStateOverall = "idle" | "running" | "completed" | "failed";
export type AgentStatus = "queued" | "active" | "done" | "skipped" | "errored" | "failed";

export interface AgentInstance {
  id: string;
  status: AgentStatus;
  startedAt?: number;
  endedAt?: number;
}

export interface PermissionRequest {
  id: string;
  agent: string;
  tool: string;
  arg: string;
  scope: "shell.destructive" | "fs.write" | "network.outbound" | string;
  risk: "low" | "medium" | "high";
  reason: string;
  t: number;
}

export interface ActionItem {
  id: string;
  kind: "issue" | "warning" | "followup";
  title: string;
  description?: string;
  fromAgent: string;
}

export interface IssueTicket {
  id: string;
  title: string;
  labels: string[];
  body: string[];
  acceptance: string[];
}
```

These coexist with today's `RunState` / `StepInfo`. The new `PipelineBar`
derives `AgentInstance[]` from `RunState.steps` via a small adapter.

### 6.5 Drag-reorder (decision)

- **No new dependency.** HTML5 drag-and-drop (`onDragStart`, `onDragOver`,
  `onDrop`) is sufficient for a 4-card horizontal list and covers
  mouse + touch input. The `pipeline-bar.jsx` prototype uses raw HTML5 DnD
  already.
- Keyboard accessibility (roving-tabindex + arrow-key swap, or full
  `@dnd-kit/sortable` ARIA listbox semantics) is **deferred** as a feature
  request, tracked in §7 and `todo.md` tech-debt. Trigger: before public
  release / WCAG 2.1 AA pass.

### 6.6 Permission decision events

The redesign **renders** permission cards but does not introduce a
backend permission stream in this scope. The wiring is:

1. `useTauriEvents` filters envelopes for a (future) `PermissionRequest`
   event type. If none exist, the array stays empty and no card renders.
2. The decision callback fires a future IPC `resolve_permission` (not
   shipping in this redesign — flagged in §7).
3. For local visual review during the redesign, a non-prod test fixture
   surfaces a card via a feature flag in dev mode (`?demo-perm=1` query
   param). This is hidden from production builds.

### 6.7 TUI keyboard map

See §4.11.

### 6.8 Agent discovery (backend-scoped)

The pipeline locates each agent file at runtime by walking a short list of
candidate paths. The search is **backend-scoped**: which paths are checked
depends on which backend drives the run.

**Universal first-priority override (both backends):**

1. `<repo_root>/.agentic/agents/<name>.md`

**ClaudeCode:**

2. `<repo_root>/.claude/agents/<name>.md`
3. `$HOME/.claude/agents/<name>.md`

**CopilotCli:**

2. `<repo_root>/.github/agents/<name>.md`
3. `$HOME/.copilot/agents/<name>.md`

First match wins. If no path resolves to an existing file, the pre-flight
check surfaces a `CoreError::AgentNotFound` that lists all three searched
paths — the error message is actionable (run `agentic-cli init` or
`agentic-cli init --copilot` to scaffold the files).

The legacy `<repo_root>/agents/` path is no longer in the search list.
Files placed there are invisible to the pipeline.

**init flag pairing:**

| Flag | Backend | Destination |
|---|---|---|
| (none) | claude-code | `<repo>/.claude/agents/` |
| `--copilot` | copilot-cli | `<repo>/.github/agents/` |
| `--global` | claude-code | `$HOME/.claude/agents/` |
| `--copilot --global` | copilot-cli | `$HOME/.copilot/agents/` |
| `--agentic` | either | `<repo>/.agentic/agents/` (universal override) |

---

## 7. Out of scope (this redesign)

The following are explicitly **deferred**. None block the redesign; each
gets a logged tech-debt entry in `todo.md` Phase 13 with a trigger.

| Item | Why deferred | Trigger to revisit |
|---|---|---|
| Agent configure side-panel | No backend API for per-agent config; UI surface only renders the kebab item as a no-op. | When `pipeline.toml` per-agent overrides ship in core. |
| Real avatar / GitHub identity API | No identity backend exists; placeholder initials suffice. | When OAuth profile fetch ships. |
| Bespoke agent illustrations | No designer-provided art beyond the emoji glyphs in `data.js`. | When the design system gets a per-agent SVG library. |
| Backend `PermissionRequest` event variant | `agentic-core::events::Event` does not yet have a `PermissionRequest` variant. The redesign renders `PermissionCard` against a fixture only; live wiring requires the orchestrator to gain a permission-gate hook in `agentic-core`. | When the orchestrator gains a permission-gate hook in `agentic-core`. |
| `pipeline.toml` editing UI | Backend already parses but mutating UI is a separate spec. | When pipeline-config persistence lands. |
| Real ticket-source body in Issue column | No `get_ticket(ticket_url)` IPC yet. | When ticket-source integration ships in core. |
| Keyboard drag-reorder for pipeline bar | HTML5 DnD covers mouse + touch; keyboard reorder needs a roving-tabindex pattern with arrow-key swap that adds ~1 step of work and isn't in the design hand-off. | Before public release / WCAG 2.1 AA pass. |

---

## 8. Acceptance criteria

### Web (Surface A)

1. The header bar renders at 48 px height with brand, slug, run-state badge, settings, theme toggle, and avatar — all positioned per §3.1.
2. The pipeline bar renders one card per agent in `RunState.steps`, with status rings matching `StepStatus`. The currently-running agent is highlighted with the amber ring + tinted bg.
3. Drag-reorder visibly moves a card across gaps; the resulting `agents` array reflects the new order.
4. The agent picker popover opens on `+` chip and `+ Add agent` clicks, search-filters the agent library, and dismisses on outside click / Esc.
5. The chat column renders user / agent / system messages with the visual contract in §3.4. Slash and `@mention` popovers still work; Cmd/Ctrl+Enter sends.
6. The activity column renders today's events filtered by tab. Tool calls render as cards; info rows as monospace lines.
7. The issue column renders the active run's ticket label, a placeholder body, and (on completion) action items + "Create spec" button.
8. The spec dialog opens, validates the title is non-empty, and on submit calls `start_ticket_run` with the title.
9. The theme toggle flips `data-theme` on `<html>` and persists to `localStorage`. All colors swap.
10. All existing `data-testid` selectors still resolve to their original elements; no existing test renamed unless its assertion was renamed in the same step.

### Tauri (Surface C)

11. When loaded inside Tauri (`window.__TAURI_INTERNALS__` truthy), the right column is 280 px instead of 340 px.
12. `tauri.conf.json` window decorations remain enabled on macOS; traffic lights come from the OS.

### TUI (Surface B)

13. The TUI renders, top to bottom: title bar (28 px) · issue header · ASCII pipeline bar · tab bar · body pane · status/command line.
14. Pressing `1`, `2`, `3` switches the body pane to logs, chat, issue respectively.
15. Pressing `?` toggles the help overlay; Esc dismisses.
16. With a pending permission, pressing `y` / `s` / `n` resolves it with the corresponding decision and emits a flash message in the status line.
17. The mode indicator at right of the status line shows NORMAL / INSERT / COMMAND with the correct color.
18. The dark palette (`#0d0e10` bg, `#5eead4` accent, etc.) is the only palette — no light theme.

### Cross-cutting

19. `pnpm -F @agentic/web-ui test` passes after every step.
20. `cargo test --workspace --all-features` passes after every step.
21. `cargo clippy --workspace --all-features --all-targets -- -D warnings` and `cargo fmt --all -- --check` pass after every step.
22. No new npm or cargo dependencies introduced without an entry in §6 or in the relevant todo step's "Notes" block.
