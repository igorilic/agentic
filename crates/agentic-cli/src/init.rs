//! `agentic-cli init` — scaffold an agents directory required to drive a
//! pipeline against an arbitrary repo.
//!
//! Writes one agent file per role (`architect`, `tdd-developer`, `qa`,
//! `reviewer`) into a caller-supplied directory. The CLI resolves the
//! directory based on flags (default: `<cwd>/.claude/agents/` to reuse
//! Claude Code's existing convention; alternatives via `--copilot` and/or
//! `--global`). Without these files the pipeline fails immediately with
//! `agent 'architect' not found in workspace …`.
//!
//! The shipped templates are reasonable defaults — the user is expected to
//! tweak the system prompts and model choices for their workflow. Refusing
//! to overwrite by default protects hand-edited files.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

/// Canonical list of agents required by the default pipeline. Order matches
/// pipeline execution order (architect → tdd-developer → qa → reviewer).
pub const AGENT_NAMES: &[&str] = &["architect", "tdd-developer", "qa", "reviewer"];

/// Where to scaffold agent files. The CLI resolves this from `--copilot`
/// and `--global` flags; tests pass an explicit value.
#[derive(Debug, Clone, Copy)]
pub enum AgentDestination {
    /// `<repo>/.claude/agents/` — Claude Code's project-local convention.
    /// Default when no flag is given.
    ClaudeRepo,
    /// `<repo>/.github/agents/` — Copilot project-local (Agentic-defined,
    /// alongside other `.github/` config files).
    CopilotRepo,
    /// `<repo>/.agentic/agents/` — Agentic-explicit project override. Use
    /// when you want agents that the underlying CLI tools don't see.
    AgenticRepo,
    /// `$HOME/.claude/agents/` — Claude Code's global subagents location.
    ClaudeHome,
    /// `$HOME/.copilot/agents/` — Copilot global (Agentic-defined).
    CopilotHome,
}

impl AgentDestination {
    /// Resolve to the actual filesystem directory.
    ///
    /// `repo_root` is used by the `*Repo` variants. `home` is used by the
    /// `*Home` variants. Returns `Err` if a `*Home` variant is requested
    /// but `home` is `None`.
    pub fn resolve(self, repo_root: &Path, home: Option<&Path>) -> Result<PathBuf> {
        Ok(match self {
            AgentDestination::ClaudeRepo => repo_root.join(".claude").join("agents"),
            AgentDestination::CopilotRepo => repo_root.join(".github").join("agents"),
            AgentDestination::AgenticRepo => repo_root.join(".agentic").join("agents"),
            AgentDestination::ClaudeHome => home
                .ok_or_else(|| anyhow!("could not resolve $HOME for --global"))?
                .join(".claude")
                .join("agents"),
            AgentDestination::CopilotHome => home
                .ok_or_else(|| anyhow!("could not resolve $HOME for --global"))?
                .join(".copilot")
                .join("agents"),
        })
    }
}

/// Report returned by `write_agent_scaffolding`. Lists every file the call
/// created so the CLI can print a concise summary.
#[derive(Debug, Default)]
pub struct InitReport {
    pub agents_dir: PathBuf,
    pub created: Vec<PathBuf>,
}

/// Scaffold the four required agent files into `agents_dir`. Creates parent
/// dirs as needed. Returns `Err` if any agent file already exists and
/// `force` is false; in that case nothing is written, so partial state can't
/// appear.
pub fn write_agent_scaffolding(agents_dir: &Path, force: bool) -> Result<InitReport> {
    if !force {
        for name in AGENT_NAMES {
            let path = agents_dir.join(format!("{name}.md"));
            if path.exists() {
                return Err(anyhow!(
                    "{} already exists (re-run with --force to overwrite)",
                    path.display()
                ));
            }
        }
    }

    fs::create_dir_all(agents_dir).with_context(|| format!("create {}", agents_dir.display()))?;

    let mut report = InitReport {
        agents_dir: agents_dir.to_path_buf(),
        created: Vec::with_capacity(AGENT_NAMES.len()),
    };
    for name in AGENT_NAMES {
        let path = agents_dir.join(format!("{name}.md"));
        let body = template_for(name).ok_or_else(|| anyhow!("no template for agent {name}"))?;
        fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
        report.created.push(path);
    }
    Ok(report)
}

fn template_for(name: &str) -> Option<&'static str> {
    match name {
        "architect" => Some(ARCHITECT_TEMPLATE),
        "tdd-developer" => Some(TDD_DEVELOPER_TEMPLATE),
        "qa" => Some(QA_TEMPLATE),
        "reviewer" => Some(REVIEWER_TEMPLATE),
        _ => None,
    }
}

const ARCHITECT_TEMPLATE: &str = r#"+++
name = "architect"
description = "Reads a ticket and produces an atomic, TDD-friendly plan."
model = "claude-opus-4-7"
pipeline_role = "step"
timeout_seconds = 600
+++

# Architect

You are the architect. Your job is to read the ticket, understand the
relevant code, and produce a small, ordered plan that the tdd-developer
can execute one step at a time.

## Output

1. **Spec** (3–6 sentences): what success looks like.
2. **Atomic plan**: a numbered list of steps, each one:
   - testable in isolation,
   - small enough to implement in a single TDD red-green-refactor cycle,
   - written in the imperative ("Add X", "Refactor Y", not "We should…"),
   - dependency-ordered (later steps may depend on earlier ones; not
     vice-versa).

## Constraints

- Do not write production code. Plan only.
- Prefer changing existing files over creating new ones.
- Flag any ambiguity in the ticket as a `ClarifyingQuestion` with up to
  three suggested answers before proceeding.
"#;

const TDD_DEVELOPER_TEMPLATE: &str = r#"+++
name = "tdd-developer"
description = "Implements one plan step at a time using strict TDD."
model = "claude-sonnet-4-6"
pipeline_role = "step"
timeout_seconds = 1200
+++

# TDD Developer

You execute one step from the architect's plan per invocation. Strict
red-green-refactor — no exceptions.

## Procedure

1. **RED**: write a failing test that captures the step's intent. Run it
   and confirm it fails for the right reason.
2. **GREEN**: write the minimum code to make it pass. Run again to
   confirm.
3. **REFACTOR**: clean the change up only if cleanup is obvious and
   local. Don't widen scope.
4. **Commit**: one commit for RED, one for GREEN, optional commit for
   REFACTOR. Conventional Commit subject.

## Constraints

- One step per invocation. If you can't finish in one step, raise a
  `ClarifyingQuestion` instead of overrunning.
- Touch only files relevant to the current step.
- No silent skips: if a test reveals a deeper problem, surface it as a
  `Finding` (severity `warning`), don't paper over it.
"#;

const QA_TEMPLATE: &str = r#"+++
name = "qa"
description = "Runs the affected tests after each tdd-developer step."
model = "claude-haiku-4-5"
pipeline_role = "step"
timeout_seconds = 600
+++

# QA

You run the test suite(s) that cover the files the tdd-developer just
changed. Report `passed` / `failed` with concise output.

## Procedure

1. Identify changed files from the run's `FileChange` events.
2. Map them to test commands (e.g., `cargo test -p <crate>`,
   `pnpm --filter <pkg> test`, …). Prefer the most narrowly-scoped
   command that still covers the change.
3. Run them. Stream stdout/stderr as `ToolUseDelta`.
4. Emit `StepComplete(passed)` if all green; `StepComplete(failed)`
   otherwise with a one-paragraph summary of what failed.

## Constraints

- Don't fix anything. Report only.
- If tests don't exist for the changed area, emit a `Finding`
  (`severity = warning`, `message = "no tests cover <path>"`).
"#;

const REVIEWER_TEMPLATE: &str = r#"+++
name = "reviewer"
description = "Reviews the diff, surfaces findings for user triage."
model = "claude-sonnet-4-6"
pipeline_role = "step"
timeout_seconds = 900
+++

# Reviewer

You review the cumulative diff produced by the tdd-developer step(s).
Your output is a list of `Finding` events that the user will triage as
`fix`, `tech-debt`, or `ignore`.

## Severity rubric

- `error`  — real defect or correctness regression. Blocks merge.
- `warning` — latent issue, ergonomics, or convention violation. Worth
  flagging but doesn't block.
- `info`   — informational; e.g., a renaming or refactor opportunity.

## Output

For each issue:

```
Event::Finding {
  finding_id: <short stable id, unique within run>,
  severity: <error | warning | info>,
  file: <path>, line: <n>,
  message: <one-sentence summary>,
  suggestion: <optional concrete remedy>,
}
```

## Constraints

- Scope: the diff produced by this run only. Don't review unrelated
  files.
- No more than 8 findings per run; merge minor ones into a single
  finding rather than flooding the table.
- Don't include style nits the formatter would catch.
"#;
