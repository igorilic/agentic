# Agentic — Implementation Todos

Source spec: spec.md (v0.1)
Generated: 2026-04-21

Execution contract:
- One step per `tdd-developer` invocation.
- Each step = single TDD cycle (RED → GREEN → REFACTOR → commit), targeted at ~30–90 min of focused work.
- Steps are linear; do not start step N+1 until step N's commit lands on `main`.
- Checkpoints (`CP-*`) are human review gates; stop there and hand control back.
- Stack invariants (do not re-decide per step): Rust edition 2024, tokio, rusqlite (bundled), reqwest (rustls), keyring, directories, serde, thiserror, anyhow, tracing. Tests: `cargo test`, `proptest`, `mockito`/`wiremock`, `tempfile`, `assert_fs`, `insta` (optional for snapshot).
- Commit style: Conventional Commits. One test-first commit per TDD cycle is allowed but the default pattern in this plan is to land the RED→GREEN→REFACTOR loop as a single `feat(...)` or `fix(...)` commit containing both the test and the implementation. Use `test(...)` only for pure test additions (e.g. fixtures).

Legend:
- Crate shorthand: `core` = `agentic-core`, `tui` = `agentic-tui`, `tauri` = `agentic-tauri`, `node` = `agentic-node`, `web` = `apps/web-ui`, `vsx` = `apps/vscode-extension`, `cli` = `agentic-cli` (dev smoke-test binary, post-MVP-mergeable into a shell).

---

## Phase 0 — Repo scaffolding

### [x] Step 0.1: Initialize Cargo workspace

**Goal**: Establish an empty Rust workspace so subsequent crates can be added incrementally.

**Depends on**: none (repo currently has only `spec.md` and `LICENSE`).

**Test first** (RED):
- Add `tests/workspace_smoke.rs` under a new `xtask`-style integration-test crate `crates/agentic-meta-tests/` with a single test `cargo_metadata_loads_workspace` that runs `cargo metadata --no-deps --format-version=1` as a subprocess and asserts exit 0 + JSON contains `"workspace_members"`.

**Implement** (GREEN):
- Create root `Cargo.toml` with `[workspace]`, `resolver = "3"`, `members = ["crates/agentic-meta-tests"]`, `[workspace.package] edition = "2024" rust-version = "1.85"` (or current stable supporting edition 2024), and `[workspace.dependencies]` empty stub with comments marking where shared deps will land.
- Create `crates/agentic-meta-tests/Cargo.toml` (package = `agentic-meta-tests`, `publish = false`).
- Create `crates/agentic-meta-tests/tests/workspace_smoke.rs`.
- Add `rust-toolchain.toml` pinning `channel = "stable"`.
- Add `.gitignore` (target/, node_modules/, dist/, .DS_Store, `*.db`, `.agentic/state.*`).

**Refactor**: None.

**Commit**: `chore(workspace): initialize cargo workspace with edition 2024`

**Verification**: `cargo test -p agentic-meta-tests`

---

### [x] Step 0.2: Scaffold `agentic-core` library crate

**Goal**: Create an empty `agentic-core` library that compiles and exports a versioned public surface.

**Depends on**: Step 0.1.

**Test first** (RED):
- In `crates/agentic-core/tests/public_surface.rs`: assert `agentic_core::VERSION` equals `env!("CARGO_PKG_VERSION")`.
- Add `crates/agentic-core/src/lib.rs` doctest for `VERSION`.

**Implement** (GREEN):
- `crates/agentic-core/Cargo.toml`: `name = "agentic-core"`, `edition.workspace = true`, empty deps.
- `crates/agentic-core/src/lib.rs`: `#![deny(unsafe_code)]`, `pub const VERSION: &str = env!("CARGO_PKG_VERSION");`.
- Add crate to root workspace `members`.

**Refactor**: None.

**Commit**: `feat(core): scaffold agentic-core library crate`

**Verification**: `cargo test -p agentic-core`

---

### Step 0.3: Wire `tracing` + `tracing-subscriber` into core with filter env

**Goal**: Every subsequent step can emit structured logs without re-deciding the subscriber shape.

**Depends on**: Step 0.2.

**Test first** (RED):
- `crates/agentic-core/src/logging.rs` tests: `fn init_test_subscriber()` can be called twice in the same process without panicking (use `OnceLock`). Test captures a span via `tracing_test::traced_test` and asserts a log line appears.

**Implement** (GREEN):
- Add workspace deps: `tracing`, `tracing-subscriber` (features: `env-filter`, `fmt`), `tracing-test` (dev-dep).
- `logging.rs` with `pub fn init(filter: Option<&str>)` honoring `AGENTIC_LOG` env var, and `pub fn init_test_subscriber()`.
- Re-export from `lib.rs`.

**Refactor**: Extract a small `LogConfig` struct if branches multiply.

**Commit**: `feat(core): add tracing subscriber with AGENTIC_LOG filter`

**Verification**: `cargo test -p agentic-core logging::`

---

### Step 0.4: Add `thiserror`/`anyhow` error scaffolding

**Goal**: Core exposes a stable `CoreError` enum and a `Result<T>` alias; downstream modules use it.

**Depends on**: Step 0.2.

**Test first** (RED):
- `crates/agentic-core/src/error.rs` tests:
  - `CoreError::Io(...)` displays via `Display` and preserves source chain.
  - `CoreError` is `Send + Sync + 'static` (compile-time assertion via `fn assert_send_sync<T: Send + Sync + 'static>() {}`).
  - Conversion `From<std::io::Error> for CoreError` works.

**Implement** (GREEN):
- Add deps: `thiserror`, `anyhow`.
- `error.rs` with variants: `Io`, `Db`, `Config`, `Auth`, `Backend`, `TicketSource`, `Parse`, `Other` (each wrapping a source where meaningful).
- `pub type Result<T> = std::result::Result<T, CoreError>;`.

**Refactor**: None yet — real conversions added in later steps.

**Commit**: `feat(core): introduce CoreError with thiserror`

**Verification**: `cargo test -p agentic-core error::`

---

### Step 0.5: Set up `pnpm` workspace skeleton

**Goal**: Root `pnpm-workspace.yaml` + empty `apps/web-ui` + empty `apps/vscode-extension` packages that `pnpm install` accepts, so frontend work can start in later phases without re-scaffolding.

**Depends on**: Step 0.1.

**Test first** (RED):
- `crates/agentic-meta-tests/tests/pnpm_workspace.rs` (gated by `#[cfg_attr(not(feature = "pnpm"), ignore)]` or by detecting `pnpm` on `PATH` and skipping otherwise): asserts `pnpm -r exec node -e "process.exit(0)"` returns 0 when run from repo root.
- If `pnpm` is not installed, the test is marked `ignored` rather than failing — but a documentation assertion checks `pnpm-workspace.yaml` exists and lists the two apps.

**Implement** (GREEN):
- `pnpm-workspace.yaml` with `packages: ["apps/*"]`.
- `apps/web-ui/package.json` (name `@agentic/web-ui`, private, minimal scripts).
- `apps/vscode-extension/package.json` (name `@agentic/vscode-extension`, private).
- `.npmrc` pinning `package-manager-strict=true` and `engine-strict=true`.
- `package.json` (root) with `"packageManager": "pnpm@9.x"` (use a current pinned version at impl time).

**Refactor**: None.

**Commit**: `chore(web): scaffold pnpm workspace with empty web-ui and vscode-extension`

**Verification**: `cargo test -p agentic-meta-tests pnpm_workspace` and manually `pnpm install` (dev note — not enforced in CI yet).

---

### Step 0.6: Add GitHub Actions `test` workflow for Rust

**Goal**: Every PR runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test --workspace` on macOS + Linux.

**Depends on**: Steps 0.1–0.4.

**Test first** (RED):
- `crates/agentic-meta-tests/tests/ci_shape.rs`: parse `.github/workflows/test.yml` as YAML and assert presence of jobs `fmt`, `clippy`, `test`, and the matrix contains `macos-latest` and `ubuntu-latest`.

**Implement** (GREEN):
- `.github/workflows/test.yml` with those three jobs; cache `~/.cargo/registry` + `target/` via `actions/cache` (or `Swatinem/rust-cache`).
- `rustfmt.toml` (minimal: `edition = "2024"`, `max_width = 100`).
- `clippy.toml` (empty at first).
- Add `serde_yaml` as a dev-dep to `agentic-meta-tests` for the parser test.

**Refactor**: None.

**Commit**: `ci: add rust test matrix for macos and linux`

**Verification**: `cargo test -p agentic-meta-tests ci_shape` (local). Actual CI runs on PR.

---

### CP-0: Review scaffolding

**Checkpoint**: Stop. Hand back to user.
- Verify local `cargo test --workspace` passes.
- Verify CI green on a throwaway branch push.
- Confirm no premature architectural decisions have been committed.

---

## Phase 1 — Core: paths, settings, and persistence foundation

### Step 1.1: Path resolver using `directories`

**Goal**: Deterministic `config_dir()`, `data_dir()`, `log_dir()` in core; injectable for tests.

**Depends on**: CP-0.

**Test first** (RED):
- `tests/paths.rs`:
  - `Paths::new_for_tests(tempdir)` yields paths rooted under the temp dir.
  - `Paths::config_file()` ends with `settings.toml`.
  - `Paths::db_file()` ends with `state.db`.
  - `Paths::ensure_dirs()` creates missing parents idempotently.

**Implement** (GREEN):
- Add dep: `directories`.
- `src/paths.rs` with `pub struct Paths { root: PathBuf, data_root: PathBuf }` and constructors `from_os()` + `for_tests(base: &Path)`.

**Refactor**: None.

**Commit**: `feat(core): add path resolver with test-friendly constructor`

**Verification**: `cargo test -p agentic-core paths::`

---

### Step 1.2: SQLite connection + WAL pragma

**Goal**: `Db` type that opens a pooled connection at `Paths::db_file()` with WAL + `foreign_keys=ON`.

**Depends on**: Step 1.1.

**Test first** (RED):
- `tests/db_open.rs`:
  - Opening a fresh DB in a tempdir succeeds.
  - `PRAGMA journal_mode` returns `wal`.
  - `PRAGMA foreign_keys` returns `1`.
  - Opening the same path twice succeeds (WAL concurrency).

**Implement** (GREEN):
- Deps: `rusqlite` (features `bundled`), `r2d2`, `r2d2_sqlite` (or a thin custom wrapper; prefer `r2d2_sqlite` at MVP).
- `src/db/mod.rs` with `Db::open(paths: &Paths) -> Result<Db>` and `Db::open_in_memory()` for tests.

**Refactor**: Extract `fn apply_pragmas(conn: &Connection)` helper.

**Commit**: `feat(core): open sqlite with WAL and foreign keys enabled`

**Verification**: `cargo test -p agentic-core db::`

---

### Step 1.3: Migration runner (0001 — workspaces table)

**Goal**: Minimal migration runner + first migration creating `workspaces`.

**Depends on**: Step 1.2.

**Decision point** (flag for implementer): Choose between `refinery` (embedded migrations crate) and a hand-rolled runner that reads `.sql` files from `include_dir!`. Tradeoff: `refinery` brings proc-macro + external schema tracking table; hand-rolled keeps surface minimal and matches spec §13.1 language ("`refinery` **or** a hand-rolled migrator"). Default recommendation unless the implementer prefers otherwise: hand-rolled (one SQL file per migration, tracked in `_migrations(version INTEGER PK, applied_at INTEGER)`), because the schema is small and we want to avoid a macro dependency in the core.

**Test first** (RED):
- `tests/migrations.rs`:
  - Running migrations on a fresh DB creates `_migrations` and `workspaces`.
  - Running twice is idempotent.
  - Each applied migration produces a row in `_migrations`.
  - Schema of `workspaces` matches spec §13.1 column-for-column (introspect via `PRAGMA table_info`).

**Implement** (GREEN):
- `src/db/migrations/mod.rs` with `Migrator::run(&Db)`.
- `src/db/migrations/0001_workspaces.sql` matching spec §13.1 `workspaces` columns.
- Invoke from `Db::open` automatically.

**Refactor**: Introduce a `Migration { version, name, sql }` struct.

**Commit**: `feat(core): add migration runner with 0001 workspaces`

**Verification**: `cargo test -p agentic-core db::migrations::`

---

### Step 1.4: Migration 0002 — runs + run_steps

**Goal**: Add the runs and run_steps tables.

**Depends on**: Step 1.3.

**Test first** (RED):
- Extend `tests/migrations.rs`:
  - After migrate, inserting a run with a missing `workspace_id` fails the FK.
  - Cascading delete: deleting a run removes its `run_steps`.
  - `idx_runs_workspace_status` and `idx_runs_started_at` exist (`sqlite_master` query).
  - `idx_run_steps_run_seq` exists.

**Implement** (GREEN):
- `src/db/migrations/0002_runs_and_steps.sql` per spec §13.1.

**Refactor**: None.

**Commit**: `feat(core): add migration 0002 for runs and run_steps`

**Verification**: `cargo test -p agentic-core db::migrations::`

---

### [x] Step 1.5: Migration 0003 — findings, clarifying_questions, file_changes

**Goal**: Add artifact tables per spec §13.1.

**Depends on**: Step 1.4.

**Test first** (RED):
- `tests/migrations.rs`: existence of tables, FK cascades on `run_id` and `step_id`, and presence of `idx_findings_run_triage`.

**Implement** (GREEN): `0003_artifacts.sql`.

**Refactor**: None.

**Commit**: `feat(core): add migration 0003 for findings, clarifying_questions, file_changes`

**Verification**: `cargo test -p agentic-core db::migrations::`

---

### [x] Step 1.6: Migration 0004 — stream_events (BLOB payload)

**Goal**: Event log table with (run_id, seq) primary key.

**Depends on**: Step 1.5.

**Test first** (RED):
- `tests/migrations.rs`: table exists, PK is composite, index `idx_stream_events_step` exists, inserting duplicate `(run_id, seq)` fails.

**Implement** (GREEN): `0004_stream_events.sql`.

**Refactor**: None.

**Commit**: `feat(core): add migration 0004 for stream_events`

**Verification**: `cargo test -p agentic-core db::migrations::`

---

### [x] Step 1.7: Migration 0005 — chat_sessions, chat_messages

**Goal**: Chat persistence tables.

**Depends on**: Step 1.6.

**Test first** (RED): existence, FK cascades, `idx_chat_messages_session_ts`.

**Implement** (GREEN): `0005_chat.sql`.

**Refactor**: None.

**Commit**: `feat(core): add migration 0005 for chat tables`

**Verification**: `cargo test -p agentic-core db::migrations::`

---

### [x] Step 1.8: Migration 0006 — auth_accounts, settings

**Goal**: Auth metadata (no secrets) + key/value settings table.

**Depends on**: Step 1.7.

**Test first** (RED): existence, `auth_accounts.id` is PK, `settings.key` is PK, scope enforced via `CHECK` clause (`user` or `workspace:*`).

**Implement** (GREEN): `0006_auth_and_settings.sql`.

**Refactor**: None.

**Commit**: `feat(core): add migration 0006 for auth_accounts and settings`

**Verification**: `cargo test -p agentic-core db::migrations::`

---

### Step 1.9: `Workspace` repository (CRUD)

**Goal**: Typed CRUD for `workspaces` (insert, get-by-id, list-recent, touch-last-opened).

**Depends on**: Step 1.8.

**Test first** (RED):
- `tests/workspace_repo.rs`:
  - Insert returns the inserted `Workspace`.
  - Get by unknown id returns `None`.
  - List ordered by `last_opened DESC`.
  - Touch updates `last_opened`.

**Implement** (GREEN):
- `src/db/workspaces.rs` with `WorkspaceRepo` holding a pool clone.
- Use `blake3` for id computation; add workspace dep.

**Refactor**: Extract a small `fn now_ms()` helper into `src/time.rs`.

**Commit**: `feat(core): add workspace repository with blake3 id`

**Verification**: `cargo test -p agentic-core db::workspaces::`

---

### Step 1.10: Settings 3-level resolver (scaffolding, no auth yet)

**Goal**: Resolve a setting key by checking env → workspace TOML → user TOML → default, returning `(value, source)`.

**Depends on**: Step 1.9.

**Test first** (RED):
- `tests/settings_resolver.rs`:
  - Env var wins over workspace.
  - Workspace wins over user.
  - User wins over default.
  - Missing key + no default returns `None`.
  - Source tag is correct in each case.
- Use a fake `EnvProvider` trait to avoid touching real env.

**Implement** (GREEN):
- Deps: `toml`, `serde` (already present).
- `src/settings/mod.rs`: `Setting<T>` + `Resolver`.
- Support at least the keys: `defaults.profile`, `defaults.backend`, `defaults.model`, `ui.theme`.

**Refactor**: Typed key enum with `.env_var()` and `.toml_path()` accessors.

**Commit**: `feat(core): three-level settings resolver with source tracking`

**Verification**: `cargo test -p agentic-core settings::`

---

### CP-1: Review persistence + settings foundation

**Checkpoint**: Stop.
- Ensure all 6 migrations run cleanly on a fresh DB.
- Verify settings resolver handles at least the four sample keys.
- Sanity-check that no IO happens outside injectable providers (i.e., tests don't need env mutation or `$HOME` writes).

---

## Phase 2 — Core: event model, broadcast bus, and persistence of events

### Step 2.1: `Event` enum + `EventEnvelope` + serde roundtrip

**Goal**: Concrete `Event` enum per spec §12.1, with `event_id`, `run_id`, optional `step_id`, `timestamp_ms`, plus serde serialization.

**Depends on**: CP-1.

**Test first** (RED):
- `tests/events_serde.rs`:
  - Each variant roundtrips through JSON.
  - `RunStarted` deserializes from a known fixture file (`tests/fixtures/events/run_started.json`).
  - `event_id` is a ULID (regex-validated).
  - `timestamp_ms` is monotonic across `Event::now()` calls in one test.

**Implement** (GREEN):
- Deps: `serde`, `serde_json`, `ulid`.
- `src/events/mod.rs`: `Event`, `EventEnvelope { event_id, run_id, step_id, timestamp_ms, event: Event }`, plus sub-enums (`StepStatus`, `RunStatus`, `Severity`, `ToolStream`, `ActionRequired`, `TokenUsage`).
- Use `#[serde(tag = "type", content = "data")]` to stabilize JSON shape.

**Refactor**: None yet; the enum will grow, keep it centralized.

**Commit**: `feat(core): define Event enum with JSON roundtrip`

**Verification**: `cargo test -p agentic-core events::`

---

### Step 2.2: Event broadcast bus

**Goal**: `EventBus` wrapping `tokio::sync::broadcast::Sender<EventEnvelope>` with a `subscribe()` API and a capacity default.

**Depends on**: Step 2.1.

**Test first** (RED):
- `tests/event_bus.rs`:
  - Two subscribers each receive every published event.
  - A slow subscriber lagging past capacity yields `RecvError::Lagged(n)` without breaking others.
  - `publish()` returns number of active receivers.
- Use `#[tokio::test]`.

**Implement** (GREEN):
- Dep: `tokio` with features `rt-multi-thread`, `macros`, `sync`.
- `src/events/bus.rs` with capacity default 1024, configurable.

**Refactor**: None.

**Commit**: `feat(core): event broadcast bus with lag detection`

**Verification**: `cargo test -p agentic-core events::bus::`

---

### Step 2.3: Event persister (writes to `stream_events`)

**Goal**: Subscribe the bus to a persister that appends each event to SQLite with monotonic `seq`.

**Depends on**: Step 2.2, Step 1.6.

**Decision point** (flag for implementer): payload encoding. Spec §13.1 says MessagePack; default to `rmp-serde`. If the implementer prefers JSON-in-BLOB for debuggability at the cost of ~30% more disk, note in commit body; the choice is reversible by migration.

**Test first** (RED):
- `tests/event_persister.rs`:
  - Publishing 100 events to the bus causes 100 rows in `stream_events` with `seq` 0..99 for the same `run_id`.
  - Persister survives decode errors on bad payloads (skips, logs; does NOT crash subscribers).
  - Querying events by `run_id` returns them in seq order.

**Implement** (GREEN):
- Dep: `rmp-serde`.
- `src/events/persist.rs` with `EventPersister::spawn(bus_subscriber, db)` returning a `JoinHandle`.

**Refactor**: Extract `fn next_seq(conn, run_id)` helper.

**Commit**: `feat(core): persist events to stream_events table`

**Verification**: `cargo test -p agentic-core events::persist::`

---

### Step 2.4: Run + Step repository

**Goal**: CRUD for `runs` and `run_steps`, including `status` transitions.

**Depends on**: Step 2.3.

**Test first** (RED):
- `tests/run_repo.rs`:
  - Create a run in `pending`, transition to `running`, `completed`.
  - Invalid transitions return `Err(CoreError::InvalidStateTransition)` (from `Pending` directly to `Completed` without `Running`).
  - `list_by_workspace(workspace_id, limit)` returns DESC by `started_at`.
  - Creating a step attached to a nonexistent run returns FK error.

**Implement** (GREEN):
- Add `InvalidStateTransition` variant to `CoreError`.
- `src/db/runs.rs` + `src/db/steps.rs`.
- Use ULIDs for ids.

**Refactor**: Shared `status_from_str`/`status_to_str` helpers.

**Commit**: `feat(core): runs and run_steps repositories with state guards`

**Verification**: `cargo test -p agentic-core db::runs:: db::steps::`

---

### CP-2: Review event + persistence shape

**Checkpoint**: Stop.
- Verify event JSON schema is stable enough to hand to shell implementers.
- Decide: does the TUI/Tauri schema need a schema-version field in the envelope? (Low-risk to add now; harder later.)

---

## Phase 3 — Core: pipeline state machine (no real backend yet)

### Step 3.1: Pipeline types + agent frontmatter parser

**Goal**: Parse agent markdown files per spec §10.3.

**Depends on**: CP-2.

**Test first** (RED):
- `tests/agents_parse.rs`:
  - Valid frontmatter parses all fields.
  - Missing `name` returns `CoreError::Parse`.
  - Unknown fields are ignored (forward-compat).
  - `pipeline_role` defaults to `step` when absent.
  - `name` mismatch with filename stem returns a specific error.
- Fixtures under `tests/fixtures/agents/`.

**Implement** (GREEN):
- Deps: `serde_yaml`, `pulldown-cmark` (optional, only if we split out the body — otherwise just string-after-second-`---`).
- `src/agents/mod.rs`.

**Refactor**: None.

**Commit**: `feat(core): parse agent markdown frontmatter`

**Verification**: `cargo test -p agentic-core agents::`

---

### Step 3.2: Agent discovery search order

**Goal**: Implement spec §10.2 search order across `.agentic/agents/`, `.claude/agents/`, `agents/`.

**Depends on**: Step 3.1.

**Test first** (RED):
- `tests/agents_discovery.rs` using `tempfile`:
  - File in `.agentic/agents/architect.md` wins over `.claude/` and `agents/`.
  - File in `.claude/agents/` wins over `agents/`.
  - Missing everywhere → `CoreError::AgentNotFound`.

**Implement** (GREEN):
- `src/agents/discovery.rs`.

**Refactor**: None.

**Commit**: `feat(core): agent discovery with priority fallback`

**Verification**: `cargo test -p agentic-core agents::discovery::`

---

### Step 3.3: `pipeline.toml` parser

**Goal**: Parse the pipeline TOML per spec §10.4, allowing multiple pipelines with `default` always present.

**Depends on**: Step 3.1.

**Test first** (RED):
- `tests/pipeline_toml.rs`:
  - Default pipeline parses with 4 steps in order.
  - `hotfix` pipeline parses (even though feature-gated off).
  - Missing file → built-in default is returned.
  - Invalid (unknown top-level key) → `CoreError::Parse`.

**Implement** (GREEN):
- `src/pipeline/config.rs`.

**Refactor**: None.

**Commit**: `feat(core): parse pipeline.toml with fallback to built-in default`

**Verification**: `cargo test -p agentic-core pipeline::config::`

---

### Step 3.4: Pipeline state machine transitions (proptest)

**Goal**: A pure state machine `PipelineSm` that accepts transition inputs and validates outputs per spec §10.1.

**Depends on**: Steps 2.4, 3.3.

**Test first** (RED):
- `tests/pipeline_sm.rs`:
  - Unit: happy path `pending → running → architect → tdd-developer → qa → reviewer → completed`.
  - Unit: QA fails 3 times → tech-debt → reviewer → `completed_with_tech_debt`.
  - Unit: cancel during any running step yields `cancelled`.
  - Proptest: any valid sequence of events leaves the SM in a reachable state (`proptest`-generated transition inputs). Invariants:
    - Once terminal (any of completed/cancelled/failed/crashed), no more transitions accepted.
    - `run.status == running` iff exactly one step has `status == running`.

**Implement** (GREEN):
- Dep: `proptest`.
- `src/pipeline/sm.rs`: `PipelineSm { state: PipelineState, ... } impl PipelineSm { pub fn handle(&mut self, input: SmInput) -> Result<Vec<Event>> }`.
- **Do not** yet connect this to real backends — output is just events.

**Refactor**: Split state representation from transition logic if `handle()` grows >100 lines.

**Commit**: `feat(core): pipeline state machine with proptest invariants`

**Verification**: `cargo test -p agentic-core pipeline::sm::`

---

### Step 3.5: Wire state machine to Run/Step repos via the bus

**Goal**: A `PipelineOrchestrator` that listens on the bus, mutates run/step rows, and rebroadcasts normalized state-change events.

**Depends on**: Steps 2.2, 2.4, 3.4.

**Test first** (RED):
- `tests/orchestrator.rs` (`#[tokio::test]`):
  - Given a seeded run, publishing `StepStarted` updates the row's status to `running`.
  - `StepComplete { passed }` increments `seq` and sets `completed_at`, `duration_ms`.
  - `RunComplete` is rebroadcast with final status and persisted.

**Implement** (GREEN):
- `src/pipeline/orchestrator.rs`.

**Refactor**: None.

**Commit**: `feat(core): orchestrator connects state machine to persistence and bus`

**Verification**: `cargo test -p agentic-core pipeline::orchestrator::`

---

### CP-3: Review pipeline machine

**Checkpoint**: Stop.
- Walk through one simulated run end-to-end in a test harness.
- Confirm no backend or auth coupling leaked into the state machine.

---

## Phase 4 — Core: Backend trait + mock backend

### Step 4.1: `Backend` trait + `ExecuteRequest`/`ExecuteOutcome` types

**Goal**: The trait from spec §11.1, implementable by mocks before real adapters exist.

**Depends on**: CP-3.

**Test first** (RED):
- `tests/backend_trait.rs`: trait compiles; a `NullBackend` implementing it trivially can be held in a `Box<dyn Backend>` and is `Send + Sync`.

**Implement** (GREEN):
- Dep: `async-trait`, `tokio-util` (for `CancellationToken`).
- `src/backends/mod.rs` with trait, `BackendId`, `ModelId`, `HealthStatus`, `TokenUsage` (relocate from events if needed — keep single source of truth).

**Refactor**: None.

**Commit**: `feat(core): define Backend trait surface`

**Verification**: `cargo test -p agentic-core backends::`

---

### Step 4.2: Scripted mock backend

**Goal**: A `ScriptedBackend` that emits a canned sequence of `Event`s to the sink, useful for all downstream tests.

**Depends on**: Step 4.1.

**Test first** (RED):
- `tests/backends_scripted.rs`:
  - Given a script of 5 events, the sink receives them in order.
  - Respects `CancellationToken` (drops remaining events).
  - Returns `ExecuteOutcome::passed` when script ends normally.
  - Returns `failed` if script contains `Event::Error { recoverable: false }`.

**Implement** (GREEN):
- `src/backends/scripted.rs` (cfg-gated `#[cfg(any(test, feature = "testing"))]` so it stays out of release builds unless opted in).

**Refactor**: None.

**Commit**: `feat(core): scripted mock backend for test harnesses`

**Verification**: `cargo test -p agentic-core backends::scripted::`

---

### Step 4.3: Integration — orchestrator + scripted backend end-to-end

**Goal**: Run a full default pipeline with four scripted "agents" and verify run+steps+events are persisted.

**Depends on**: Steps 3.5, 4.2.

**Test first** (RED):
- `crates/agentic-core/tests/e2e_scripted.rs`:
  - Seed workspace + run.
  - Run orchestrator with four scripted backends (architect, tdd, qa, reviewer) each emitting 2 events then `StepComplete { passed }`.
  - Assert final run status = `completed`.
  - Assert `stream_events` has 4 × (StepStarted + 2 deltas + StepComplete) + RunStarted + RunComplete.

**Implement** (GREEN):
- Tie together the harness in `tests/support/` helpers. No new public APIs needed.

**Refactor**: Extract test-only helpers into a `agentic-core` `testing` feature.

**Commit**: `test(core): end-to-end pipeline test with scripted backends`

**Verification**: `cargo test -p agentic-core --features testing e2e_scripted`

---

### CP-4: Milestone 1 — core can run architect-only step against scripted backend and persist events

**Checkpoint**: Stop. Hand back to user for end-to-end review.
- Demonstrate: `cargo test -p agentic-core --features testing` all green.
- Confirm event JSON shapes via fixture diff.
- Decide whether to proceed with the Claude adapter or the CLI smoke binary first (next step proposes CLI, easy to reorder).

---

## Phase 5 — Core: dev CLI smoke binary

### Step 5.1: `agentic-cli` binary crate

**Goal**: A thin binary that wires `Paths`, `Db`, `EventBus`, orchestrator, and prints events as JSON to stdout. Not a shipping artifact; used for smoke tests and as the seed of the TUI later.

**Depends on**: CP-4.

**Test first** (RED):
- `crates/agentic-cli/tests/cli_smoke.rs`:
  - `agentic-cli run --scripted <path>` exits 0 and emits one JSON event per line.
  - `--help` includes `run` subcommand.
  - Invalid DB path returns exit code 2.

**Implement** (GREEN):
- Dep: `clap` (features `derive`).
- `crates/agentic-cli/src/main.rs`.
- Subcommands: `run --scripted <path>`, `doctor`, `migrate`.

**Refactor**: None.

**Commit**: `feat(cli): add agentic-cli smoke binary`

**Verification**: `cargo test -p agentic-cli`

---

### Step 5.2: `doctor` subcommand — environment probe

**Goal**: `agentic-cli doctor` checks `claude`, `copilot`, `gh`, `glab` on PATH and prints a table.

**Depends on**: Step 5.1.

**Test first** (RED):
- `tests/doctor.rs`:
  - With `which=false` stubbed, output includes "claude: not found".
  - With stubbed `which=true`, output includes "claude: found at …".
- Use a trait-based probe for injectability.

**Implement** (GREEN):
- `src/doctor.rs` in the cli crate.

**Refactor**: None.

**Commit**: `feat(cli): add doctor subcommand`

**Verification**: `cargo test -p agentic-cli doctor`

---

## Phase 6 — Core: Claude Code backend adapter

### Step 6.1: Claude stream-JSON parser (offline fixtures)

**Goal**: Parse line-delimited Claude Agent SDK events per spec §11.2 from static fixtures into `Event`.

**Depends on**: CP-4.

**Test first** (RED):
- `tests/claude_parser.rs`:
  - Fixture `claude/message_start.jsonl` → emits `StepStarted`-related deltas.
  - `tool_use` line → `ToolUseStart`.
  - `message_delta` with token usage → mapped onto `TokenUsage`.
  - Bad JSON line → one `Error { code: "protocol_error" }` event, then parser continues.
- Fixtures stored under `crates/agentic-core/tests/fixtures/claude/`.

**Implement** (GREEN):
- `src/backends/claude_code/parser.rs`.
- Do NOT spawn any process yet — parser takes an `impl AsyncBufRead` + emits to an `EventSink`.

**Refactor**: Factor the token accumulator into its own struct.

**Commit**: `feat(core): parse Claude Code stream-json events`

**Verification**: `cargo test -p agentic-core backends::claude_code::parser::`

---

### Step 6.2: Subprocess runner + stdin piping

**Goal**: Spawn `claude -p --output-format stream-json ...` with CWD, env, and stdin piping. Injectable binary path.

**Depends on**: Step 6.1.

**Test first** (RED):
- `tests/claude_subprocess.rs`:
  - Use a shell script fixture (`tests/fixtures/bin/fake-claude.sh`) that echoes a known JSONL stream on stdin.
  - Subprocess runner invokes it with `CLAUDE_CODE_BIN` env override and captures stdout.
  - SIGTERM on cancel is delivered within 1s; process exits within 5s or SIGKILL.
  - On Windows, use a `.bat` equivalent; test gated by `#[cfg(unix)]` for the signal check.

**Implement** (GREEN):
- Dep: `tokio` features `process`, `io-util`.
- `src/backends/claude_code/runner.rs`.

**Refactor**: Extract `Cancellable` helper if used by multiple adapters.

**Commit**: `feat(core): spawn claude subprocess with cancellation support`

**Verification**: `cargo test -p agentic-core backends::claude_code::runner::`

---

### Step 6.3: Claude `Backend` trait impl — end-to-end against `fake-claude`

**Goal**: `ClaudeCodeBackend::execute` that wires parser + runner + sink, producing an `ExecuteOutcome`.

**Depends on**: Steps 6.1, 6.2.

**Test first** (RED):
- `tests/claude_backend_e2e.rs`:
  - With the fake-claude fixture emitting a passing script, execute returns `StepStatus::Passed`, populated `TokenUsage`, `cost_usd` computed from a bundled pricing table.
  - Fake-claude emitting an `error` event maps to `Error { recoverable: false }` and `StepStatus::Failed`.
  - Cancel mid-stream: outcome is `StepStatus::Failed { code: "cancelled" }` (or an explicit `Cancelled` variant — confirm naming against §11.4).

**Implement** (GREEN):
- `src/backends/claude_code/mod.rs` implementing `Backend`.
- `src/backends/claude_code/pricing.rs` with a static map keyed by model id (bundled per spec §11.2).

**Refactor**: None.

**Commit**: `feat(core): claude-code backend adapter`

**Verification**: `cargo test -p agentic-core backends::claude_code::`

---

### Step 6.4: File-snapshot diffing

**Goal**: Before/after per step, walk affected paths (gathered from `Edit`/`Write` tool-use events), compute hashes, emit `FileChange`, and persist a unified-diff patch into `file_changes.diff`.

**Depends on**: Step 6.3.

**Test first** (RED):
- `tests/file_snapshots.rs`:
  - Modifying file A → `FileChange { path: A, before_hash, after_hash }` emitted.
  - `file_changes.diff` contains a unified-diff patch recoverable via `similar`/`diffy`.
  - Ignoring binary files: a file >1 MB or non-UTF8 stores hashes but `diff = NULL`.

**Implement** (GREEN):
- Deps: `similar` or `diffy` (pick one; recommend `similar` — both are fine), `blake3` (already added).
- `src/backends/file_snapshots.rs` used by all backends.

**Refactor**: None.

**Commit**: `feat(core): file snapshot diffing for tool-use edits`

**Verification**: `cargo test -p agentic-core backends::file_snapshots::`

---

### CP-5: Milestone 2 — full default pipeline using real `claude` CLI

**Checkpoint**: Stop. Hand back to user.
- Run `agentic-cli run --ticket "free text: hello world"` in a scratch repo with `CLAUDE_CODE_BIN=<real claude>`.
- Inspect SQLite contents.
- Decide: stick with `similar` or swap for `diffy` based on patch fidelity.

---

## Phase 7 — Core: Copilot CLI backend adapter

### Step 7.1: Copilot CLI probe + decision point

**Goal**: Verify the current Copilot CLI JSON output schema (spec §25.1 explicitly defers this).

**Depends on**: CP-5.

**Decision point** (flag for implementer): Before writing the parser, run `copilot --help` and `copilot <prompt> --no-interactive` (or equivalent) and record a few representative fixtures. Document the observed schema in `docs/copilot-schema.md`. If the schema is incompatible with our `Event` enum (e.g. Copilot emits raw streaming text without tool-use structure), raise an ADR (`/adr`) before proceeding.

**Test first** (RED):
- `tests/copilot_fixtures_exist.rs`: assert at least 3 fixture files under `tests/fixtures/copilot/` exist and each is non-empty JSONL.

**Implement** (GREEN):
- Record fixtures.
- Write `docs/copilot-schema.md` summarizing the observed event types.
- If an ADR is warranted, author `docs/decisions/ADR-001-copilot-stream-mapping.md`.

**Refactor**: None.

**Commit**: `docs(copilot): record observed stream schema and fixtures`

**Verification**: `cargo test -p agentic-core copilot_fixtures_exist`

---

### Step 7.2: Copilot parser

**Goal**: Parse Copilot JSONL into core `Event`s.

**Depends on**: Step 7.1.

**Test first** (RED): Analogous to Step 6.1 but for Copilot fixtures.

**Implement** (GREEN): `src/backends/copilot_cli/parser.rs`.

**Refactor**: Pull shared JSONL-line-splitter helper out of Claude + Copilot into `src/backends/util.rs`.

**Commit**: `feat(core): parse copilot-cli stream events`

**Verification**: `cargo test -p agentic-core backends::copilot_cli::parser::`

---

### Step 7.3: Copilot subprocess runner + backend impl

**Goal**: Analogous to Steps 6.2 + 6.3 for Copilot.

**Depends on**: Step 7.2.

**Test first** (RED): Fake `copilot.sh` fixture; happy + error + cancel paths.

**Implement** (GREEN): `src/backends/copilot_cli/{runner.rs,mod.rs}`.

**Refactor**: Introduce a shared `SubprocessAdapter` trait if duplication is >60%.

**Commit**: `feat(core): copilot-cli backend adapter`

**Verification**: `cargo test -p agentic-core backends::copilot_cli::`

---

### Step 7.4: Model list resolution

**Goal**: `CopilotCli::supported_models()` tries `copilot models list` at runtime, falling back to a bundled list when unsupported.

**Depends on**: Step 7.3.

**Test first** (RED):
- Fake binary returns a JSON list → parsed to `Vec<ModelId>`.
- Fake binary returns non-zero → fallback list returned.

**Implement** (GREEN):
- `src/backends/copilot_cli/models.rs`.

**Refactor**: None.

**Commit**: `feat(core): copilot model list with runtime probe and fallback`

**Verification**: `cargo test -p agentic-core backends::copilot_cli::models::`

---

## Phase 8 — Core: ticket sources

### Step 8.1: `TicketSource` trait

**Goal**: Define the trait with `fetch(ref: &TicketRef) -> Result<Ticket>`; `Ticket` has title, body, comments, ac_field, url.

**Depends on**: CP-5.

**Test first** (RED):
- Trait compile test; `FreeTextTicketSource` impl returns a ticket with given body.

**Implement** (GREEN):
- `src/ticket_sources/{mod.rs, free_text.rs}`.

**Refactor**: None.

**Commit**: `feat(core): ticket source trait with free-text impl`

**Verification**: `cargo test -p agentic-core ticket_sources::`

---

### Step 8.2: GitHub Issues ticket source (using wiremock)

**Goal**: Fetch an issue from github.com or GHES, including ACs parsed from body.

**Depends on**: Step 8.1.

**Test first** (RED):
- `tests/ticket_github.rs` with `wiremock`:
  - 200 OK issue JSON → parsed ticket.
  - 404 → `CoreError::TicketSource(NotFound)`.
  - 401 → `CoreError::Auth`.
  - GHES host in config hits the correct base URL.

**Implement** (GREEN):
- Deps: `reqwest` (features `rustls-tls`, `json`), `wiremock` (dev-dep).
- `src/ticket_sources/github.rs`.
- AC parser: look for `## Acceptance Criteria` section; fall back to description.

**Refactor**: Extract shared HTTP client factory.

**Commit**: `feat(core): github issues ticket source`

**Verification**: `cargo test -p agentic-core ticket_sources::github::`

---

### Step 8.3: GitLab Issues ticket source

**Goal**: Analogous to 8.2 for GitLab (cloud + self-hosted base URL).

**Depends on**: Step 8.2.

**Test first** (RED): wiremock fixtures for GitLab issue JSON.

**Implement** (GREEN): `src/ticket_sources/gitlab.rs`.

**Refactor**: None.

**Commit**: `feat(core): gitlab issues ticket source`

**Verification**: `cargo test -p agentic-core ticket_sources::gitlab::`

---

### Step 8.4: Jira ticket source

**Goal**: Fetch a Jira Cloud issue by key (e.g. `PROJ-123`). AC parsing reads the `customfield_*` for AC if configured, else the description.

**Depends on**: Step 8.3.

**Decision point**: which `customfield_*` id holds AC varies per Jira instance. Implementer should expose a `jira.ac_custom_field` setting (spec §14.4) and default to description when unset. Note in commit body.

**Test first** (RED): wiremock fixtures for Jira issue JSON, both with and without custom field.

**Implement** (GREEN): `src/ticket_sources/jira.rs`.

**Refactor**: None.

**Commit**: `feat(core): jira ticket source with configurable AC field`

**Verification**: `cargo test -p agentic-core ticket_sources::jira::`

---

### CP-6: Review backends + ticket sources

**Checkpoint**: Stop.
- Verify both Copilot and Claude adapters pass their end-to-end fixture tests.
- Verify all three remote ticket sources are wiremock-tested.
- Decide whether Jira AC custom-field support blocks MVP or can be a follow-up note.

---

## Phase 9 — Core: auth

### Step 9.1: Keyring abstraction + in-memory test impl

**Goal**: `SecretStore` trait with `get/set/delete(key: &str)`, backed by `keyring` in prod, `MemSecretStore` in tests.

**Depends on**: CP-6.

**Test first** (RED):
- `tests/secret_store.rs`:
  - `MemSecretStore`: set/get/delete behave correctly.
  - Deleting a missing key returns `Ok(())`.

**Implement** (GREEN):
- Dep: `keyring`.
- `src/auth/secrets.rs`.

**Refactor**: None.

**Commit**: `feat(core): secret store abstraction`

**Verification**: `cargo test -p agentic-core auth::secrets::`

---

### Step 9.2: PKCE helpers

**Goal**: Generate `code_verifier`, `code_challenge`, and `state`.

**Depends on**: Step 9.1.

**Test first** (RED):
- `tests/pkce.rs`:
  - `code_verifier` length between 43 and 128.
  - `code_challenge == base64url(sha256(code_verifier))`.
  - `state` is 128+ bits of entropy (regex/length check).
  - Proptest: generating 1000 verifiers yields 1000 distinct challenges.

**Implement** (GREEN):
- Deps: `sha2`, `base64` (or `base64ct`), `rand` (use `rand::rngs::OsRng`).
- `src/auth/pkce.rs`.

**Refactor**: None.

**Commit**: `feat(core): pkce helpers with proptest`

**Verification**: `cargo test -p agentic-core auth::pkce::`

---

### Step 9.3: Loopback listener

**Goal**: Start an ephemeral `axum` server on a random localhost port; return `(port, Future<Result<CallbackQuery>>)`; valid callback resolves; invalid path returns 404; timeout cancels.

**Depends on**: Step 9.2.

**Test first** (RED):
- `tests/loopback.rs`:
  - Post-start: port is in the ephemeral range.
  - `GET /callback?code=abc&state=xyz` resolves the future with `CallbackQuery { code, state }`.
  - `GET /other` returns 404 and does not resolve.
  - Timeout path returns `CoreError::Auth(Timeout)`.

**Implement** (GREEN):
- Deps: `axum`, `tokio` (rt + macros + time).
- `src/auth/loopback.rs`.

**Refactor**: None.

**Commit**: `feat(core): pkce loopback listener`

**Verification**: `cargo test -p agentic-core auth::loopback::`

---

### Step 9.4: Token exchange (GitHub)

**Goal**: Exchange `code + verifier` for a token at GitHub; wiremock-tested.

**Depends on**: Step 9.3.

**Test first** (RED):
- `tests/token_exchange_github.rs` with wiremock:
  - Valid exchange → `AccessToken { token, refresh_token, expires_at }`.
  - 400 invalid_grant → `CoreError::Auth`.
  - State mismatch at validation time rejects the callback before exchange.

**Implement** (GREEN):
- `src/auth/oauth_github.rs`.

**Refactor**: None.

**Commit**: `feat(core): github oauth token exchange`

**Verification**: `cargo test -p agentic-core auth::oauth_github::`

---

### Step 9.5: Token exchange (GitLab)

**Goal**: Analogous to 9.4 for GitLab.

**Depends on**: Step 9.4.

**Test first** (RED): wiremock happy + error paths.

**Implement** (GREEN): `src/auth/oauth_gitlab.rs`.

**Refactor**: Share a generic `OAuthExchanger` if duplication warrants.

**Commit**: `feat(core): gitlab oauth token exchange`

**Verification**: `cargo test -p agentic-core auth::oauth_gitlab::`

---

### Step 9.6: Device code flow fallback

**Goal**: Implement the OAuth device code flow for headless/SSH environments (spec §15.5).

**Depends on**: Step 9.4.

**Test first** (RED): wiremock stub that returns `slow_down` then `authorization_pending` then success; assert backoff and success resolution.

**Implement** (GREEN): `src/auth/device_code.rs`.

**Refactor**: None.

**Commit**: `feat(core): oauth device code fallback flow`

**Verification**: `cargo test -p agentic-core auth::device_code::`

---

### Step 9.7: CLI session delegate (`gh auth token`)

**Goal**: If `gh auth status` reports a valid session, optionally import the token via `gh auth token`.

**Depends on**: Step 9.1.

**Test first** (RED):
- With a faked `gh` shell script that prints a token, importer stores it under the right `auth_account_id`.
- With `gh` missing, returns `CoreError::Auth(NoExistingSession)`.

**Implement** (GREEN): `src/auth/gh_delegate.rs`.

**Refactor**: None.

**Commit**: `feat(core): import gh cli session as fallback auth`

**Verification**: `cargo test -p agentic-core auth::gh_delegate::`

---

### Step 9.8: Token refresh background task

**Goal**: A task that wakes within 5 min of `token_expires_at` and refreshes via the provider's refresh endpoint.

**Depends on**: Steps 9.4, 9.5.

**Test first** (RED): fake clock + wiremock; verify refresh at T-5m; on failure, marks account `needs_reauth`.

**Implement** (GREEN): `src/auth/refresh.rs`.

**Refactor**: None.

**Commit**: `feat(core): background token refresh with needs_reauth state`

**Verification**: `cargo test -p agentic-core auth::refresh::`

---

### CP-7: Review auth

**Checkpoint**: Stop.
- Verify loopback + exchange + refresh + delegate all individually tested.
- Decide: is the Jira 3LO flow going to reuse the GitHub loopback machinery or need its own? (Recommend reuse but ensure scopes + endpoints are configurable.)

---

## Phase 10 — Tauri shell (MVP scaffolding)

### Step 10.1: `agentic-tauri` crate + `agentic-core` dependency

**Goal**: Empty Tauri 2.x binary crate that `cargo build -p agentic-tauri` succeeds on.

**Depends on**: CP-7.

**Test first** (RED): `cargo build -p agentic-tauri` compiles; meta-tests assert `tauri.conf.json` identifier is `io.agentic.app` (or similar).

**Implement** (GREEN):
- Deps: `tauri = "2"`.
- `crates/agentic-tauri/{src/main.rs, tauri.conf.json, build.rs}`.
- Add to workspace members.

**Refactor**: None.

**Commit**: `feat(tauri): scaffold agentic-tauri crate`

**Verification**: `cargo build -p agentic-tauri && cargo test -p agentic-meta-tests tauri_conf`

---

### Step 10.2: Web UI scaffold (Vite + React + TypeScript)

**Goal**: `apps/web-ui` builds an empty SPA that Tauri can serve.

**Depends on**: Step 10.1.

**Decision point** (flag for implementer): Spec §25.5 lists React vs Svelte as open. Default to React for ecosystem; Svelte is acceptable if the implementer has a strong reason — must note in commit body and raise an ADR.

**Test first** (RED):
- `apps/web-ui/src/__tests__/app.test.tsx` via `vitest` + `@testing-library/react`: renders "Agentic" heading.
- `pnpm --filter @agentic/web-ui test` wired through a workspace script.

**Implement** (GREEN):
- `pnpm --filter @agentic/web-ui add` React, Vite, Vitest, RTL, Tailwind.
- `apps/web-ui/src/App.tsx`, `main.tsx`, `index.html`.

**Refactor**: None.

**Commit**: `feat(web): scaffold react + vite + vitest for tauri ui`

**Verification**: `pnpm --filter @agentic/web-ui test` (manual until CI is wired).

---

### Step 10.3: Tauri IPC — `subscribe_events`

**Goal**: Tauri command that subscribes to core `EventBus` and emits events to the frontend via `window.emit`.

**Depends on**: Steps 10.1, 2.2.

**Test first** (RED):
- Rust-side `tests/tauri_ipc.rs`: using Tauri's mock runtime, invoke the command and assert it emits at least one event within a deadline when a test bus publishes.

**Implement** (GREEN):
- `crates/agentic-tauri/src/commands/events.rs`.

**Refactor**: None.

**Commit**: `feat(tauri): subscribe_events command forwards bus to webview`

**Verification**: `cargo test -p agentic-tauri`

---

### Step 10.4: Event viewer panel (React)

**Goal**: A scrollable list that renders every event received. Primitive; no styling beyond Tailwind.

**Depends on**: Steps 10.2, 10.3.

**Test first** (RED):
- Vitest: component receives three mock events and renders three rows in order.
- Supplies a mocked `useTauriEvents()` hook.

**Implement** (GREEN):
- `apps/web-ui/src/components/EventList.tsx`.

**Refactor**: None.

**Commit**: `feat(web): event list component with tauri bridge`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### CP-8: Milestone 3 — Tauri app renders a scripted run's events

**Checkpoint**: Stop.
- Launch `pnpm --filter @agentic/web-ui dev` + `cargo tauri dev`.
- Run a scripted pipeline from the UI (simple "Start scripted run" button wired via Tauri command).
- Confirm events appear as they stream.

---

## Phase 11 — Tauri shell: cockpit + chat MVP

### Step 11.1: Cockpit stepper component

**Goal**: Render a four-step stepper (architect → tdd → qa → reviewer) with status icons per spec §7.1/§12.3.

**Depends on**: CP-8.

**Test first** (RED): Vitest — given a `RunState` with `qa.status = 'failed'`, the third step renders the failure icon; total token count is summed.

**Implement** (GREEN): `apps/web-ui/src/components/Stepper.tsx`.

**Refactor**: None.

**Commit**: `feat(web): cockpit stepper component`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### Step 11.2: Chat pane MVP

**Goal**: Chat input + message list bound to `chat_messages` via a Tauri command.

**Depends on**: Step 11.1.

**Test first** (RED): submit "hello" from the input → command is called with that body; reply appears in the message list.

**Implement** (GREEN):
- New Tauri command `chat_send_message`.
- React `ChatPane.tsx` + `useChat` hook.

**Refactor**: None.

**Commit**: `feat(web): chat pane mvp with tauri message send`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### Step 11.3: Slash-command routing (`/plan`, `/status`, `/cancel`)

**Goal**: Parse leading `/<cmd> args` and dispatch to typed handlers per spec §16.4. Start with three commands.

**Depends on**: Step 11.2.

**Test first** (RED): unit test a pure `parseSlashCommand` function across success and malformed inputs (`/plan` with no args → validation error; `/plan #42` → structured command).

**Implement** (GREEN): `apps/web-ui/src/slash/parser.ts` + dispatcher.

**Refactor**: None.

**Commit**: `feat(web): slash command parser and dispatcher`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### Step 11.4: `@mention` routing (`@architect`)

**Goal**: Parse leading `@<agent>` and dispatch a single-agent run through core.

**Depends on**: Step 11.3, 3.2.

**Test first** (RED): parser recognizes `@architect rest of message`; dispatcher invokes a single `Backend.execute` call (via a new Tauri command); result streams into chat, not cockpit.

**Implement** (GREEN): `mention` parser + Tauri command `mention_agent`.

**Refactor**: None.

**Commit**: `feat(web): @mention routing for single-agent runs`

**Verification**: `pnpm --filter @agentic/web-ui test && cargo test -p agentic-tauri`

---

### Step 11.5: Findings table + triage buttons

**Goal**: Render findings from a run with `[Fix] [Tech-debt] [Ignore]` actions wired to a Tauri command that updates the row.

**Depends on**: Step 11.1, 1.5.

**Test first** (RED): Vitest — clicking `[Tech-debt]` invokes the command with `triage='tech-debt'`; Rust test — command updates the row.

**Implement** (GREEN):
- `apps/web-ui/src/components/FindingsTable.tsx`.
- Tauri command `triage_finding`.

**Refactor**: None.

**Commit**: `feat(tauri): findings table with triage actions`

**Verification**: `pnpm --filter @agentic/web-ui test && cargo test -p agentic-tauri`

---

### CP-9: Milestone 4 — Tauri happy-path run-through with scripted backend

**Checkpoint**: Stop.
- Walk a scripted pipeline end-to-end in the UI: start from chat, watch cockpit update, triage a finding.
- Decide diff viewer direction (Monaco vs `@git-diff-view/react`, per spec §25.3) before Phase 13.

---

## Phase 12 — TUI shell (MVP scaffolding through cockpit parity)

### Step 12.1: `agentic-tui` crate scaffolding

**Goal**: Binary that opens an alt-screen buffer, renders a "Hello Agentic" in ratatui, and exits on `q`.

**Depends on**: CP-7.

**Test first** (RED): `tests/tui_smoke.rs` using `ratatui::backend::TestBackend`: the first frame contains "Agentic".

**Implement** (GREEN):
- Deps: `ratatui`, `crossterm`.
- `crates/agentic-tui/src/main.rs`.

**Refactor**: None.

**Commit**: `feat(tui): scaffold ratatui binary with test backend`

**Verification**: `cargo test -p agentic-tui`

---

### Step 12.2: Pane layout (chat + cockpit, `Tab` switch, `[` / `]` resize)

**Goal**: Two vertical panes; focus switch and resize via keys per spec §7.2.

**Depends on**: Step 12.1.

**Test first** (RED): snapshot tests via `insta` with the test backend — initial 50/50 layout; after `]`: 60/40; `Tab`: focus badge moves.

**Implement** (GREEN): `src/layout.rs`, `src/app.rs`.

**Refactor**: Extract `AppState` from `App`.

**Commit**: `feat(tui): two-pane layout with resize and focus`

**Verification**: `cargo test -p agentic-tui`

---

### Step 12.3: Event subscription + cockpit rendering

**Goal**: The TUI subscribes to the core bus and renders events in the cockpit pane (mirroring the Tauri event viewer, but ratatui).

**Depends on**: Steps 12.2, 2.2.

**Test first** (RED): simulate a scripted run; snapshot shows four rows in the stepper with the expected status icons.

**Implement** (GREEN): `src/views/cockpit.rs`.

**Refactor**: None.

**Commit**: `feat(tui): cockpit renders events from core bus`

**Verification**: `cargo test -p agentic-tui`

---

### Step 12.4: Command mode (`:plan`, `:status`, `:q`)

**Goal**: `:` enters command mode, typing runs slash-command equivalents per spec §7.2/§16.4.

**Depends on**: Step 12.3, 3.5.

**Test first** (RED): pressing `:plan "hello"` triggers a scripted run in the test harness and the cockpit populates.

**Implement** (GREEN): `src/modes.rs`.

**Refactor**: None.

**Commit**: `feat(tui): command mode with plan/status/q commands`

**Verification**: `cargo test -p agentic-tui`

---

### Step 12.5: Findings table with keyboard triage (`f`/`t`/`i`)

**Goal**: Findings keyboard triage per spec §16.6.

**Depends on**: Step 12.4, 1.5.

**Test first** (RED): seed findings; press `j` twice + `t` → row 3 transitions to `tech-debt`.

**Implement** (GREEN): `src/views/findings.rs`.

**Refactor**: None.

**Commit**: `feat(tui): findings table with keyboard triage`

**Verification**: `cargo test -p agentic-tui`

---

### CP-10: Milestone 5 — TUI reaches cockpit parity with Tauri on scripted runs

**Checkpoint**: Stop.
- Run `agentic-tui` in a real terminal.
- Verify `:plan`, cockpit rendering, triage, `:q`.

---

## Phase 13 — Diff viewing + file changes surface

### Step 13.1: Unified diff renderer in TUI

**Goal**: Show `file_changes.diff` as inline unified diff with +/- coloring per spec §7.2.

**Depends on**: CP-10, 6.4.

**Test first** (RED): given a patch fixture, the renderer produces the expected styled lines (snapshot).

**Implement** (GREEN): `src/views/diff.rs` using `syntect` for syntax colors.

**Refactor**: None.

**Commit**: `feat(tui): unified diff viewer with syntax highlighting`

**Verification**: `cargo test -p agentic-tui`

---

### Step 13.2: Diff viewer in Tauri

**Goal**: Embedded diff viewer for `FileChange` events.

**Depends on**: CP-9, 6.4.

**Decision point**: pick Monaco or `@git-diff-view/react` per spec §25.3. Record in commit body or ADR.

**Test first** (RED): Vitest — component receives a patch string and renders two panes (before/after) or one unified view; test the prop/contract, not the visual.

**Implement** (GREEN): `apps/web-ui/src/components/DiffViewer.tsx`.

**Refactor**: None.

**Commit**: `feat(web): diff viewer component`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

## Phase 14 — VS Code extension (MVP scaffolding)

### Step 14.1: napi-rs bindings for core (`agentic-node`)

**Goal**: A native module exposing `startRun`, `subscribeEvents` (async iterator), `triageFinding` as N-API functions.

**Depends on**: CP-7, 4.3.

**Decision point** (flag for implementer): napi-rs vs WASM per spec §25.4. Default to napi-rs (better streaming perf); reserve WASM as a fallback build.

**Test first** (RED):
- `crates/agentic-node/__tests__/smoke.test.ts` using `vitest` + `@napi-rs/cli`: loading the module works; `startRun` with scripted inputs yields events via async iterator.

**Implement** (GREEN):
- `crates/agentic-node/{Cargo.toml, src/lib.rs, package.json, build.rs}` with `napi-rs` derive macros.

**Refactor**: None.

**Commit**: `feat(node): napi-rs bindings for core`

**Verification**: `pnpm --filter @agentic/node test`

---

### Step 14.2: VS Code extension scaffolding

**Goal**: A minimal extension that activates on workspace open and registers a single command `Agentic: Hello`.

**Depends on**: Step 14.1.

**Test first** (RED):
- `apps/vscode-extension/src/__tests__/activation.test.ts` using `@vscode/test-electron`: extension activates and `Agentic: Hello` command is registered.

**Implement** (GREEN):
- `apps/vscode-extension/{package.json, tsconfig.json, src/extension.ts}`.
- `pnpm --filter @agentic/vscode-extension add vscode`.

**Refactor**: None.

**Commit**: `feat(vsx): scaffold vscode extension`

**Verification**: `pnpm --filter @agentic/vscode-extension test`

---

### Step 14.3: Sidebar view with chat webview

**Goal**: Activity bar icon opens sidebar; webview uses the same `web-ui` build.

**Depends on**: Step 14.2, CP-9.

**Test first** (RED): extension test — opening the sidebar creates a webview panel with the expected HTML.

**Implement** (GREEN):
- `apps/vscode-extension/src/views/sidebar.ts`.
- Share the `web-ui` build output via a `webviewPanel.webview.html` setter.

**Refactor**: None.

**Commit**: `feat(vsx): sidebar view with shared webview`

**Verification**: `pnpm --filter @agentic/vscode-extension test`

---

### Step 14.4: Slash commands → VS Code commands

**Goal**: Each slash command becomes a VS Code command (`Agentic: Plan…`, etc.) accessible via `Cmd+Shift+P`.

**Depends on**: Step 14.3.

**Test first** (RED): command registration present for the MVP set (plan, status, cancel, triage, answer, retry, resume, workspace, backend, model, settings, runs, pr, clear, help).

**Implement** (GREEN): `apps/vscode-extension/src/commands/` registering each.

**Refactor**: None.

**Commit**: `feat(vsx): register slash commands as vscode commands`

**Verification**: `pnpm --filter @agentic/vscode-extension test`

---

### Step 14.5: Native diff editor for `FileChange`

**Goal**: On `FileChange`, open the workspace file in `vscode.diff` against a virtual URI showing the `before_hash` snapshot content.

**Depends on**: Step 14.3, 6.4.

**Test first** (RED): mock `vscode` API test — on a `FileChange` event, `vscode.commands.executeCommand('vscode.diff', ...)` is called with the expected args.

**Implement** (GREEN): `apps/vscode-extension/src/diff.ts` + a `TextDocumentContentProvider` for `agentic://` URIs.

**Refactor**: None.

**Commit**: `feat(vsx): native diff editor on FileChange events`

**Verification**: `pnpm --filter @agentic/vscode-extension test`

---

### Step 14.6: Findings → editor decorations

**Goal**: Reviewer findings render as squiggles + hover tooltip with `[Fix] [Tech-debt] [Ignore]` actions.

**Depends on**: Step 14.5, 1.5.

**Test first** (RED): mock test — on a `Finding` event with a file path + line, `TextEditorDecorationType` is applied at that range; hover provider returns actions.

**Implement** (GREEN): `apps/vscode-extension/src/decorations.ts`.

**Refactor**: None.

**Commit**: `feat(vsx): findings as editor decorations with triage hover`

**Verification**: `pnpm --filter @agentic/vscode-extension test`

---

### CP-11: Milestone 6 — all three shells run a scripted pipeline end-to-end

**Checkpoint**: Stop.
- Tauri, TUI, and VS Code all process a scripted run.
- Decide whether to start real OAuth against github.com or polish the triage UX first.

---

## Phase 15 — Auth UX integration (across shells)

### Step 15.1: `Agentic: Sign in with GitHub` in Tauri + VS Code

**Goal**: Wire the PKCE loopback flow (step 9.3+9.4) behind a UI button.

**Depends on**: CP-11, 9.4.

**Test first** (RED): Tauri command test — clicking "Sign in" triggers the loopback listener, stores a fake token via `MemSecretStore`, updates `auth_accounts`.

**Implement** (GREEN): Tauri + VS Code wrappers around the core auth API; add `Agentic: Sign in with GitHub` command.

**Refactor**: Extract shared logic into the core so both shells call the same API.

**Commit**: `feat(auth): sign in with github from tauri and vscode`

**Verification**: `cargo test -p agentic-tauri && pnpm --filter @agentic/vscode-extension test`

---

### Step 15.2: GHES / self-hosted GitLab custom client ID dialog

**Goal**: When the user connects to a new GHES host, show the three-option dialog per spec §15.3.

**Depends on**: Step 15.1.

**Test first** (RED): Vitest — dialog renders three options; picking "BYO client id" stores the value in `auth.github_enterprise.hosts.<host>.client_id`.

**Implement** (GREEN): `apps/web-ui/src/components/AuthGhesDialog.tsx`; settings writer for the host entry.

**Refactor**: None.

**Commit**: `feat(auth): ghes custom client id dialog`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### Step 15.3: Jira Cloud OAuth

**Goal**: Wire the PKCE loopback flow against Atlassian OAuth 2.0 (3LO).

**Depends on**: Step 15.2, 9.4.

**Test first** (RED): wiremock for Atlassian endpoints; end-to-end token exchange.

**Implement** (GREEN): reuse the generic `OAuthExchanger`; add `src/auth/oauth_jira.rs`.

**Refactor**: None.

**Commit**: `feat(auth): jira cloud oauth with pkce loopback`

**Verification**: `cargo test -p agentic-core auth::oauth_jira::`

---

### Step 15.4: `needs_reauth` banner + recovery

**Goal**: When refresh fails or the token is invalidated, show a persistent banner in all shells with a re-auth CTA.

**Depends on**: Step 15.3, 9.8.

**Test first** (RED): unit — when `auth_accounts.needs_reauth = true`, the core `AuthStatus` API returns that state and the Tauri/VS Code banner mounts.

**Implement** (GREEN): `src/auth/status.rs` API; UI banners in Tauri + VS Code.

**Refactor**: None.

**Commit**: `feat(auth): needs_reauth banner across shells`

**Verification**: `cargo test && pnpm -r test`

---

## Phase 16 — Onboarding, preflight, crash recovery, cancellation

### Step 16.1: Profile auto-detection from `git remote`

**Goal**: On workspace open, parse the remote URL and propose the matching profile.

**Depends on**: CP-11, 1.9.

**Test first** (RED): unit — `github.com` → `github`; `gitlab.com` → `gitlab`; unknown → `custom`; `github.mycorp.com` (configured as GHES) → `github`.

**Implement** (GREEN): `src/profile/detect.rs`.

**Refactor**: None.

**Commit**: `feat(core): profile auto-detection from git remote`

**Verification**: `cargo test -p agentic-core profile::`

---

### Step 16.2: Onboarding wizard (Tauri)

**Goal**: Three-step wizard per spec §19.1.

**Depends on**: Step 16.1, 15.1.

**Test first** (RED): Vitest — each step renders and the final step triggers `Agentic: Sign in with GitHub`.

**Implement** (GREEN): `apps/web-ui/src/onboarding/Wizard.tsx`.

**Refactor**: None.

**Commit**: `feat(web): three-step onboarding wizard`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### Step 16.3: Preflight checks banner

**Goal**: If `claude`/`copilot` is missing, show a dismissible banner per spec §19.2.

**Depends on**: Step 16.2, 5.2.

**Test first** (RED): Vitest — when `doctor` API returns `claude: missing`, banner renders with install command copy button.

**Implement** (GREEN): `apps/web-ui/src/components/PreflightBanner.tsx`.

**Refactor**: None.

**Commit**: `feat(web): preflight banner for missing cli tools`

**Verification**: `pnpm --filter @agentic/web-ui test`

---

### Step 16.4: Crash detection on startup

**Goal**: On app startup, core detects runs stuck in `running` whose `subprocess_pid` is dead, offering Resume / Start new / Discard per spec §17.5.

**Depends on**: Step 16.1, 2.4.

**Test first** (RED):
- Unit: given a run row with `status=running` and a non-existent pid, `detect_crashes()` returns that run.
- Integration: resume re-runs the failed step from scratch while preserving prior step outputs.

**Implement** (GREEN): `src/pipeline/crash_recovery.rs`.

**Refactor**: None.

**Commit**: `feat(core): crash detection and resume prompt`

**Verification**: `cargo test -p agentic-core pipeline::crash_recovery::`

---

### Step 16.5: Cancellation end-to-end (UI → core → subprocess)

**Goal**: `/cancel` button in Tauri + `Ctrl-C` in TUI + `Agentic: Cancel Run` in VS Code, all routing through `CancellationToken` per spec §17.3.

**Depends on**: Step 16.4, 6.2.

**Test first** (RED): integration — starting a scripted run + pressing cancel results in `status=cancelled` within 6s; files unchanged.

**Implement** (GREEN): wire cancel buttons across shells; ensure `SIGTERM → 5s → SIGKILL` is honored.

**Refactor**: None.

**Commit**: `feat(core): end-to-end cancellation across shells`

**Verification**: `cargo test && pnpm -r test`

---

## Phase 17 — Release engineering

### Step 17.1: CI matrix extension (Windows + cross-compile check)

**Goal**: Extend `.github/workflows/test.yml` to include `windows-latest` + arm64 Linux cross-check.

**Depends on**: Step 0.6 (original CI).

**Test first** (RED): extend `ci_shape.rs` to assert the matrix includes `windows-latest`.

**Implement** (GREEN): update the workflow.

**Refactor**: None.

**Commit**: `ci: extend matrix to windows and arm64 linux`

**Verification**: `cargo test -p agentic-meta-tests ci_shape`

---

### Step 17.2: Release workflow for Tauri binaries

**Goal**: `.github/workflows/release.yml` builds signed `.dmg`/`.msi`/`.AppImage`/`.deb` on tag push.

**Depends on**: Step 17.1.

**Decision point** (flag for implementer): Signing requires secrets (Apple Developer cert, Windows EV cert). These must be added manually to GitHub secrets; note blocking dependencies in commit body.

**Test first** (RED): meta-test — workflow YAML parses; expected artifact names are referenced in `upload-artifact` steps.

**Implement** (GREEN): release workflow + `tauri-plugin-updater` config.

**Refactor**: None.

**Commit**: `ci: release workflow for tauri binaries`

**Verification**: `cargo test -p agentic-meta-tests release_shape`

---

### Step 17.3: TUI via cargo install + Homebrew tap

**Goal**: Document `cargo install agentic-tui` and add a Homebrew formula.

**Depends on**: Step 17.1.

**Test first** (RED): meta-test — `crates/agentic-tui/Cargo.toml` sets `description`, `license`, `repository`, `readme` for crates.io; a Homebrew formula file exists under `packaging/homebrew/agentic.rb`.

**Implement** (GREEN): update metadata + create the formula.

**Refactor**: None.

**Commit**: `chore(release): tui distribution metadata and homebrew formula`

**Verification**: `cargo test -p agentic-meta-tests tui_release_meta`

---

### Step 17.4: VS Code marketplace publish workflow

**Goal**: `.github/workflows/vsx-publish.yml` publishes to Marketplace + Open VSX on tag push.

**Depends on**: Step 17.1.

**Test first** (RED): meta-test — workflow YAML parses; uses `vsce publish` and `ovsx publish`.

**Implement** (GREEN): workflow file.

**Refactor**: None.

**Commit**: `ci: vscode extension publish workflow`

**Verification**: `cargo test -p agentic-meta-tests vsx_publish_shape`

---

### CP-12: Milestone 7 — Release candidate

**Checkpoint**: Stop.
- Dry-run a tagged release on a pre-release branch.
- Verify signed artifacts produced.
- Complete the 10-item success criteria checklist in spec §26 before cutting 1.0.

---

## Integration test pass

### Step I.1: End-to-end happy path against real `claude`

**Goal**: With a configured workspace + real `claude` login, run `/plan "free text"` and verify the full pipeline completes.

**Depends on**: CP-12.

**Test first** (RED): A gated integration test under `crates/agentic-core/tests/integration/` that requires `AGENTIC_E2E=1` to run.

**Implement** (GREEN): wiring only; no new production code.

**Refactor**: None.

**Commit**: `test(e2e): happy-path claude-backed pipeline run`

**Verification**: `AGENTIC_E2E=1 cargo test -p agentic-core --test integration`

---

### Step I.2: End-to-end happy path against real `copilot`

**Goal**: Same, for the GitLab profile + Copilot backend.

**Depends on**: Step I.1.

**Test first** (RED): gated test.

**Implement** (GREEN): wiring only.

**Refactor**: None.

**Commit**: `test(e2e): happy-path copilot-backed pipeline run`

**Verification**: `AGENTIC_E2E=1 cargo test -p agentic-core --test integration`

---

### Step I.3: Crash + resume + cancel E2E

**Goal**: Assert spec §26 criteria 5 (resume after crash) and 6 (cancel mid-run).

**Depends on**: Steps 16.4, 16.5, I.1.

**Test first** (RED): gated tests forcibly killing the subprocess mid-run, then restarting and asserting the resume prompt appears.

**Implement** (GREEN): wiring only.

**Refactor**: None.

**Commit**: `test(e2e): crash recovery and cancellation acceptance`

**Verification**: `AGENTIC_E2E=1 cargo test`

---

## Status checklist

- [ ] Phase 0 — scaffolding (0.1–0.6, CP-0)
- [ ] Phase 1 — persistence foundation (1.1–1.10, CP-1)
- [ ] Phase 2 — event model + bus (2.1–2.4, CP-2)
- [ ] Phase 3 — pipeline state machine (3.1–3.5, CP-3)
- [ ] Phase 4 — Backend trait + scripted backend (4.1–4.3, CP-4 = Milestone 1)
- [ ] Phase 5 — dev CLI (5.1, 5.2)
- [ ] Phase 6 — Claude adapter (6.1–6.4, CP-5 = Milestone 2)
- [ ] Phase 7 — Copilot adapter (7.1–7.4)
- [ ] Phase 8 — ticket sources (8.1–8.4, CP-6)
- [ ] Phase 9 — auth (9.1–9.8, CP-7)
- [ ] Phase 10 — Tauri scaffolding (10.1–10.4, CP-8 = Milestone 3)
- [ ] Phase 11 — Tauri cockpit + chat (11.1–11.5, CP-9 = Milestone 4)
- [ ] Phase 12 — TUI (12.1–12.5, CP-10 = Milestone 5)
- [ ] Phase 13 — diff viewing (13.1, 13.2)
- [ ] Phase 14 — VS Code extension (14.1–14.6, CP-11 = Milestone 6)
- [ ] Phase 15 — auth UX (15.1–15.4)
- [ ] Phase 16 — onboarding + recovery (16.1–16.5)
- [ ] Phase 17 — release (17.1–17.4, CP-12 = Milestone 7)
- [ ] Integration (I.1–I.3)

---

## Planning notes for implementer

Decisions the architect deferred to the tdd-developer (each surfaced at its step, collected here for visibility):

1. **Step 1.3 — Migration runner**: hand-rolled vs `refinery`. Recommendation: hand-rolled, to keep core dep-light. Swap later via migration.
2. **Step 2.3 — Event payload encoding**: MessagePack (`rmp-serde`) per spec, but JSON-in-BLOB is an acceptable debug-first alternative. Reversible by migration.
3. **Step 6.4 — Diff library**: `similar` vs `diffy`. Default `similar` for broader feature surface.
4. **Step 7.1 — Copilot stream schema**: spec §25.1 explicitly defers; implementer must record fixtures first.
5. **Step 8.4 — Jira AC custom field**: configurable per instance; default falls back to description parsing.
6. **Step 10.2 — Frontend framework**: React (default) vs Svelte, per spec §25.5. React unless the implementer has a reason; raise an ADR if Svelte.
7. **Step 13.2 — Diff viewer**: Monaco vs `@git-diff-view/react`, per spec §25.3. Decide based on bundle size measurements.
8. **Step 14.1 — VS Code bridge**: napi-rs (default) vs WASM, per spec §25.4. Keep WASM in mind as fallback.
9. **Step 17.2 — Signing**: requires Apple Developer + Windows EV certs via GitHub secrets; non-code prerequisite.

Cross-cutting reminders:

- **Schema versioning**: consider adding `schema_version: u32` to `EventEnvelope` at Step 2.1 time — cheaper than a migration later.
- **`Error.code` taxonomy** (spec §25.6): needs a dedicated ADR before Step 6.3 fully lands, since UI error copy will key off these strings.
- **Migration from the existing `agentic-orchestration` repo** (spec §25.7): a dedicated post-MVP step not yet scheduled; flag for the user's roadmap.
- **Windows subprocess semantics**: Steps 6.2, 7.3, 16.5 need a `cfg(windows)` branch for signal handling (`TerminateProcess` instead of SIGTERM). Tests gated accordingly.
- **Tauri + keyring on Linux**: the `libsecret` backend requires a running daemon. Document in README before Phase 9 ships.
- **Feature flags for shell builds**: release builds should not include `scripted` backend; gate behind `#[cfg(any(test, feature = "testing"))]` as noted in Step 4.2.
