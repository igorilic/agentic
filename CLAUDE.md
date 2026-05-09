# Agentic — Session Workflow Rules

These rules apply to every session working on this repo. Future sessions
must read this file at start and follow the flow without restating it.

## 1. Plan before executing

Before touching code on any todo.md step, post a short plan to the
user (≤8 bullets) covering: contract being delivered, files
created/changed, test fixtures, deferred scope. If scope is non-trivial
or has a decision point, **wait for confirmation** before starting.

Skip the plan only for trivial follow-ups (single typo / one-liner the
user just dictated).

## 2. Execute with the `tdd-developer` agent

Implementation work runs through the `tdd-developer` subagent:
- One step from `todo.md` per invocation.
- RED test first → GREEN minimum implementation → REFACTOR → commit.
- The agent commits its own work.

Don't write code directly in the main session for a planned step. The
exception is mechanical follow-ups (clippy fixups, fmt, doc tweaks)
that the agent's commit didn't include.

## 3. Verify with `qa` + `reviewer` after every step

After `tdd-developer` returns, **always** spawn `qa` and `reviewer` in
parallel (single message, two `Agent` tool calls):

- **qa**: runs the affected test suites, `cargo clippy`, `cargo fmt --check`,
  `pnpm test` for changed JS workspaces. Reports pass/fail.
- **reviewer**: reviews the diff against spec.md, todo.md contracts,
  CLAUDE.md rules, and conventions. Produces a punch list grouped as
  **fix / tech-debt / ignore**. Don't auto-apply — surface to user for
  triage.

User triages findings. Apply fix / tech-debt / ignore decisions. Up to
3 review-and-fix loops per step before remaining items become tech-debt.

## 4. Tech-debt is for genuinely deferred scope only

Before logging an item to `todo.md` tech-debt, ask: **"could I have
finished this in the current step's scope without inflating it?"**

If yes — finish it now. Tech-debt is not a release valve for
half-implementations. Categories that must NOT be deferred:

- **Half-done plumbing**: state populated but no fn exposes it; setter
  exists but no getter; fields added but no callers wired.
- **Test coverage gaps that the step's contract requires**: if the
  reviewer would call out "test asserts X passes but doesn't verify
  the side-effect actually landed", finish the assertion.
- **Trivial polish items < 30 minutes**: aria-labels, key fixes,
  consistent naming.

Categories that **may** be deferred (with tech-debt note):

- New crate dependencies that materially affect bundle size / build matrix.
- Cross-cutting refactors that touch multiple steps' surfaces.
- Verification steps that depend on infrastructure not present locally
  (CI on other architectures, real OAuth providers, etc.).

When deferring, write the tech-debt entry with: **what's missing**,
**why it's deferred** (concrete reason, not "future work"), and a
**trigger** for when it should be picked up.

**Always file a GitHub issue for the deferred item** (`gh issue create`
in the active repo). Use a `tech-debt` label. The issue body mirrors
the todo.md entry: what's missing, why deferred, the trigger. Link the
issue back from the todo.md tech-debt entry as `(GH #N)`. The issue
is the durable tracker; the todo.md entry is a quick-glance index.

## 5. Commits + push

The `tdd-developer` agent's commits use Conventional Commits with a
body explaining "why" (not just "what"). After review-fix loops, push
to `main`. Don't squash multiple steps into one commit.

## 6. Where the global pipeline rules live

`~/.claude/CLAUDE.md` documents the architect → tdd-developer → qa →
reviewer pipeline at the global level. This file is the project-level
amendment: same flow, plus the explicit tech-debt discipline above.

## 7. Stack conventions (auto-detected, kept here for reference)

- Rust 2024 workspace (`Cargo.toml` resolver = "3"). Crates under `crates/*`.
- React + Vite + Vitest + Testing Library + Tailwind for `apps/web-ui`.
- pnpm for JS workspaces.
- ratatui + crossterm for `crates/agentic-tui`.
- Tauri 2.x for `crates/agentic-tauri`.
- Workspace tests: `cargo test --workspace --all-features` + per-app `pnpm test`.
- Lint gates: `cargo clippy --workspace --all-features --all-targets -- -D warnings`,
  `cargo fmt --all -- --check`.
