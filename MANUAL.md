# Agentic — User Manual

A simplified, honest user manual for what works today and how to use it. Not the spec — see `spec.md` for the full design. Not the roadmap — see `todo.md` for what's shipped vs planned.

> **Freshness**: this file is updated alongside any feature/bug commit that changes user-visible behaviour. If a section here disagrees with the code, trust the code and file an issue.

---

## 1. What is Agentic?

A cockpit for driving and watching AI-coding pipelines:

```
architect → tdd-developer → qa → reviewer
```

Each step is an agent backed by a real CLI tool (`claude` or `copilot`). You give it a ticket; the pipeline produces a spec, runs TDD, runs tests, and reviews the result. You triage the findings and ship.

Three shells share one Rust core:

| Shell | Status |
|---|---|
| **CLI** (`agentic-cli`) | working — recommended for real work today |
| **Tauri desktop app** | MVP cockpit — observability + chat stub + scripted demos |
| **TUI** | not built yet (Phase 12) |
| **VS Code extension** | not built yet (Phase 14) |

> **Realistic expectation**: For driving real pipelines against real tickets, use the CLI. The Tauri app is currently a cockpit for **watching** runs and **demoing** the pipeline shape with scripted JSON; it does not yet kick off real backend pipelines from the UI.

---

## 2. What works today vs what's stubbed

Roughly Phase 11 of 14 is in flight. Quick truth-table:

| Capability | Working | Stubbed | Not yet |
|---|---|---|---|
| `agentic-cli run --ticket "fix X"` against your repo | ✅ | | |
| `agentic-cli run --scripted demo.json` | ✅ | | |
| `agentic-cli doctor` / `migrate` / `init` | ✅ | | |
| Claude Code backend | ✅ | | |
| Copilot CLI backend | ✅ | | |
| Event persistence (SQLite) | ✅ | | |
| Tauri shell — scripted run + cockpit + findings triage | ✅ | | |
| Tauri shell — chat (echo only) | | ✅ | |
| Tauri shell — `/plan` `/status` `/cancel` slash commands | | ✅ | |
| Tauri shell — `@architect …` mentions | | ✅ | |
| Tauri shell — kick off ticket-driven pipeline from chat | | | ❌ |
| GH / GitLab OAuth + ticket fetch | core implemented | | not exposed in UI |
| TUI shell | | | ❌ |
| VS Code extension | | | ❌ |

"Stubbed" means the IPC + UI flow is wired but the result text is `[STUB] …` placeholder until the real backend lands.

---

## 3. First-time setup

### Prerequisites

- **Rust** 1.83+ (`rustup install stable`)
- **pnpm** + **Node 20+** (for the Tauri shell)
- **Claude Code** OR **Copilot CLI** installed and logged in
- **gh** or **glab** for ticket fetch from GitHub / GitLab
- macOS / Linux / Windows

### Install dependencies and build

```fish
git clone <this repo>
cd agentic
pnpm install                      # web-ui deps
cargo build --workspace           # all Rust crates
```

### Verify the environment

```fish
cargo run -p agentic-cli -- doctor
```

Expected output — each row should say `found at /…`:

```
tool        status
----------------------------------------
claude      found at /Users/you/.local/bin/claude
copilot     found at /opt/homebrew/bin/copilot
gh          found at /opt/homebrew/bin/gh
glab        found at /opt/homebrew/bin/glab
```

Missing rows are non-fatal — only the backends you actually use need to be present.

### Initialize the database

```fish
cargo run -p agentic-cli -- migrate
```

Creates `~/Library/Application Support/agentic/state.db` (macOS) — equivalent path on Linux/Windows under `directories::ProjectDirs`. Idempotent; safe to re-run after a pull.

### Scaffold a project's `.agentic/` directory

Before you can run a real ticket-driven pipeline against another repo, that repo needs four agent files. Bootstrap them in one shot:

```fish
cd ~/work/my-project
~/agentic/target/debug/agentic-cli init
```

This writes:

```
.agentic/agents/architect.md
.agentic/agents/tdd-developer.md
.agentic/agents/qa.md
.agentic/agents/reviewer.md
```

Each file has reasonable defaults (Opus for architect, Sonnet for tdd-developer/reviewer, Haiku for qa) and a starter system prompt. You should read each one and edit the prompt to fit your project's conventions.

Flags:
- `--target <path>` — scaffold into a different directory (defaults to cwd)
- `--force` — overwrite existing files (default: refuse, so hand-edits aren't clobbered)

---

## 4. The pipeline

Four roles, run in sequence per ticket. Each is a markdown file with TOML frontmatter:

| Role | Job |
|---|---|
| `architect` | Read the ticket, propose a step-by-step plan and design |
| `tdd-developer` | Pick the first plan step, write a failing test, make it pass, refactor |
| `qa` | Run the affected tests, report pass/fail |
| `reviewer` | Review the diff, surface findings (`fix` / `tech-debt` / `ignore`) |

The orchestrator is a state machine in `agentic-core` (`crates/agentic-core/src/pipeline/sm.rs`). A tdd-developer step can loop with `qa` if tests fail; a `reviewer` step can loop with `tdd-developer` to apply fixes. There's a hard cap on retries.

Findings flow into a typed `findings` table. You triage each one (currently via the Tauri UI for scripted demo runs) → `fix` / `tech-debt` / `ignore`. Triage state survives reloads (round-trips through SQLite).

---

## 5. Agent files

The pipeline looks for agent definitions in this order under your **target repo's root**:

1. `<repo>/.agentic/agents/<name>.md`
2. `<repo>/.claude/agents/<name>.md`
3. `<repo>/agents/<name>.md`

Each is a markdown file with TOML frontmatter between `+++` fences, e.g. `<repo>/.agentic/agents/architect.md`:

```markdown
+++
name = "architect"
description = "Reads a ticket and proposes a step-by-step plan"
model = "claude-sonnet-4-6"
pipeline_role = "Architect"
timeout_seconds = 600
+++

# Architect

You are the architect. Read the ticket. Produce…

(everything below the fence becomes the agent's system prompt)
```

Required fields: `name`, `description`, `pipeline_role`. Optional: `model`, `tools`, `allowed_questions`, `timeout_seconds`. The body becomes the agent's system prompt.

You need four files: `architect.md`, `tdd-developer.md`, `qa.md`, `reviewer.md`. Without them the pipeline fails immediately with `agent 'architect' not found`.

---

## 6. Using the CLI (recommended for real work today)

The CLI is the working entry point for actual pipeline runs.

### Run a ticket-driven pipeline

From inside the repo you want Agentic to work on:

```fish
cd ~/work/my-project

# Fix a bug — free-text ticket
cargo run --manifest-path ~/agentic/Cargo.toml -p agentic-cli -- \
  run --ticket "Login race causes 500 when two clients hit /auth concurrently" \
  --backend claude-code

# Or with a model override:
… --model claude-sonnet-4-6

# Switch to Copilot CLI:
… --backend copilot-cli
```

The command prints one JSON envelope per line (one event), then exits with the run summary. Pipe to `jq` for readability:

```fish
… run --ticket "…" 2>&1 | jq -c '{type:.event.type, run:.run_id[0:6], step:.step_id, msg:.event.data.summary // .event.data.content // ""}'
```

What the run does:
1. Records `RunStarted` for the ticket
2. Discovers `architect.md` from the agent search paths and runs it
3. Streams `TextDelta` / `ToolUseStart` / `ToolUseEnd` envelopes to stdout
4. On `StepComplete(passed)`, advances to `tdd-developer`
5. Writes everything to `stream_events` in SQLite
6. Exits with `RunComplete(status, summary)`

### Replay a scripted JSON for offline testing

```fish
cargo run -p agentic-cli -- run --scripted scripted-runs/demo.json
```

Same envelope format as a real run, but the events come from a JSON file — no LLM calls, no network. Useful for testing UI changes and agent file iteration without burning tokens.

### Override the data dir (for test isolation)

```fish
agentic-cli --data-dir /tmp/agentic-test run --ticket "…"
```

---

## 7. Using the Tauri shell

Today: a cockpit for **watching** runs (especially scripted demos), **chatting** with stubs, and **triaging findings**.

### Launch

Two options:

**A — production-ish bundle (one command):**
```fish
pnpm --filter @agentic/web-ui build
ln -sf ../../apps/web-ui/dist crates/agentic-tauri/frontend
cargo run -p agentic-tauri
```

**B — split dev with hot reload:** add to `crates/agentic-tauri/tauri.conf.json` `build` block (don't commit):
```json
"devUrl": "http://localhost:5173",
"beforeDevCommand": "pnpm --filter @agentic/web-ui dev"
```
Then:
```fish
cargo tauri dev
```

### What you can do in the UI

- **Cockpit / Stepper** — pipeline progress (`architect → tdd-developer → qa → reviewer`), token totals, cost
- **EventList** — the raw envelope stream (most-recent 500, sliding window)
- **StartRunForm** — kick off a *scripted* run from a JSON path. Real ticket runs are not exposed here yet.
- **ChatPane** — type a message; assistant echoes it (stub). Slash + mention commands recognised:
  - `/plan #42` → system message `[STUB] /plan…`
  - `/status <run-id>`, `/cancel <run-id>` → same stubs
  - `@architect ship it` → routes through `mention_agent` IPC, streams two stub envelopes onto the dedicated `agentic://mention-event` channel which renders as `chat-message-mention` rows
- **FindingsTable** — for the run shown in the cockpit, lists `Event::Finding` entries with `[Fix] [Tech-debt] [Ignore]` buttons. Triage writes through the IPC and updates `findings.triage` in SQLite. See §8 for what each tag means in practice.

### The demo loop (CP-9)

Save as `scripted-runs/demo.json` at repo root:

```json
[
  {"type":"StepStarted","data":{"agent":"architect","model":"sonnet"}},
  {"type":"TextDelta","data":{"content":"Designing spec…"}},
  {"type":"StepComplete","data":{"status":"passed","summary":"spec ready","token_usage":{"input_tokens":120,"output_tokens":340,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"cost_usd":0.01,"duration_ms":1200}},
  {"type":"StepStarted","data":{"agent":"reviewer","model":"sonnet"}},
  {"type":"Finding","data":{"finding_id":"f1","severity":"warning","file":"src/main.rs","line":42,"message":"missing-error-handling","suggestion":null}},
  {"type":"Finding","data":{"finding_id":"f2","severity":"error","file":"src/auth.rs","line":17,"message":"hardcoded-secret","suggestion":"move to settings"}},
  {"type":"StepComplete","data":{"status":"needs_triage","summary":"2 findings","token_usage":{"input_tokens":300,"output_tokens":500,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},"cost_usd":0.02,"duration_ms":2400}}
]
```

Paste the path into StartRunForm, set delay 200ms, click **Start**. Watch the Stepper, EventList, and FindingsTable populate. Triage a finding — reload the window — the badge should persist.

---

## 8. Triage tags — Fix / Tech-debt / Ignore

Each `Finding` the reviewer emits has three possible triage states. You set them via the `[Fix] [Tech-debt] [Ignore]` buttons on a row in the FindingsTable (Tauri UI today; future TUI / VS Code surfaces will follow). The triage state lives on the `findings` row — it survives reloads and is queryable with `SELECT triage FROM findings WHERE run_id = '…'`.

### `Fix` — block the merge

The finding represents a real defect or correctness regression that must be addressed before the change ships. The pipeline's reviewer-loop semantics (when wired in Phase 13+) will route findings tagged `fix` back to the tdd-developer agent for another pass; max 3 loops, then the run completes with status `failed`.

**Use for:**
- Bugs the reviewer caught that have no test coverage yet
- Spec or contract violations
- Security issues (hardcoded secrets, injection vectors, missing auth checks)
- Anything the reviewer marked `severity = "error"` that you confirm is real

### `Tech-debt` — file as a follow-up

The finding is real but not urgent enough to block this change. You acknowledge it and expect to address it later. Convention in this project: file each `tech-debt` triaged finding as a GitHub issue with the `tech-debt` label so it doesn't fall on the floor.

**Use for:**
- Latent issues the reviewer flagged but that the current change didn't introduce
- Ergonomics, naming, or refactor opportunities
- Missing test coverage for an unrelated area
- Anything `severity = "warning"` that you accept for now

When you tag `tech-debt`, the run completes with `status = completed_with_tech_debt` (already wired in `RunStatus`).

### `Ignore` — false positive or out-of-scope

The finding is wrong, irrelevant to this change, or so minor it isn't worth tracking. The row stays in the DB with `triage = ignore` for audit but doesn't generate an issue and doesn't block the merge.

**Use for:**
- Reviewer's nits the formatter would have caught anyway
- Findings about code outside the change's scope
- Genuine false positives (the reviewer misread something)

### Re-triage

You can change a finding's triage by clicking a different button on the row. The badge updates locally and the new value writes through `triage_finding(findingId, triage)`. Re-triage is idempotent in the DB — only the latest button click counts.

### Querying triage state

```fish
sqlite3 ~/Library/Application\ Support/agentic/state.db "
  SELECT
    severity,
    COALESCE(triage, '<untriaged>') AS triage,
    message
  FROM findings
  WHERE run_id = '01...'
  ORDER BY created_at;
"
```

---

## 9. Practical scenarios

### Scenario A — fix a bug in another repo

```fish
cd ~/work/my-project

# 1. Scaffold agent files (one-time per repo)
~/agentic/target/debug/agentic-cli init
# Edit .agentic/agents/{architect,tdd-developer,qa,reviewer}.md to taste.

# 2. Verify tools
~/agentic/target/debug/agentic-cli doctor

# 3. Run the pipeline against your bug
~/agentic/target/debug/agentic-cli run \
  --ticket "Login race: two concurrent /auth requests can return 500" \
  --backend claude-code 2>&1 | tee /tmp/run.log

# 4. (Optional) Open the Tauri shell in another terminal — it'll show the
#    DB-persisted events from this run via get_event_history once you wire
#    a runId. (Direct mid-run streaming from a CLI run into the desktop UI
#    is not wired yet — they share the same DB but separate buses.)
```

### Scenario B — develop a feature TDD-style

Same as A, but with a feature description as the ticket. The pipeline expects the architect to plan the feature, the tdd-developer to write failing tests then implement, qa to run them, reviewer to surface findings.

```fish
agentic-cli run --ticket "Add export-to-CSV button on the reports page that …"
```

### Scenario C — debug a pipeline run

```fish
# Find the run
sqlite3 ~/Library/Application\ Support/io.agentic.app/agentic.db \
  "SELECT id, ticket_ref, status, summary FROM runs ORDER BY started_at DESC LIMIT 5;"

# See its events
sqlite3 -header -column ~/Library/Application\ Support/io.agentic.app/agentic.db \
  "SELECT seq, event_type, hex(payload) FROM stream_events WHERE run_id='01...' LIMIT 20;"

# See findings + triage
sqlite3 -header -column … \
  "SELECT id, severity, message, triage FROM findings WHERE run_id='01...';"
```

The DB is the ground truth. The Tauri UI's history is read from `stream_events` via `get_event_history(runId)`.

### Scenario D — iterate on agent prompts without burning tokens

Build a scripted JSON that mimics the events your real run would produce, run it through `agentic-cli run --scripted`, observe the cockpit's response. Tweak the agent system-prompt; rerun. No LLM calls.

---

## 10. Where things live

| What | Path (macOS) |
|---|---|
| SQLite database | `~/Library/Application Support/io.agentic.app/agentic.db` |
| App data dir | `~/Library/Application Support/io.agentic.app/` |
| Logs | stderr (Tauri) / stdout (CLI). Set `RUST_LOG=agentic_core=debug,agentic_tauri=debug`. |
| Build artifacts | `target/debug/` and `target/release/` at repo root |
| Web UI source | `apps/web-ui/` |
| Rust crates | `crates/agentic-{core,cli,tauri,meta-tests}/` |
| Tauri config | `crates/agentic-tauri/tauri.conf.json` |
| Migrations | `crates/agentic-core/src/db/migrations/000*.sql` |

Linux: `~/.local/share/io.agentic.app/`. Windows: `%APPDATA%\io.agentic\app\`.

To start fresh:
```fish
rm ~/Library/Application\ Support/io.agentic.app/agentic.db
agentic-cli migrate
```

---

## 11. Troubleshooting

### "agent 'architect' not found"

You're running against a repo that has no agent files. Add them under `.agentic/agents/` (see §5).

### Tauri panics at startup with "no reactor running"

Fixed in commit `c868ac1` (April 2026). If you still see it, your tree is older — pull `main`.

### `cargo tauri dev` errors with `ENOENT … apps/web-ui`

The `beforeDevCommand` path is wrong. Use:
```json
"beforeDevCommand": "pnpm --filter @agentic/web-ui dev"
```
That works regardless of the directory you invoked `cargo tauri dev` from.

### CI's `sigkill_escalation_after_grace_period` test is flaky

It's a timing-based test on a busy CI machine; usually passes on retry. Not yet pinned.

### `start_scripted_run` rejects path "outside scope"

The path validator only allows files under `cwd` or the app's data dir. Move your JSON into the repo root or under `~/Library/Application Support/io.agentic.app/`.

### Findings table is empty after a real CLI run

Real CLI ticket runs use a separate DB connection from the Tauri app's bus. The Tauri UI today reads events that the *Tauri-spawned scripted run* persists. Wiring CLI runs into the Tauri cockpit live is on the roadmap — for now, query SQLite directly (Scenario C) or use scripted demos to exercise the UI.

### Model access / auth errors from a backend

`claude` and `copilot` handle their own auth — the pipeline shells out to them. If a step says `agent 'X' not found in PATH` or auth-related errors, fix it in the underlying CLI tool first (`claude /login`, `gh auth login`).

---

## 12. Project layout

```
agentic/
├── apps/web-ui/              React + Vite + Tailwind + Vitest (Tauri webview)
├── crates/
│   ├── agentic-core/         Core: pipeline, events, DB, backends, agents
│   ├── agentic-cli/          CLI binary (`agentic-cli`)
│   ├── agentic-tauri/        Tauri shell (binary + IPC commands)
│   └── agentic-meta-tests/   Cross-cutting integration tests + CI shape
├── spec.md                   Full design spec
├── todo.md                   Step-by-step roadmap (62 steps, ~27 done)
└── MANUAL.md                 This file
```

Key invariants:
- **No daemon** — each shell holds its own state via embedded SQLite.
- **Strict serial concurrency at MVP** — one active run per workspace.
- **Event envelopes are normalised across backends** — the same `Event` enum from `claude-code` and `copilot-cli`.

---

## 13. Roadmap pointers

- Tauri shell completion (Phase 11): findings table ✅ wired (Step 11.5). Real ticket runs from chat input still pending.
- TUI (Phase 12): not started.
- Auth UI / settings panel (Phases 7–8 surfaced in shells): core ready; UI not exposed.
- VS Code extension (Phase 14): not started.

For exact status see `todo.md`. The convention: `### [x] Step N.N` = shipped, `### Step N.N` = pending.

---

## 14. Getting help

- **What does X command do?** — `cargo run -p agentic-cli -- <cmd> --help`
- **What does the spec actually say?** — `spec.md`
- **What's planned vs done?** — `todo.md`
- **What's in the DB right now?** — `sqlite3 ~/Library/Application\ Support/io.agentic.app/agentic.db ".tables"`
- **What's the latest event for a run?** — query `stream_events`; payloads are MessagePack-encoded `Event` variants (use `rmp-serde` to decode programmatically).

This manual reflects the state at commit-time of its addition. If something here doesn't match the code, trust the code; this file may have drifted.
