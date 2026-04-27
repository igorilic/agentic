//! Integration tests for `agentic-cli init`.
//!
//! Scaffolds `.agentic/agents/{architect,tdd-developer,qa,reviewer}.md` in
//! a target directory so the pipeline finds its agents on first run.

use agentic_cli::init::{AGENT_NAMES, write_agent_scaffolding};
use std::fs;

fn make_target() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn writes_all_four_agent_files_under_target_dot_agentic_agents() {
    let tmp = make_target();
    let target = tmp.path();

    write_agent_scaffolding(target, false).expect("init should succeed on empty dir");

    for name in AGENT_NAMES {
        let path = target
            .join(".agentic")
            .join("agents")
            .join(format!("{name}.md"));
        assert!(path.exists(), "expected {} to be created", path.display());
        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.starts_with("+++"),
            "{} should start with TOML frontmatter fence",
            path.display()
        );
        assert!(
            body.contains(&format!("name = \"{name}\"")),
            "{} should declare matching name",
            path.display()
        );
        assert!(
            body.contains("pipeline_role = \"step\""),
            "{} should declare pipeline_role = step",
            path.display()
        );
    }
}

#[test]
fn refuses_to_overwrite_existing_files_without_force() {
    let tmp = make_target();
    let target = tmp.path();
    let architect = target.join(".agentic").join("agents").join("architect.md");

    // First call creates the files.
    write_agent_scaffolding(target, false).unwrap();
    let original = fs::read_to_string(&architect).unwrap();

    // Second call without --force should error and leave content alone.
    let result = write_agent_scaffolding(target, false);
    assert!(result.is_err(), "second init without --force must fail");

    let after = fs::read_to_string(&architect).unwrap();
    assert_eq!(
        original, after,
        "file content must not change on failed init"
    );
}

#[test]
fn force_overwrites_existing_files() {
    let tmp = make_target();
    let target = tmp.path();
    let architect = target.join(".agentic").join("agents").join("architect.md");

    write_agent_scaffolding(target, false).unwrap();
    // Tamper with the file so we can detect overwrite.
    fs::write(&architect, "stale content").unwrap();

    write_agent_scaffolding(target, true).expect("--force should succeed");

    let body = fs::read_to_string(&architect).unwrap();
    assert!(
        body.starts_with("+++"),
        "force should restore the canonical template"
    );
}

#[test]
fn lists_what_was_written_so_callers_can_print_a_report() {
    let tmp = make_target();
    let report = write_agent_scaffolding(tmp.path(), false).expect("init");

    assert_eq!(
        report.created.len(),
        AGENT_NAMES.len(),
        "report should list one entry per agent file"
    );
    for entry in &report.created {
        let stem = entry.file_stem().and_then(|s| s.to_str()).unwrap();
        assert!(
            AGENT_NAMES.contains(&stem),
            "report entry {} should match a known agent",
            entry.display()
        );
    }
}

#[test]
fn writes_into_a_fresh_subdirectory_when_target_does_not_exist() {
    let tmp = make_target();
    let target = tmp.path().join("brand-new-project");
    assert!(!target.exists());

    write_agent_scaffolding(&target, false).expect("init");

    assert!(
        target
            .join(".agentic")
            .join("agents")
            .join("architect.md")
            .exists()
    );
}
