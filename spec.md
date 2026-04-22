# Agentic — UI & Chat for Agentic Orchestration

Developer hand-off specification.

Version: 0.1 (MVP)
Date: 2026-04-21
Status: Ready for implementation

---

## 1. Overview

Agentic is a cross-platform UI + chat layer for the `agentic-orchestration` pipeline. It gives developers a cockpit for driving and monitoring their agentic pipelines (`architect → tdd-developer → qa → reviewer`) and a chat surface for talking to the underlying LLM backends.

The product ships in three form factors simultaneously from a shared Rust core:

- **Tauri desktop app** — macOS, Windows, Linux
- **Terminal UI (TUI)** — macOS, Windows, Linux
- **VS Code extension** — VS Code, VSCodium, Cursor, Windsurf

All three shells embed the same Rust library and render the same structured event streams. No daemon, no IPC — each process holds its own state via an embedded SQLite.

---

## 2. Goals & Non-Goals

### Goals

- One product, three shells, shared core.
- Drive `architect → tdd-developer → qa → reviewer` against **Claude Code** and **Copilot CLI** (with model choice).
- Support two canonical profiles: **GitHub + Issues + Claude Code** and **GitLab + Jira + Copilot CLI**.
- PKCE-based OAuth authentication with GitHub/GHES and GitLab (cloud + self-hosted).
- Cross-platform secret storage via OS keychain.
- Zero-config first-run when a user opens a repo that already follows the conventions.
- Structured event stream normalized across backends; renders at each shell's native fidelity.
- Strict serial concurrency at MVP — one active pipeline run per workspace.

### Non-Goals (MVP)

- Ollama / LM Studio support (deferred to v2 as chat-only backends).
- Linear ticket source (deferred; abstraction ready).
- Parallel/concurrent pipeline runs (deferred to v2).
- Alternate pipelines via `pipeline.toml` (parser ships, only `default` pipeline used at MVP).
- Telemetry, crash reporting, usage stats.
- Cloud sync / cross-device state.
- Multi-tenant / team-shared server.
- Desktop notifications (shipping with none; opt-in in v2).

---

## 3. Target Users

- **Primary**: Solo and small-team developers using an agentic orchestration pipeline backed by Claude Code or Copilot CLI.
- **Enterprise-friendly**: Developers on GitHub Enterprise Server or self-hosted GitLab. Auth flow accommodates enterprise SSO via browser (user is already logged into GHE/GitLab in their browser).
- **Cross-platform**: macOS, Windows, Linux; zsh / bash / fish / PowerShell — no shell-specific assumptions.

---

## 4. Glossary

| Term | Meaning |
|---|---|
| **Workspace** | A repo the user is working on. 1 workspace = 1 repo. |
| **Profile** | A canonical combination of repo host + ticket source + backend. Two shipped: GitHub Profile and GitLab Profile. |
| **Backend** | An LLM runner/agent executor. Claude Code or Copilot CLI at MVP. |
| **Pipeline** | The default agent sequence: `architect → tdd-developer → qa → reviewer`. |
| **Run** | One execution of a pipeline, start to finish. |
| **Step** | One agent's execution within a run. |
| **Ticket source** | Where a run's task description comes from: GitHub Issues, GitLab Issues, Jira, or free-text. |
| **Event stream** | The normalized protocol between core and UIs for streaming run/step output. |

---

## 5. High-Level Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         Agentic Core                            │
│                    (Rust, shared embedded lib)                  │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Pipeline    │  │  Event Bus   │  │  Workspace/Settings  │  │
│  │  State       │  │  (broadcast) │  │  Resolver            │  │
│  │  Machine     │  │              │  │                      │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
│         │                 │                      │              │
│  ┌──────▼──────┐  ┌───────▼──────┐  ┌────────────▼───────────┐ │
│  │  Backend    │  │  Ticket      │  │  SQLite DB             │ │
│  │  Adapters   │  │  Source      │  │  (single global)       │ │
│  │  (trait)    │  │  Adapters    │  │                        │ │
│  │             │  │  (trait)     │  │  workspaces, runs,     │ │
│  │  - Claude   │  │              │  │  run_steps, findings,  │ │
│  │  - Copilot  │  │  - GitHub    │  │  chat_*, stream_events │ │
│  │             │  │  - GitLab    │  │                        │ │
│  │             │  │  - Jira      │  │                        │ │
│  │             │  │  - FreeText  │  │                        │ │
│  └──────┬──────┘  └───────┬──────┘  └────────────────────────┘ │
└─────────┼─────────────────┼────────────────────────────────────┘
          │                 │
          ▼                 ▼
  ┌───────────────┐   ┌────────────┐   ┌─────────────────┐
  │ claude CLI    │   │ GitHub API │   │  OS Keychain    │
  │ copilot CLI   │   │ GitLab API │   │  (via keyring)  │
  │               │   │ Jira API   │   │                 │
  └───────────────┘   └────────────┘   └─────────────────┘
          ▲
          │
  ┌───────┴────────────────────────────────────────────┐
  │                   Shell Adapters                    │
  │  ┌─────────────┐  ┌─────────┐  ┌─────────────────┐ │
  │  │   Tauri     │  │   TUI   │  │  VS Code ext    │ │
  │  │  (native)   │  │ (ratatui│  │  (napi-rs or    │ │
  │  │             │  │ crossterm│  │  WASM bridge)   │ │
  │  └─────────────┘  └─────────┘  └─────────────────┘ │
  └────────────────────────────────────────────────────┘
```

### Core Responsibilities

1. **Pipeline state machine** — owns the `architect → tdd → qa → reviewer` sequencing; not delegated to the backend.
2. **Backend adapters** — normalize `claude` / `copilot` subprocess output into the structured event stream.
3. **Ticket source adapters** — fetch ticket metadata (GitHub, GitLab, Jira, FreeText).
4. **Auth + settings + secrets** — OAuth PKCE loopback flows, three-level settings resolution, keychain access.
5. **Persistence** — SQLite at `$DATA_DIR/agentic/state.db`.
6. **Event bus** — broadcasts events to all subscribed shells; each shell renders at its native fidelity.

### Shell Responsibilities

Shells are thin event consumers + input dispatchers. They don't hold pipeline state, don't parse backend output, don't manage secrets directly.

---

## 6. Tech Stack

### Core

- **Language**: Rust (edition 2024).
- **Async runtime**: `tokio`.
- **DB**: `rusqlite` (bundled) for SQLite, `sqlx` rejected to avoid build-time DB schema requirements.
- **Serialization**: `serde` + `serde_json` (events), `toml` (config).
- **HTTP**: `reqwest` (async) for OAuth + ticket sources.
- **Process**: `tokio::process` for subprocess spawning + stream piping.
- **Secrets**: `keyring` crate (macOS Keychain, libsecret, Windows Credential Manager).
- **Paths**: `directories` crate (XDG / macOS / Windows standard dirs).
- **Event bus**: `tokio::sync::broadcast`.
- **Error**: `thiserror` for library errors, `anyhow` for application-level.
- **Logging**: `tracing` + `tracing-subscriber`.

### Tauri Shell

- **Tauri 2.x**, with `tauri-plugin-updater` for auto-updates.
- **Frontend**: TypeScript + React (or Svelte; see §21).
- **Markdown**: `marked` or `react-markdown` with `remark-gfm`.
- **Syntax highlighting**: `shiki`.
- **Diff viewer**: Monaco's diff editor (embedded) or `@git-diff-view/react`.
- **Styling**: Tailwind CSS + shadcn/ui-style primitives.

### TUI Shell

- **`ratatui`** for widgets + layout.
- **`crossterm`** for terminal backend (cross-platform).
- **Markdown**: `termimad`.
- **Syntax highlighting**: `syntect`.
- **Diff rendering**: inline unified diff with `+`/`-` highlighting (no side-by-side).

### VS Code Extension

- **TypeScript** + VS Code Extension API.
- **Rust core consumed via** `napi-rs` (N-API native bindings) as the primary path; WASM as a fallback for environments where native binaries cause friction.
- **Chat + cockpit**: webview, reusing the Tauri frontend build as much as possible.
- **Diffs + findings**: native VS Code diff editor + `languages.registerCodeLensProvider` for findings.
- **Notifications**: VS Code's `window.showInformationMessage` / `showWarningMessage`.

### Distribution

- **Monorepo**: `cargo` workspace + `pnpm` workspace for frontend + TS extension.
- **Build**: GitHub Actions matrix (macOS arm64/x64, Windows x64, Linux x64/arm64).
- **Signing**: Apple Developer ID + notarization (macOS); EV code signing cert (Windows).
- **Channels**:
  - Tauri: `.dmg` / `.msi` / `.exe` / AppImage / `.deb` via GitHub Releases; `tauri-plugin-updater` for incremental updates.
  - TUI: `cargo install agentic-tui` (crates.io), Homebrew tap, `winget`, later `apt`/`dnf`.
  - VS Code extension: VS Code Marketplace + Open VSX.

---

## 7. Form Factors

### 7.1 Tauri Desktop

- Two-column layout: **chat (left, ~40%)** | **cockpit (right, ~60%)**.
- Resizable divider, user-persisted ratio.
- Top bar: workspace picker, profile badge, backend + model picker.
- Minimum window size: 1024×640. Below that, cockpit collapses to a toggle.
- Native OS menus (macOS: menu bar; Windows/Linux: in-window menu).
- System tray icon (v2; not MVP).

### 7.2 TUI

- Two vertical panes (default 50/50, `[` / `]` to resize, `Tab` to switch focus).
- Left pane: chat (scrollback + input at bottom).
- Right pane: cockpit (stepper + per-step details).
- Top status bar: workspace name, profile, backend:model, active-run status.
- Bottom command bar: current mode + key hints.
- Modes: `normal` (vim-like nav), `insert` (typing), `command` (`:plan PROJ-123`, `:status`, `:q`).
- Keyboard-only by design; mouse optional (`crossterm` mouse events for click/scroll).
- Runs full-screen (alternate screen buffer) by default; `-n` flag for inline mode (minimal, for piping).

### 7.3 VS Code Extension

- **Activity bar**: Agentic icon opens the sidebar view.
- **Sidebar view (primary)**: chat (native input + webview message list) + collapsible "Runs" tree below.
- **Main editor area**: cockpit webview (opens via `Agentic: Show Cockpit` command or clicking a run in the sidebar).
- **Command contributions**: every slash command registered as a VS Code command (`Agentic: Plan…`, `Agentic: Status`, etc.) — accessible via VS Code's `Cmd+Shift+P`.
- **Native file diffs**: when the stream emits `file_change`, open in VS Code's native diff editor (`vscode.diff` command).
- **Findings → editor decorations**: reviewer findings render as editor decorations (squiggles + hover tooltip with `[Fix] [Tech-debt] [Ignore]` actions) on the affected files.
- **Settings UI**: VS Code Settings page (`agentic.*` keys) for shell-specific preferences, with a deep link to open the Tauri/TUI-style advanced settings panel (in a webview) for shared Rust-core settings.
- **Single active extension instance per VS Code window**. Opening a second VS Code window starts a second independent Rust core process.

---

## 8. Profiles

Profiles are first-class settings objects. Two are pre-configured; any other combination shows as "Custom".

### 8.1 GitHub Profile

| Dimension | Value |
|---|---|
| Repo host | GitHub (github.com or GHES) |
| Ticket source | GitHub Issues |
| Backend | Claude Code |
| PR output | Pull Request |
| Auth | PKCE loopback against the configured host |

### 8.2 GitLab Profile

| Dimension | Value |
|---|---|
| Repo host | GitLab (gitlab.com or self-hosted) |
| Ticket source | Jira |
| Backend | Copilot CLI (with model choice) |
| PR output | Merge Request |
| Auth | PKCE loopback against GitLab + separate Jira auth |

### 8.3 Profile Auto-Detection

On workspace open:

1. Read `git remote get-url origin`.
2. Parse host.
3. If host matches `github.com` or a configured GHES host → suggest **GitHub Profile**.
4. If host matches `gitlab.com` or a configured self-hosted GitLab host → suggest **GitLab Profile**.
5. Show a one-time dialog: "Detected a GitHub/GitLab repo — apply the GitHub/GitLab profile?" with `[Apply] [Customize] [Not now]`.
6. User choice is saved to `<repo>/.agentic/config.toml`.

### 8.4 Custom Profile

Free combination of any repo host, ticket source, backend. Off the happy path; the UI shows a subtle "Custom configuration" indicator with a tooltip explaining that auto-behaviors (ticket ID inference, branch naming) fall back to generic defaults.

---

## 9. Core Concepts

### 9.1 Workspace

- 1 workspace ↔ 1 repo.
- Identified by a stable `workspace_id`: `blake3(canonical_remote_url || canonical_path)[..16]`.
- If a repo is moved on disk, matching falls back to `canonical_remote_url`. User is prompted to re-bind.
- Workspace list is persisted; UI shows "Recent workspaces" (like VS Code).
- Only one workspace visible per app window. Open a second window to work on two workspaces simultaneously.

### 9.2 Run

- A single execution of a pipeline.
- States: `pending → running → (completed | completed_with_tech_debt | failed | cancelled | crashed)`.
- At most **one run in `running` state per workspace** (serial concurrency).
- Attempting to start a new run while one is active prompts "A run is already in progress. Cancel it?"

### 9.3 Step

- A single agent's execution within a run.
- States: `pending → running → (passed | failed | needs_triage | skipped)`.
- Each step has structured outputs: streaming text, tool calls, file changes, findings (reviewer only), clarifying questions (architect only).

### 9.4 Pipeline

- Default sequence: `architect → tdd-developer → qa → reviewer`.
- Defined in `<repo>/.agentic/pipeline.toml` if present (alternate pipelines like `hotfix` — infrastructure ready at MVP, feature-gated off).
- The **QA fix-loop** matches existing behavior: up to 3 retry loops of `tdd-developer ↔ qa`; on the 4th failure, remaining issues move to tech-debt and the run enters `completed_with_tech_debt`.

### 9.5 Backend

- An executor that takes an agent prompt + context and produces a stream of events.
- Implements the `Backend` trait (§11).

### 9.6 Ticket Source

- Provides ticket metadata (title, description, acceptance criteria, comments) given an identifier.
- Implements the `TicketSource` trait.

---

## 10. Pipeline Model

### 10.1 State Machine

```
                    ┌──────────────────────────────────────┐
                    │                                       │
   ┌─────────┐      ▼      ┌────────────┐   ┌─────┐   ┌────────┐
   │ pending │──start─→ running ────→ │ architect │─→ │ tdd │─→ │   qa   │
   └─────────┘              └───┬────┘      │ -developer │    └─────┬──┘
                                │           └────────────┘          │
                                │                                    │ fail &
                                │                                    │ loops<3
                                │                    ┌───────────────┘
                                ▼                    ▼
                          ┌──────────┐         ┌────────────┐
                          │ cancelled│         │ tdd-dev    │
                          └──────────┘         │ (retry)    │
                                               └──────┬─────┘
                                                      │
                                    loops≥3 → tech-debt, continue to reviewer
                                                      │
                                                      ▼
                                               ┌──────────┐
                                               │ reviewer │
                                               └──────┬───┘
                                                      │
                                                      ▼
                                     ┌────────────────┴───────────────┐
                                     ▼                                 ▼
                              ┌────────────┐                 ┌──────────────────────┐
                              │ completed  │                 │completed_with_tech_debt│
                              └────────────┘                 └──────────────────────┘
```

Transitions happen via events on the bus; the state machine subscribes and mutates SQLite + rebroadcasts normalized state-change events.

### 10.2 Agent Discovery

Search order (first match wins for each agent name):

1. `<repo>/.agentic/agents/<name>.md` — primary, Agentic's namespace.
2. `<repo>/.claude/agents/<name>.md` — fallback to Claude Code's convention.
3. `<repo>/agents/<name>.md` — legacy fallback (this repo's current layout).

### 10.3 Agent Frontmatter Schema

```toml
+++
name = "architect"                        # required, must match filename stem
description = "Designs feature spec…"     # required, one-line
model = "claude-opus-4-7"                 # optional; default = profile's default model
tools = ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"]
                                          # optional; default = all
allowed_questions = 5                     # optional; architect-only semantic
pipeline_role = "step"                    # one of: step | mention | both
                                          # step = participates in default pipeline
                                          # mention = @-only; not in default pipeline
                                          # both = step + @-mentionable outside pipeline
timeout_seconds = 1800                    # optional; overrides per-step timeout
+++
<markdown body = system prompt>
```

Schema is parsed with the `toml` crate; unknown fields are preserved but ignored (forward-compat). Frontmatter uses `+++` fences (TOML convention) rather than the YAML `---`. Rationale: the Rust YAML crates flagged at Dependabot audit time (`serde_yaml`, `serde_yml`) are unmaintained; TOML is already a core dependency of the pipeline config parser and avoids that maintenance risk.

### 10.4 Pipeline TOML Schema

```toml
# <repo>/.agentic/pipeline.toml
# Optional. If absent, the default pipeline is used:
#   architect → tdd-developer → qa → reviewer

[pipelines.default]
steps = [
  { agent = "architect",      stop_on_failure = true,  allowed_questions = 5 },
  { agent = "tdd-developer",  stop_on_failure = true,  qa_fix_loop_cap = 3 },
  { agent = "qa",             stop_on_failure = false },
  { agent = "reviewer",       stop_on_failure = false },
]

[pipelines.hotfix]                        # MVP parses, doesn't expose in UI
steps = [
  { agent = "troubleshooter", stop_on_failure = true },
  { agent = "tdd-developer",  stop_on_failure = true,  qa_fix_loop_cap = 1 },
  { agent = "qa",             stop_on_failure = false },
]
```

At MVP, only `default` is exposed; the `[pipelines.<name>]` table parses so v2 can enable `/hotfix` and similar without a schema change.

---

## 11. Backends

### 11.1 Backend Trait

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    fn id(&self) -> BackendId;             // "claude-code" | "copilot-cli"
    fn display_name(&self) -> &str;
    fn supported_models(&self) -> Vec<ModelId>;
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Executes a single agent prompt and streams events.
    async fn execute(
        &self,
        req: ExecuteRequest,
        event_sink: EventSink,
    ) -> Result<ExecuteOutcome>;
}

pub struct ExecuteRequest {
    pub workspace: WorkspaceRef,
    pub run_id: RunId,
    pub step_id: StepId,
    pub agent_name: String,
    pub agent_prompt: String,                // markdown body from agent file
    pub user_context: String,                // ticket body, free-text, or preceding step output
    pub model: Option<ModelId>,
    pub tools: Vec<ToolName>,                // allowed tools (whitelist)
    pub cwd: PathBuf,                        // repo root
    pub timeout: Option<Duration>,
    pub cancel: CancellationToken,
}

pub struct ExecuteOutcome {
    pub status: StepStatus,                  // passed | failed | needs_triage | skipped
    pub summary: String,                     // 1-2 sentence summary from the agent
    pub token_usage: TokenUsage,
    pub cost_usd: Option<f64>,
}
```

### 11.2 Claude Code Adapter

- **Invocation**: `claude -p --output-format stream-json [--model <id>] [--allowed-tools …] --append-system-prompt <file>`.
- **Input**: user context piped via stdin.
- **Output parser**: line-delimited JSON; each line is a Claude Agent SDK event (`message_start`, `content_block_delta`, `tool_use`, `tool_result`, `message_delta`, `message_stop`, `error`). Translated 1:1 (mostly) into the core's event enum.
- **File changes**: tracked via `Edit` / `Write` tool calls; the core diffs file snapshots before/after per step to produce a clean `diff.patch` per run.
- **Cost**: computed from `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens` using the model's current pricing table (bundled, versioned).

### 11.3 Copilot CLI Adapter

- **Invocation**: `copilot --model <id> --no-interactive` (or equivalent; verify exact flags against current `copilot` CLI at implementation time).
- **Input**: prompt piped via stdin.
- **Output parser**: Copilot CLI's JSON stream (verify current schema). Translated into the core's event enum.
- **Model choice**: whatever Copilot CLI supports at build time — `gpt-5`, `claude-sonnet-4-6`, `claude-opus-4-7`, etc. List is fetched at runtime via `copilot models list` (if supported) or falls back to a bundled list per Copilot version.
- **Tool use**: Copilot's native tool-use protocol; normalized into the core's `tool_use_*` events.
- **File changes**: same diffing approach as Claude adapter — snapshot before/after.

### 11.4 Adapter Error Mapping

| Native signal | Core event |
|---|---|
| Non-zero exit + `SIGKILL`/`SIGTERM` | `error { recoverable: true, code: "subprocess_killed" }` |
| Non-zero exit, no signal | `error { recoverable: false, code: "subprocess_failed", stderr }` |
| HTTP 429 (rate limit) | `error { recoverable: true, code: "rate_limited", retry_after }` |
| HTTP 5xx | `error { recoverable: true, code: "upstream_5xx" }` |
| Auth failure (401/403) | `error { recoverable: false, code: "auth_failed" }` — prompts user to re-auth |
| Timeout (core-enforced) | `error { recoverable: false, code: "timeout" }` |
| Parse error (bad JSON from backend) | `error { recoverable: false, code: "protocol_error" }` |

### 11.5 Deferred Backends (v2)

- **Ollama** — HTTP at `OLLAMA_HOST` (default `http://localhost:11434`). Chat-only. Tool use best-effort per model.
- **LM Studio** — HTTP at `LMSTUDIO_BASE_URL` (default `http://localhost:1234/v1`). OpenAI-compatible. Chat-only.

Both adapters implement the same `Backend` trait; they're just not registered in the MVP build.

---

## 12. Event Stream Protocol

### 12.1 Event Types

All events carry `{ event_id, run_id, step_id?, timestamp_ms }` plus type-specific payload.

```rust
pub enum Event {
    // Run lifecycle
    RunStarted { ticket: TicketRef, profile: ProfileId, backend: BackendId, model: ModelId },
    RunComplete { status: RunStatus, duration_ms: u64, summary: String },

    // Step lifecycle
    StepStarted { agent: String, model: ModelId },
    StepComplete { status: StepStatus, summary: String, token_usage: TokenUsage, cost_usd: Option<f64> },

    // Streaming content
    TextDelta { content: String },
    ThinkingDelta { content: String },                   // extended-thinking, collapsed by default
    ToolUseStart { tool_call_id: String, tool_name: String, input: serde_json::Value },
    ToolUseDelta { tool_call_id: String, stream: ToolStream, content: String },  // stdout | stderr
    ToolUseEnd { tool_call_id: String, exit_code: Option<i32>, duration_ms: u64 },

    // Artifacts
    FileChange { path: PathBuf, before_hash: String, after_hash: String },
    Finding {
        finding_id: String,
        severity: Severity,               // error | warning | info
        file: Option<PathBuf>,
        line: Option<u32>,
        message: String,
        suggestion: Option<String>,
    },
    ClarifyingQuestion {
        question_id: String,
        question: String,
        suggested_answers: Vec<String>,   // may be empty
    },

    // Control flow
    RetryStarted { attempt: u32, reason: String },
    Error { code: String, message: String, recoverable: bool, retry_after_ms: Option<u64> },
    UserActionNeeded { action: ActionRequired },  // triage findings, answer questions, etc.
}

pub enum ToolStream { Stdout, Stderr }

pub enum ActionRequired {
    AnswerClarifyingQuestions { question_ids: Vec<String> },
    TriageFindings { finding_ids: Vec<String> },
    QaRetryDecision,
}
```

### 12.2 Transport

- **In-process** (Tauri + TUI): `tokio::sync::broadcast::Sender<Event>`. Each subscriber gets a `Receiver`.
- **VS Code extension**: events cross the napi-rs boundary. Native module exposes an async iterator (`for await (const evt of core.events())`) backed by the broadcast channel.
- **Persistence**: every event is appended to `stream_events` table with `(run_id, step_id, seq)` as the ordering key. Enables cockpit replay for past runs.

### 12.3 Rendering Per Shell

| Event | Tauri | TUI | VS Code |
|---|---|---|---|
| `TextDelta` | `react-markdown` + `shiki` | `termimad` + `syntect` | same as Tauri (webview) |
| `ThinkingDelta` | Collapsed block, expandable | Collapsed, `z` to expand | Same as Tauri |
| `ToolUseStart/Delta/End` | Collapsible card with icon | Boxed section, header | Same as Tauri |
| `FileChange` | Monaco diff inline | Unified diff, scrollable | **Native VS Code diff editor** |
| `Finding` | Row in findings table | Row in findings table | **Editor decoration** + findings table |
| `ClarifyingQuestion` | Form in cockpit | Modal-style prompt | Webview form in cockpit panel |
| `Error` | Toast + inline callout | Inline callout (red) | Toast via `window.showErrorMessage` |
| `UserActionNeeded` | Banner at top of cockpit | Status bar indicator | Sidebar badge |

### 12.4 Token / Cost Display

- Every `StepComplete` event carries `token_usage` and `cost_usd`.
- Cockpit renders per-step: `🟢 passed · 12.4s · 2,341 in / 487 out · $0.038`.
- Run total shown at bottom of cockpit: `Total: $0.127 · 18,023 tokens · 2m 14s`.

---

## 13. Data Model

### 13.1 SQLite Schema

Single DB at `$DATA_DIR/agentic/state.db`. Migrations managed via `refinery` or a hand-rolled migrator.

```sql
CREATE TABLE workspaces (
    id            TEXT PRIMARY KEY,        -- workspace_id (blake3 hash prefix)
    name          TEXT NOT NULL,           -- last folder component
    root_path     TEXT NOT NULL,           -- last known canonical path
    remote_url    TEXT,                    -- canonical git remote
    profile       TEXT NOT NULL,           -- 'github' | 'gitlab' | 'custom'
    created_at    INTEGER NOT NULL,
    last_opened   INTEGER NOT NULL
);

CREATE TABLE runs (
    id             TEXT PRIMARY KEY,       -- ulid
    workspace_id   TEXT NOT NULL REFERENCES workspaces(id),
    pipeline_name  TEXT NOT NULL DEFAULT 'default',
    status         TEXT NOT NULL,          -- pending | running | completed | completed_with_tech_debt | failed | cancelled | crashed
    ticket_type    TEXT,                   -- 'github-issue' | 'gitlab-issue' | 'jira' | 'free-text'
    ticket_ref     TEXT,                   -- #42, PROJ-123, or free-text hash
    ticket_title   TEXT,
    ticket_body    TEXT,                   -- snapshotted at run start
    backend        TEXT NOT NULL,          -- 'claude-code' | 'copilot-cli'
    model          TEXT NOT NULL,
    started_at     INTEGER NOT NULL,
    completed_at   INTEGER,
    duration_ms    INTEGER,
    token_usage    TEXT,                   -- json
    cost_usd       REAL,
    summary        TEXT,
    subprocess_pid INTEGER                 -- for crash detection
);

CREATE INDEX idx_runs_workspace_status ON runs(workspace_id, status);
CREATE INDEX idx_runs_started_at       ON runs(started_at DESC);

CREATE TABLE run_steps (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq          INTEGER NOT NULL,
    agent_name   TEXT NOT NULL,
    status       TEXT NOT NULL,           -- pending | running | passed | failed | needs_triage | skipped
    started_at   INTEGER,
    completed_at INTEGER,
    duration_ms  INTEGER,
    token_usage  TEXT,                    -- json
    cost_usd     REAL,
    summary      TEXT,
    retry_count  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_run_steps_run_seq ON run_steps(run_id, seq);

CREATE TABLE findings (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id      TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    severity     TEXT NOT NULL,
    file_path    TEXT,
    line         INTEGER,
    message      TEXT NOT NULL,
    suggestion   TEXT,
    triage       TEXT,                     -- null | 'fix' | 'tech-debt' | 'ignore'
    triaged_at   INTEGER,
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_findings_run_triage ON findings(run_id, triage);

CREATE TABLE clarifying_questions (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id         TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    question        TEXT NOT NULL,
    suggested_answers TEXT,                -- json array
    answer          TEXT,
    answered_at     INTEGER,
    created_at      INTEGER NOT NULL
);

CREATE TABLE file_changes (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id      TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    path         TEXT NOT NULL,
    before_hash  TEXT,
    after_hash   TEXT,
    diff         BLOB,                     -- unified diff patch
    created_at   INTEGER NOT NULL
);

-- Event log (full replay fidelity)
CREATE TABLE stream_events (
    run_id       TEXT NOT NULL,
    step_id      TEXT,
    seq          INTEGER NOT NULL,
    event_type   TEXT NOT NULL,
    payload      BLOB NOT NULL,            -- MessagePack-encoded Event
    timestamp_ms INTEGER NOT NULL,
    PRIMARY KEY (run_id, seq)
);

CREATE INDEX idx_stream_events_step ON stream_events(step_id, seq);

CREATE TABLE chat_sessions (
    id            TEXT PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces(id),
    title         TEXT,
    created_at    INTEGER NOT NULL,
    last_message_at INTEGER
);

CREATE TABLE chat_messages (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    run_id       TEXT REFERENCES runs(id),           -- null if pure chat
    role         TEXT NOT NULL,                      -- user | assistant | system | tool
    content      TEXT NOT NULL,                      -- markdown body
    metadata     TEXT,                               -- json (tool calls, citations, etc.)
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_chat_messages_session_ts ON chat_messages(session_id, created_at);

CREATE TABLE auth_accounts (
    id             TEXT PRIMARY KEY,                 -- e.g., 'github:github.com', 'gitlab:gitlab.com', 'jira:mycompany.atlassian.net'
    provider       TEXT NOT NULL,                    -- github | gitlab | jira | claude | copilot
    host           TEXT NOT NULL,
    username       TEXT,
    client_id      TEXT,                             -- for GHES BYO client ID
    -- Secrets: tokens stored in keychain keyed by this id; not in DB.
    token_expires_at INTEGER,
    created_at     INTEGER NOT NULL,
    last_used_at   INTEGER
);

CREATE TABLE settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL,                           -- json
    scope   TEXT NOT NULL,                           -- 'user' | 'workspace:<id>'
    updated_at INTEGER NOT NULL
);
```

### 13.2 File-System Layout

```
$CONFIG_DIR/agentic/
  settings.toml                  # user-global non-secret settings

$DATA_DIR/agentic/
  state.db                       # single SQLite
  logs/
    agentic-<date>.log           # tracing output, rotated daily

<repo-root>/.agentic/
  config.toml                    # workspace settings (committable for team defaults)
  pipeline.toml                  # optional alternate pipeline definitions
  agents/                        # optional per-workspace agent overrides
    <agent-name>.md
  .gitignore                     # excludes state.* and cache/; allows config.toml

OS Keychain:
  service = "agentic"
  account = <auth_account_id>    # e.g., "github:github.com"
  secret  = <access_token | refresh_token blob>
```

### 13.3 Path Resolution

Resolved via the `directories` crate:

| Platform | `$CONFIG_DIR` | `$DATA_DIR` |
|---|---|---|
| macOS | `~/Library/Application Support/agentic/` | `~/Library/Application Support/agentic/` |
| Linux | `~/.config/agentic/` (XDG_CONFIG_HOME) | `~/.local/share/agentic/` (XDG_DATA_HOME) |
| Windows | `%APPDATA%\agentic\` | `%APPDATA%\agentic\` |

---

## 14. Settings Resolution

### 14.1 Three-Level Hierarchy

Resolution order (first match wins for each setting):

1. **Environment variables** — runtime overrides.
2. **Workspace settings** — `<repo>/.agentic/config.toml`.
3. **User-global settings** — `$CONFIG_DIR/agentic/settings.toml`.
4. **Built-in defaults** — compiled into the binary.

The UI settings panel displays the **source** of each value (`env: GITHUB_TOKEN`, `workspace`, `user`, or `default`) so resolution is never mysterious.

### 14.2 Environment Variables

Auto-detected:

| Variable | Maps to |
|---|---|
| `GITHUB_TOKEN` / `GH_TOKEN` | GitHub auth token (primary) |
| `GITLAB_TOKEN` | GitLab auth token |
| `JIRA_API_TOKEN` | Jira auth token (Atlassian) |
| `ANTHROPIC_API_KEY` | Reserved (not used at MVP since we go through `claude` CLI) |
| `OLLAMA_HOST` | Ollama endpoint (v2) |
| `LMSTUDIO_BASE_URL` | LM Studio endpoint (v2) |
| `AGENTIC_BACKEND` | Default backend for this shell session |
| `AGENTIC_MODEL` | Default model for this shell session |
| `AGENTIC_PROFILE` | Force a profile (`github` / `gitlab` / `custom`) |
| `CLAUDE_CODE_BIN` | Override path to `claude` CLI |
| `COPILOT_BIN` | Override path to `copilot` CLI |
| `AGENTIC_LOG` | Tracing filter string (e.g., `agentic_core=debug`) |

### 14.3 User-Global Settings (`settings.toml`)

```toml
# ~/Library/Application Support/agentic/settings.toml (macOS example)

[ui]
theme = "auto"                  # auto | light | dark
font_size = 14
cockpit_ratio = 0.6

[defaults]
profile = "github"              # fallback profile for workspaces without an explicit config
backend = "claude-code"
model = "claude-opus-4-7"
allowed_questions = 5           # default for architect
qa_fix_loop_cap = 3

[auth.github]
host = "github.com"
client_id = "<baked-in-public-client-id>"

[auth.github_enterprise]
# Populated when the user connects to a GHES host for the first time.
# Multiple entries allowed, keyed by host.
# hosts."github.mycorp.com" = { client_id = "…" }

[auth.gitlab]
host = "gitlab.com"
client_id = "<baked-in-public-client-id>"

[auth.jira]
# Populated on first connect.
# instances."mycompany.atlassian.net" = { email = "…" }

[notifications]
enabled = false                 # MVP: always false; v2: per-event toggles
```

### 14.4 Workspace Settings (`<repo>/.agentic/config.toml`)

```toml
# Overrides user-global settings for this workspace only.
# May be committed to git to share team defaults.

[profile]
kind = "github"                 # github | gitlab | custom

[backend]
id = "claude-code"              # for gitlab profile, typically "copilot-cli"
model = "claude-opus-4-7"       # optional

[repo]
host = "github.com"             # or "github.mycorp.com" for GHES
# For GitLab: host = "gitlab.com" or "gitlab.mycorp.com"

[ticket_source]
kind = "github-issue"           # github-issue | gitlab-issue | jira | free-text
# For Jira:
# jira.instance_url = "https://mycompany.atlassian.net"
# jira.project_key = "PROJ"

[pipeline]
# Overrides default pipeline args
allowed_questions = 3
qa_fix_loop_cap = 3
```

---

## 15. Authentication

### 15.1 OAuth 2.0 Authorization Code + PKCE + Loopback

Primary flow for GitHub (incl. GHES) and GitLab (incl. self-hosted):

1. User clicks **Sign in with GitHub / GitLab**.
2. Core generates PKCE `code_verifier` (random 43-128 char string) and `code_challenge = S256(code_verifier)`; generates `state` (CSRF token).
3. Core spins up an ephemeral HTTP listener on a random loopback port (`127.0.0.1:<random>`).
4. Core opens the user's default browser to:
   ```
   https://<host>/login/oauth/authorize
     ?client_id=<client_id>
     &redirect_uri=http://127.0.0.1:<port>/callback
     &scope=<space-separated>
     &state=<csrf>
     &code_challenge=<challenge>
     &code_challenge_method=S256
   ```
5. User (already logged in to GHE/GitLab in browser) sees a one-click "Authorize Agentic" screen → approves.
6. Browser redirects to `http://127.0.0.1:<port>/callback?code=<code>&state=<state>`.
7. Core's listener validates `state`, exchanges `code` + `code_verifier` for access token:
   ```
   POST https://<host>/login/oauth/access_token
   Content-Type: application/x-www-form-urlencoded
   client_id=<id>&code=<code>&code_verifier=<verifier>&redirect_uri=<redir>
   ```
8. Access + refresh tokens stored in OS keychain keyed by `auth_account_id`.
9. Listener shuts down; serves a "You can close this tab" HTML page back to the browser.

### 15.2 Requested Scopes

| Provider | Scopes |
|---|---|
| GitHub | `repo`, `read:org`, `read:user`, `copilot` (when in GitHub profile) |
| GitLab | `api`, `read_user`, `read_repository`, `write_repository` |
| Jira | `read:jira-work`, `read:jira-user`, `write:jira-work`, `offline_access` |

### 15.3 Per-Provider Tailoring

- **GitHub (github.com)**: PKCE loopback, published public client ID.
- **GitHub Enterprise Server**: on first connect to a new host, dialog offers:
  - **(i)** Register our OAuth App — shows a downloadable manifest (JSON) to give to GHES admin; once registered, user pastes the client ID.
  - **(ii)** Bring-your-own — user self-registers an OAuth App on GHES (if allowed by org policy), pastes client ID.
  - **(iii)** PAT paste — fallback for orgs that disable custom OAuth Apps.
  - Stored per-host in `auth.github_enterprise.hosts.<host>.client_id` (non-secret).
- **GitLab (gitlab.com)**: PKCE loopback, published public client ID (registered as a GitLab Application).
- **Self-hosted GitLab**: same three-option dialog as GHES.
- **Jira Cloud**: Atlassian OAuth 2.0 (3LO) with PKCE. Rotating refresh tokens. Same loopback mechanism.
- **Jira Server / Data Center**: PAT paste only (OAuth is Jira-Cloud-only and enterprise Jira Server PAT is the accepted path).
- **Claude Code**: we don't manage Anthropic auth — we shell out to `claude`; if `claude login` hasn't been run, we surface a "Please run `claude login`" inline message with a copy-to-clipboard button.
- **Copilot CLI**: piggybacks on GitHub auth. Verified via `gh copilot --version` or `copilot --version` succeeding after GitHub sign-in.

### 15.4 Always-On Fallbacks

- **Delegate to existing CLI session**: on startup, core silently checks `gh auth status` / `glab auth status` / `claude -v`. If valid sessions are detected and no stored OAuth token exists for that host, user is offered "Use your existing `gh` session?" as a one-click skip of the OAuth flow. Token extracted via `gh auth token` and stored in keychain.
- **Env var override**: any `*_TOKEN` env var silently overrides the stored token at resolution time. UI shows `Source: env: GITHUB_TOKEN`.

### 15.5 Headless / SSH TUI

If the loopback flow fails (common cause: no `$DISPLAY`, or running over SSH without X-forwarding):

- Core detects inability to open a browser (via `open` / `xdg-open` / `start` failing or explicit `--headless` flag).
- Falls back to **OAuth Device Code flow**:
  - Display `ABCD-1234` and `https://<host>/login/device`.
  - Poll `POST /login/oauth/access_token` with `grant_type=urn:ietf:params:oauth:grant-type:device_code` until authorized or timeout.
- Same token storage as loopback path.

### 15.6 Token Refresh

- Core tracks `token_expires_at` in `auth_accounts`.
- Background task refreshes tokens within 5 minutes of expiry (where refresh tokens are available).
- On refresh failure or irreversible 401, marks account as `needs_reauth` and surfaces a banner.

---

## 16. UX

### 16.1 Layout

Two-column (chat + cockpit) as established in §7.

### 16.2 Action Anchors

Smart defaults per action:

| Action | Anchor | Rationale |
|---|---|---|
| Start new run (`/plan`) | Chat input | Natural-language entry |
| Answer clarifying question | Cockpit form | Structured input |
| Triage finding | Cockpit button row | Bulk-friendly |
| QA retry/skip decision | Cockpit buttons | Structured choice |
| Freeform discussion | Chat | Conversational |
| `/pr` after completion | Chat command or cockpit CTA | Both; single handler |
| Switch backend / model | Top bar picker | Global state |
| Open settings | Menu + `Cmd-,` | Standard |
| Cancel run | Cockpit "Cancel" button + `/cancel` | Both |

### 16.3 Chat Intent Routing

- `/<word>` at message start → **slash command** (deterministic).
- `@<name>` at message start → **agent mention** (deterministic, routes to `Backend.execute()` with agent prompt + message body).
- Anything else → **LLM tool-call fallthrough**:
  - Send to the selected backend with a tool schema containing: `run_pipeline`, `get_status`, `mention_agent`, `triage_finding`, `answer_question`, `create_pr`.
  - If the model calls a tool, the core routes to the appropriate handler; otherwise, the model's text response renders as a chat message.

### 16.4 Slash Commands (MVP Grammar)

| Command | Arguments | Behavior |
|---|---|---|
| `/plan` | `<ticket>` | Start default pipeline. `<ticket>` is `PROJ-123` (Jira), `#42` (GH/GL), or `"free text"` |
| `/hotfix` | `<ticket>` | Alternate pipeline (v2 content; MVP ships disabled, command hidden) |
| `/status` | — | Render current run state in cockpit |
| `/cancel` | — | Cancel current run, leave files as-is |
| `/triage` | `<finding-id> <fix\|tech-debt\|ignore>` | Triage a finding |
| `/answer` | `<question-id> <text>` | Respond to architect clarifying question |
| `/retry` | `[step]` | Retry failed step (defaults to last failed) |
| `/resume` | — | Resume a crashed run |
| `/workspace` | `[open\|switch\|close] <path?>` | Workspace management |
| `/backend` | `<claude\|copilot>` | Set backend for next run (session-scoped) |
| `/model` | `<name>` | Set model for next run (session-scoped) |
| `/settings` | — | Open settings panel |
| `/runs` | `[n]` | Show last N runs (default 10) |
| `/pr` | — | Create PR/MR from current diff using `/pr` skill structure |
| `/clear` | — | Clear current chat (not pipeline state) |
| `/help` | `[command]` | Help |

### 16.5 @mentions

- `@architect`, `@tdd-developer`, `@qa`, `@reviewer`, `@troubleshooter` — built-ins.
- `@<name>` — any agent discovered in `.agentic/agents/`, `.claude/agents/`, or `agents/`.
- Mentioning dispatches a **single** agent run (not the pipeline). Output streams into chat, not cockpit. No state-machine transition.

### 16.6 Keyboard Shortcuts

| Key | Action |
|---|---|
| `Cmd/Ctrl+K` (Tauri/VS Code), `:` (TUI) | Command palette |
| `Cmd/Ctrl+Enter` | Send chat message |
| `Esc` | Close palette / collapse step |
| `Cmd/Ctrl+,` | Open settings |
| `Cmd/Ctrl+L` | Toggle cockpit pane |
| `Cmd/Ctrl+1..9` (Tauri/VS Code), `1..9` in normal mode (TUI) | Switch workspace |
| `Tab` (TUI) | Switch pane focus |
| `[` / `]` (TUI) | Resize panes |
| `j` / `k` (TUI findings table) | Navigate |
| `f` / `t` / `i` (TUI findings table) | Triage fix/tech-debt/ignore |
| `z` (TUI) | Expand/collapse thinking block |

### 16.7 Notifications

- **MVP: disabled**. No OS notifications are emitted by Agentic.
- Design leaves room for (v2) per-event toggles in `[notifications]` settings.

---

## 17. Error Handling & Resilience

### 17.1 Transient vs Fatal

- **Transient**: rate limits (429), network timeouts, 5xx from LLM API, subprocess killed by signal.
  - Automatic retry with exponential backoff: base 1s, factor 2, cap 30s, max 3 attempts.
  - `RetryStarted` event emitted; UI shows "Rate limited, retrying in 8s" inline.
- **Fatal**: auth failures (401/403), protocol errors (bad JSON), tool not found, non-zero exit without signal, timeouts (see §17.4).
  - Mark step as `failed`. UI surfaces a "Retry" or "Abort run" button.
  - `Error { recoverable: false }` event.

### 17.2 QA Fix-Loop

- Max 3 retry loops of `tdd-developer ↔ qa`.
- On 4th QA failure, remaining issues auto-filed to `findings` with `severity=warning` and triage `tech-debt`.
- Run transitions to `completed_with_tech_debt`, reviewer step still runs.

### 17.3 User Cancellation

- Cancel via `/cancel` or the cockpit "Cancel" button or `Ctrl-C` (TUI).
- Send `CancellationToken.cancel()` to the active backend adapter.
- Backend adapter kills the subprocess (SIGTERM, then SIGKILL after 5s).
- **Files are left as-is** — no auto-rollback. User handles with `git checkout` or keeps changes.
- Run transitions to `cancelled`. Event emitted.

### 17.4 Timeouts

- Per-step timeout configurable in `pipeline.toml` or agent frontmatter (`timeout_seconds`).
- **Default: no timeout**. Must be explicitly set.
- On timeout: subprocess killed (SIGTERM → SIGKILL), step marked `failed` with `code: timeout`. Non-recoverable.

### 17.5 Crash Recovery

On app startup:

1. Query `runs WHERE status = 'running'`.
2. For each such run:
   - Check if `subprocess_pid` is alive (`kill -0 pid` / Windows `OpenProcess`).
   - If dead → candidate for crash recovery.
3. If any crashed runs detected, show dialog:
   ```
   We detected an interrupted run from <timestamp>
   on workspace <workspace_name>, step <step_name>.

   [Resume from <step>]   [Start new]   [Discard]
   ```
4. **Resume semantics**: re-run the failed step from scratch (no in-step checkpointing at MVP). Prior step outputs preserved.

### 17.6 DB Consistency

- All multi-table writes wrapped in SQLite transactions.
- `WAL` mode for concurrent reads.
- Crash during a transaction → SQLite rolls back automatically.
- `PRAGMA integrity_check` run on startup if last shutdown was unclean.

---

## 18. Workspace & Concurrency

### 18.1 Workspace Lifecycle

- **Open**: user picks a repo directory. Core computes `workspace_id`. If new, inserts a row in `workspaces`. Updates `last_opened`.
- **Profile detection**: as in §8.3. User's choice saved to `<repo>/.agentic/config.toml`.
- **Switch**: workspace picker in top bar / `Cmd-Shift-P Workspace: Switch`. Closes current workspace's UI state, loads target's.
- **Close**: unloads state from the shell; workspace row retained.
- **Forget**: explicit action removes from `workspaces`. Does not delete run history (retained for cross-workspace queries).

### 18.2 Workspace Identity

- `workspace_id = blake3(canonical(remote_url) || canonical(root_path))[..16]`.
- If remote_url is absent (new repo, no remotes), use canonical path only.
- On move: core detects `root_path` changed for a known `remote_url` → offers "Re-bind workspace from `<old>` to `<new>`?".

### 18.3 Concurrency

- **Within a workspace**: strictly one active run. Starting a second prompts to cancel the first.
- **Across workspaces**: independent (no constraint).
- **Across windows**: independent. Each app window/process has its own Rust core and its own SQLite connection (WAL handles concurrent access cleanly).

### 18.4 Multi-Window

- Tauri: user opens a second window via menu or `File > New Window`. Each window = one workspace.
- TUI: each `agentic-tui` invocation = one workspace. Shell pane-splitting is the user's tmux-equivalent choice.
- VS Code: one extension instance per VS Code window.

---

## 19. Onboarding / First Run

### 19.1 Wizard Steps (3-step minimal)

1. **Open a workspace**
   - "Pick a folder to get started" → native folder picker.
   - Detects `.git/`; if absent, offers "This folder isn't a git repo. Initialize one?" or "Pick a different folder".

2. **Profile detection & confirmation**
   - Parses `git remote get-url origin`.
   - Suggests GitHub or GitLab profile based on host.
   - Dialog: "This looks like a GitHub repo. Apply the GitHub Profile (GitHub Issues + Claude Code)?" with `[Apply] [Customize] [Not now]`.

3. **Authentication**
   - If profile applied and no valid token in keychain → opens PKCE loopback flow.
   - If CLI session detected → offers "Use your existing `gh`/`glab` session?" skip.
   - On success, lands in main UI.

### 19.2 Inline Preflight Checks

After wizard:

- Is `claude` on PATH? If not and profile is GitHub → banner: "Claude Code not detected. Install from https://docs.claude.com/claude-code [Copy install command]".
- Is `copilot` on PATH? If not and profile is GitLab → similar banner.
- Is `claude login` set up? If `claude -v` succeeds but a dry-run fails with auth error → banner with instructions.

Non-blocking — banner persists until resolved or dismissed.

### 19.3 First-Run Tutorial

After wizard, a dismissible inline card in the cockpit column:

```
Ready to go. Try:
  /plan #1        — run the default pipeline on issue #1
  /plan "<text>"  — run on a free-text task description
  @architect      — ask the architect agent something
  Cmd-K           — command palette
```

Card dismisses on first successful `/plan` or `[x]` click. Never shown again per workspace.

---

## 20. Distribution & Packaging

### 20.1 Release Cadence

- **Core + Tauri + TUI**: monthly minor releases, patch as needed.
- **VS Code extension**: tracks core version; marketplace auto-updates.
- **Semver**: `0.x.y` pre-1.0; `1.0` when all MVP features ship stably.

### 20.2 Channels

| Shell | Channel | Artifact |
|---|---|---|
| Tauri macOS | GitHub Releases | `Agentic-<version>-aarch64.dmg`, `-x86_64.dmg` (signed + notarized) |
| Tauri Windows | GitHub Releases | `Agentic-<version>-setup.exe` (NSIS), `.msi` (WiX) |
| Tauri Linux | GitHub Releases | `.AppImage`, `.deb` (x86_64, aarch64) |
| Tauri auto-update | Tauri updater | Signed delta updates |
| TUI | `cargo install agentic-tui` | Source build |
| TUI | Homebrew tap | `brew install igor/tap/agentic` |
| TUI | `winget` | `winget install Agentic.Tui` |
| TUI | `.deb` | Deferred to post-MVP (PPA setup) |
| VS Code | VS Code Marketplace | `agentic.agentic` |
| VS Code | Open VSX | Same, for VSCodium/Cursor/Windsurf |

### 20.3 Signing & Notarization

- macOS: Apple Developer ID certificate + notarytool submission; hardened runtime enabled.
- Windows: EV code signing cert; timestamped with DigiCert's RFC 3161 server.
- Linux: GPG-signed `.deb` and detached signatures for AppImage.

### 20.4 Auto-Update Strategy

- Tauri: `tauri-plugin-updater` polls a signed manifest at release time; user prompted with release notes.
- VS Code: native marketplace auto-update.
- TUI: no auto-update (Homebrew / cargo / winget handle).

---

## 21. Repository Layout (Monorepo)

```
agentic/                                  # new repo
├── Cargo.toml                            # cargo workspace root
├── pnpm-workspace.yaml                   # pnpm workspace root
├── crates/
│   ├── agentic-core/                     # the shared Rust library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pipeline/                 # state machine
│   │   │   ├── backends/
│   │   │   │   ├── claude_code.rs
│   │   │   │   └── copilot_cli.rs
│   │   │   ├── ticket_sources/
│   │   │   │   ├── github.rs
│   │   │   │   ├── gitlab.rs
│   │   │   │   ├── jira.rs
│   │   │   │   └── free_text.rs
│   │   │   ├── events/                   # Event enum + broadcast bus
│   │   │   ├── auth/                     # OAuth PKCE + keychain
│   │   │   ├── settings/                 # 3-level resolver
│   │   │   ├── db/                       # sqlite, migrations
│   │   │   └── agents/                   # markdown + frontmatter parser
│   │   └── Cargo.toml
│   ├── agentic-tauri/                    # Tauri backend (thin wrapper)
│   │   ├── src/main.rs
│   │   ├── tauri.conf.json
│   │   └── Cargo.toml
│   ├── agentic-tui/                      # ratatui binary
│   │   ├── src/main.rs
│   │   └── Cargo.toml
│   └── agentic-node/                     # napi-rs bindings for VS Code
│       ├── src/lib.rs
│       └── package.json
├── apps/
│   ├── web-ui/                           # React/Svelte app for Tauri + VS Code webview
│   │   ├── src/
│   │   ├── package.json
│   │   └── vite.config.ts
│   └── vscode-extension/
│       ├── src/
│       │   ├── extension.ts
│       │   ├── views/
│       │   └── commands/
│       ├── package.json
│       └── tsconfig.json
├── docs/
│   ├── architecture.md
│   ├── auth.md
│   └── agent-format.md
├── .github/
│   └── workflows/
│       ├── build.yml                     # cross-platform matrix
│       ├── release.yml
│       └── test.yml
├── spec.md                               # this document
├── README.md
├── LICENSE                               # MIT or Apache-2.0
└── CHANGELOG.md
```

---

## 22. MVP Cut & Roadmap

### 22.1 MVP (v1) — target first release

All three shells + both profiles ship together.

**Must-have:**
- Rust core with pipeline state machine, event bus, SQLite persistence
- Claude Code backend adapter
- Copilot CLI backend adapter with model choice
- GitHub profile (GitHub Issues + Claude Code + PR creation)
- GitLab profile (Jira + Copilot CLI + MR creation)
- GitHub Issues + GitLab Issues + Jira + free-text ticket sources
- OAuth PKCE loopback + CLI-session delegate + env-var + PAT fallback + device code for headless
- GHES and self-hosted GitLab with per-host OAuth App registration
- Keychain secret storage
- Three-level settings resolution
- Tauri desktop (macOS, Windows, Linux, signed)
- TUI (cargo install, Homebrew)
- VS Code extension (Marketplace + Open VSX)
- Structured event stream with all event types specified in §12.1
- Cockpit with action anchors for clarifying Q&A, findings triage, QA decisions
- Chat with hybrid routing (slash + @mention + tool-call fallthrough)
- Crash detection + resume prompt
- Cancellation (leave files as-is)
- Minimal onboarding wizard

**Explicitly excluded (post-MVP):**
- Ollama / LM Studio (v2)
- Parallel runs / multi-run cockpit (v2)
- Alternate pipelines (`/hotfix`) exposed in UI (v2; parser ships)
- Desktop notifications (v2)
- Telemetry (never, or explicit opt-in much later)
- Linear ticket source (v2)
- Auto-rollback on cancel (v2+, likely never)
- Cloud sync (likely never)
- Web-based multi-tenant version (likely never)

### 22.2 Post-MVP Roadmap

**v1.1 (within 2 months of MVP):**
- Ollama adapter (chat-only)
- LM Studio adapter (chat-only)
- `/hotfix` alternate pipeline enabled

**v1.2:**
- Per-event desktop notifications (opt-in)
- Parallel runs (up to N) with cockpit tabs
- Linear ticket source

**v1.3:**
- Plugin API for custom ticket sources and backends
- Pipeline templates gallery (share `pipeline.toml` snippets)

### 22.3 Phasing Within MVP

Even with "all three shells in v1", internal phasing for the build team:

1. **Week 1–4**: Core library (pipeline state machine, event bus, SQLite, settings, auth, Claude Code adapter). CLI smoke test (`agentic-cli plan #1` prints events).
2. **Week 5–8**: Tauri shell (web UI scaffold, React/Svelte components, cockpit + chat wired to core events).
3. **Week 7–10**: Copilot CLI adapter + model choice + GitLab profile + Jira ticket source. Parallel with Tauri work.
4. **Week 9–12**: TUI shell (ratatui widgets, panes, modes, keybindings).
5. **Week 11–14**: VS Code extension (napi-rs bindings, webview panels, native diffs, command contributions).
6. **Week 13–16**: OAuth flows complete (GHES/self-hosted GitLab dialogs), keychain, settings panel across all shells.
7. **Week 15–18**: Onboarding, crash recovery, polish, end-to-end testing, signing/notarization, release artifacts.
8. **Week 19–20**: RC → 1.0.

20-week estimate for a 1-developer team; compressible with parallel work or contractors on Tauri frontend.

---

## 23. Security Considerations

- **Secrets never written to disk in plaintext**; always keychain.
- **Workspace config** (`.agentic/config.toml`) may be committed — explicitly excludes any secret fields. A lint in the settings writer refuses to write token-like values to this file.
- **OAuth `state` CSRF token** validated on every callback.
- **Loopback redirect** uses random ports per auth attempt; listener serves only the expected `/callback` path; responds 404 to any other path.
- **PKCE `code_verifier`** never logged, never transmitted except in the final token exchange.
- **Subprocess invocations**: agent prompts are passed via stdin or temporary files with `0600` permissions (never as command-line args, to avoid `ps` leakage).
- **SQLite DB** stored in user's data dir with OS-default permissions (user-only on Unix).
- **Code signing** mandatory on macOS (notarization) and Windows. Unsigned builds available for power users via separate unsigned channel; not the default.
- **No telemetry** at MVP means no analytics endpoint to secure; fewer attack surfaces.

---

## 24. Testing Strategy

- **Unit tests** (`cargo test`): Rust core, per module. 100% coverage goal on pipeline state machine + auth flows + event parsers. Property-based tests (`proptest`) for state-machine transitions.
- **Integration tests**: backend adapters against mock `claude` / `copilot` binaries that emit canned JSON streams. One integration per adapter covering happy path + each error class.
- **End-to-end tests**: Playwright against Tauri running headless; vscode-test for VS Code extension; `expectrl` or `vt100` for TUI snapshots.
- **OAuth flow tests**: local mock IdP (Rust axum) that implements the PKCE handshake. Run per-provider. Skip in CI for external auth; run in release gate.
- **Cross-platform matrix**: GitHub Actions across macOS arm64, macOS x64, Windows x64, Linux x64, Linux arm64. Build + unit + integration on every platform; E2E on macOS arm64 + Windows x64 + Linux x64.

---

## 25. Open Questions

These are acknowledged gaps that need resolution during implementation but don't block starting:

1. **Copilot CLI output format**: exact JSON schema of `copilot` streaming output may have changed since this spec was written. Implementer must run `copilot --help` and verify the event parser's mapping.
2. **Jira Cloud rate limits**: Atlassian API has per-app rate limits; batch fetching of tickets may need throttling. Define during Jira adapter implementation.
3. **Monaco embedding in Tauri**: Monaco is heavy (~3 MB gzipped). Consider lighter alternatives (e.g., `@git-diff-view/react`) if bundle size becomes a problem.
4. **VS Code webview ↔ core communication**: napi-rs exposes async iterators; confirm this works reliably from a VS Code webview via the postMessage bridge (vs calling napi directly from extension host).
5. **Frontend framework choice**: React (mature ecosystem) vs Svelte (lighter bundle). Default to React unless bundle size in Tauri becomes a concern. Decide before week 5.
6. **Error event `code` taxonomy**: the specific strings in `Error.code` need an enum-like definition for UI mapping (icons, copy, actions). To be standardized during core impl.
7. **Migration from existing repo**: users of the current `agentic-orchestration` CLI pipeline need a migration path. Ideally: Agentic reads existing `agents/*.md` and `.claude/settings.json` without requiring reorganization. Validate on this repo first.

---

## 26. Success Criteria

MVP ships when:

1. All three shells launch on macOS, Windows, and Linux without errors.
2. A fresh user can go from install → first successful `/plan` in under 5 minutes on a configured GitHub repo.
3. Both profiles (GitHub+Claude, GitLab+Copilot) complete a real pipeline end-to-end against a test repo.
4. OAuth flows succeed for github.com, GHES (tested against a test instance), gitlab.com, and self-hosted GitLab.
5. Crash mid-run → restart → resume prompt appears → resumed run completes successfully.
6. Cancel mid-run → subprocess dies within 6s → files left as-is → user can `git checkout` cleanly.
7. Per-step cost/token display correct within ±1% against backend-reported usage.
8. 0 critical or high-severity issues from a security-focused code review.
9. Signed binaries verify on all three OSes.
10. VS Code extension passes Marketplace verification.

---

*End of spec.*
