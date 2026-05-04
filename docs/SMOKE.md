# Smoke-Test Playbook

A repeatable walkthrough of every UI surface. Run after any significant change
to the web-ui to catch regressions before they reach main.

---

## Live permission gate smoke test (P.6.1.b)

A `#[ignore]`d Rust integration test that exercises the full backend stack with
a real Claude Code subprocess and a real `AsyncGate`.

### Prerequisites

- `ANTHROPIC_API_KEY` set in your shell
- `claude` (Claude Code CLI) on `PATH` — verify with `which claude`
- Internet access (the CLI calls the Anthropic API)

### Run

```bash
cargo test -p agentic-cli --test e2e_permissions_live -- --ignored --nocapture
```

### What it does

1. Creates a temporary sandbox directory with two placeholder Python files
   (`palindrome.py`, `test_palindrome.py`) and a git repo.
2. Writes a `permissions.toml` with `Read(*)`, `Write(*)`, `Edit(*)`, `Bash(*)`
   on the allowlist and `Bash(rm -rf /*)`, `Bash(sudo *)` on the denylist.
3. Wires a real `ClaudeCodeBackend` (via `ClaudeCodeBackend::from_env()`) through
   a real `AsyncGate` backed by the bus.
4. Runs a single `tdd-developer` step asking Claude to implement a palindrome
   function in Python.
5. Drains bus envelopes and asserts:
   - At least one `ToolUseStart` was observed (Claude called a tool).
   - At least one `PermissionResolved` arrived (the gate fired).
   - At least one `PermissionResolved` has `source=AllowlistConfig` (the allow
     patterns matched).
   - The run completes within 5 minutes without a panic.

### Cost

Uses `claude-haiku-4-5-20251001` (cheapest Anthropic model) to minimise API
spend. A typical palindrome task costs < $0.01.

### Source

`crates/agentic-cli/tests/e2e_permissions_live.rs`

---

## Modes

### 1 — Browser dev (fast iteration)

```sh
pnpm -F @agentic/web-ui dev
# opens http://localhost:5173
```

The dev-invoke mock (`apps/web-ui/src/utils/devInvokeMock.ts`) stubs
`window.__TAURI_INTERNALS__.invoke`. Calling "Run pipeline" dispatches
`start_ticket_run` to the mock, which schedules a simulated 4-agent happy-path
run entirely in the browser — no Rust process, no LLM calls.

**Use for:** UI layout, styling, interaction logic, slash/mention popovers,
SpecDialog, Settings modal.

**Limitations:**
- Only one scenario (happy path, no findings, no rate limits).
- `start_scripted_run` is not implemented in the mock (always returns `undefined`).
- Real permissions flow is absent (GH #88).
- `list_runs` returns `[]` — History tab always shows empty state.

---

### 2 — Real Tauri backend

```sh
cd crates/agentic-tauri
cargo tauri dev
# optionally: AGENTIC_WORKSPACE_ROOT=/path/to/project cargo tauri dev
```

This runs the full Rust backend. IPC calls hit the real handlers. LLM calls
are made against the configured backend (claude-code by default).

**Use for:** real Findings, file-change events, permission prompts, History
tab, Settings → existing auth accounts, scripted runs.

**Cost:** each real run calls an LLM and burns tokens. Use scripted runs
(mode 3) to exercise the UI without LLM cost.

---

### 3 — Scripted runs (deterministic, no LLM cost)

Scripted runs replay a JSON fixture through the real Rust event bus without
invoking any LLM. They are the best way to test specific scenarios
(findings, rate-limit warnings, severity classifier) repeatedly and cheaply.

**Fixture files** live in `crates/agentic-tauri/scripted-runs/`:

| File | Scenario |
|---|---|
| `demo.json` | 2-step partial run — architect passes, reviewer emits 2 findings |
| `happy-path.json` | 4-agent clean run, no findings, completes with `status: "completed"` |
| `failure-rate-limit.json` | 4-agent run with `severity: "warning"` rate-limit findings between steps |
| `mixed-findings.json` | Reviewer step emits error + warning + info findings for severity-classifier testing |

#### Invoking a scripted run

After W.8.5 removed `StartRunForm`, there is no UI affordance for scripted
runs. Invoke the IPC command directly from the browser devtools console while
running in **real Tauri mode**:

```js
// Tauri dev only — the browser dev mock does not implement start_scripted_run.
await window.__TAURI_INTERNALS__.invoke("start_scripted_run", {
  scriptPath: "scripted-runs/happy-path.json",
  delayMs: 200,
});
```

`scriptPath` must be relative to the Tauri app's working directory (i.e. the
`crates/agentic-tauri/` directory when running `cargo tauri dev`).
`delayMs` controls the inter-event sleep in milliseconds; 200 is a comfortable
pace for watching the activity column update. Pass `0` to drain instantly.

The command returns the `run_id` string on success. Errors surface as a
rejected promise with a string message.

> Note: the path is validated against cwd and app data dir. Relative paths
> resolve from cwd, so `"scripted-runs/happy-path.json"` works when the
> working directory is `crates/agentic-tauri/`.

---

## Smoke Checklist

Work through each section in order. Check off items as you verify them.
Note the mode you used (dev / Tauri / scripted) and any failures.

---

### Header bar (W.1.x)

| # | Check | Mode |
|---|---|---|
| H1 | Brand tile ("Agentic") and workspace slug render in the header | dev |
| H2 | Theme toggle (sun/moon icon) flips between light and dark mode | dev |
| H3 | After flipping theme, reload the page — theme persists (`localStorage`) | dev |
| H4 | Settings cog icon is visible and recognizable as a gear | dev |
| H5 | Run pipeline button is present in idle state | dev |
| H6 | Start a run — state pill changes from idle → running with elapsed timer | dev |
| H7 | Run completes — pill changes from running → completed | dev |

---

### Pipeline bar (W.2.x)

| # | Check | Mode |
|---|---|---|
| P1 | 4 agent cards render: architect / tdd-developer / qa / reviewer | dev |
| P2 | Each card shows its per-agent SVG glyph | dev |
| P3 | Step numbers 01 / 02 / 03 / 04 appear left of each tile | dev |
| P4 | Drag a card across a gap and drop — order updates in place | dev |
| P5 | Reload after reorder — confirm order persists (or resets if not persisted) | dev |
| P6 | "+ Add agent" end cap is present and clickable | dev |
| P7 | Clicking "+ Add agent" opens the agent picker | dev |
| P8 | Selecting an agent from the picker appends it to the pipeline | dev |
| P9 | "+" chips between cards are present and open picker at correct index | dev |
| P10 | Kebab menu on a card: "Remove" drops the card | dev |
| P11 | Kebab menu: "Skip" dims the card (opacity-50 + strikethrough name) | dev |
| P12 | Kebab menu: "Configure" opens a placeholder modal | dev |
| P13 | Start a run — StatusDot pills update: queued → Running → Done per agent | dev |

---

### Chat column (W.4.x)

| # | Check | Mode |
|---|---|---|
| C1 | Composer is present with chips below the textarea | dev |
| C2 | Paper-plane send button is visible | dev |
| C3 | Doc-icon button for "New spec" is visible | dev |
| C4 | Send button background flips dark when input has content | dev |
| C5 | Typing `/` opens the slash command popover | dev |
| C6 | Slash popover shows 4 commands: /plan, /brainstorm, /develop, /spec | dev |
| C7 | Pressing Esc closes the slash popover without sending | dev |
| C8 | Reopening `/` after Esc shows the full list again | dev |
| C9 | Typing `@` opens the mention popover with the 12 agents | dev |
| C10 | Cmd/Ctrl+Enter sends the message; bare Enter inserts a newline | dev |
| C11 | Clicking the doc icon opens SpecDialog | dev |
| C12 | SpecDialog: Create button is disabled when title is empty | dev |
| C13 | SpecDialog: backdrop click closes the dialog | dev |
| C14 | SpecDialog: Esc key closes the dialog | dev |
| C15 | SpecDialog: Cancel button closes the dialog | dev |
| C16 | SpecDialog: fill title and click "Create & run" — a run starts | dev |

---

### Activity column (W.5.x)

| # | Check | Mode |
|---|---|---|
| A1 | Header tabs render: All / Tool calls / Permissions / Errors | dev |
| A2 | Tab counts update as events arrive | dev |
| A3 | Clicking "Tool calls" tab hides non-tool-call rows | dev |
| A4 | Clicking "Errors" tab hides non-error rows | dev |
| A5 | Tool calls render as ToolCallCard with collapsible body | dev |
| A6 | Run a scripted fixture with findings — findings appear in the list | scripted |
| A7 | Finding with `severity: "error"` shows a red chip | scripted |
| A8 | Finding with `severity: "warning"` or `"info"` shows a plain info row | scripted |
| A9 | Agent name shown as human-readable label (architect, not a ULID) | dev |
| A10 | Run progression: agents flip queued → active → done as events arrive | dev |

For A6–A8, use `mixed-findings.json` via the scripted-run console snippet.

---

### Issue column (W.6.x)

| # | Check | Mode |
|---|---|---|
| I1 | Header strip shows the ticket id and a StatusDot pill | dev |
| I2 | StatusDot pill maps correctly from run state (idle / running / done) | dev |
| I3 | Description and Acceptance criteria section labels are present | dev |
| I4 | Acceptance items show `[ ]` during a run | dev |
| I5 | Acceptance items flip to `[x]` when the run completes | dev |
| I6 | Action items section appears when run completes with findings | scripted |
| I7 | "Create spec" button in action items opens SpecDialog | scripted |
| I8 | Starting a run via SpecDialog — title appears in issue column header | dev |
| I9 | Starting a run via `/plan` — title appears in issue column header | dev |

For I6–I7, use `mixed-findings.json` or `demo.json` via scripted run.

---

### Settings modal (W.8.2)

| # | Check | Mode |
|---|---|---|
| S1 | Clicking the header cog icon opens the Settings modal | dev |
| S2 | General tab is selected by default and shows settings content | dev |
| S3 | History tab shows past runs list (or empty state if no runs) | Tauri |
| S4 | Backdrop click closes the modal | dev |
| S5 | Esc key closes the modal | dev |
| S6 | Close button (×) dismisses the modal | dev |
| S7 | There is NO standalone "History" button on the header bar | dev |

---

### Theme tokens

| # | Check | Mode |
|---|---|---|
| T1 | Flip theme — `<html data-theme>` attribute changes in DevTools Elements | dev |
| T2 | Background, foreground, border, and muted text all use themed tokens | dev |
| T3 | Status pills (idle / running / done) use correct themed colors | dev |
| T4 | `localStorage` key persists the chosen theme across hard reload | dev |

---

## Known Limitations and Dev-Mode Caveats

- **No real permissions in dev mode** — the mock returns no permission-request
  events. Permissions tab will always show 0. (GH #88)
- **`/plan` dispatches correctly** — `/brainstorm`, `/develop`, and `/spec`
  are visible in the picker but the dispatcher returns `unknown_command` at
  the IPC layer. (GH #95)
- **Spec body dropped at IPC layer** — the UI captures the spec body locally
  for the IssueColumn, but the value is not forwarded to the backend. (GH #92)
- **`ToolUseEnd` events filtered** — result chip refinement is not
  implemented; filtering prevents duplicate cards.
- **Real backend rate limits** — during heavy Tauri runs the Anthropic API may
  throttle requests. This is distinct from the subscription cap. Use scripted
  runs to reproduce rate-limit warnings without incurring API calls.
- **History tab empty in dev mode** — `list_runs` returns `[]` in the mock;
  the empty state is expected.

---

## Reporting Bugs

Include in your report:

1. **Mode**: browser dev / real Tauri / scripted (and which fixture)
2. **Scenario**: what you were doing step by step
3. **Observed**: what actually happened
4. **Expected**: what should have happened
5. **Console errors**: copy any stack traces from the browser DevTools console
6. **Phase/step reference**: e.g. "W.4.3 slash popover" if known

File as a GitHub issue with label `bug`. Reference the relevant smoke-test
check number (e.g. "C7 fails after typing Esc — popover reopens immediately").
