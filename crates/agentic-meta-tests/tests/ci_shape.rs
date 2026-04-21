use std::path::PathBuf;

fn workflow_yaml_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // agentic-meta-tests/Cargo.toml -> repo root is two parents up.
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(".github/workflows/test.yml")
}

fn load_workflow() -> serde_yaml::Value {
    let p = workflow_yaml_path();
    let content = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", p.display()));
    serde_yaml::from_str(&content).expect("workflow is not valid YAML")
}

#[test]
fn workflow_file_exists() {
    let p = workflow_yaml_path();
    assert!(p.exists(), "expected {} to exist", p.display());
}

#[test]
fn workflow_defines_fmt_clippy_test_jobs() {
    let wf = load_workflow();
    let jobs = wf.get("jobs").expect("workflow has no 'jobs' key");
    for name in ["fmt", "clippy", "test"] {
        assert!(
            jobs.get(name).is_some(),
            "workflow.jobs is missing '{name}' (have: {:?})",
            jobs.as_mapping().map(|m| m.keys().collect::<Vec<_>>())
        );
    }
}

#[test]
fn test_job_matrix_includes_macos_and_linux() {
    let wf = load_workflow();
    let matrix = wf
        .get("jobs")
        .and_then(|j| j.get("test"))
        .and_then(|t| t.get("strategy"))
        .and_then(|s| s.get("matrix"))
        .and_then(|m| m.get("os"))
        .expect("test job has no strategy.matrix.os");
    let os_list: Vec<String> = matrix
        .as_sequence()
        .expect("matrix.os is not a sequence")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    for needle in ["macos-latest", "ubuntu-latest"] {
        assert!(
            os_list.iter().any(|s| s == needle),
            "matrix.os missing {needle}; have {os_list:?}"
        );
    }
}

#[test]
fn test_job_installs_pnpm_and_node() {
    // Closes Step 0.5 review Finding 2: CI must have pnpm on PATH so the
    // pnpm_install_succeeds_on_clean_workspace test exercises the real path.
    let wf = load_workflow();
    let steps = wf
        .get("jobs")
        .and_then(|j| j.get("test"))
        .and_then(|t| t.get("steps"))
        .and_then(|s| s.as_sequence())
        .expect("test job has no steps sequence");
    let uses: Vec<String> = steps
        .iter()
        .filter_map(|s| s.get("uses").and_then(|u| u.as_str()).map(String::from))
        .collect();
    assert!(
        uses.iter().any(|u| u.starts_with("pnpm/action-setup")),
        "test job must use pnpm/action-setup; uses: {uses:?}"
    );
    assert!(
        uses.iter().any(|u| u.starts_with("actions/setup-node")),
        "test job must use actions/setup-node; uses: {uses:?}"
    );
}

#[test]
fn workflow_has_workflow_dispatch_trigger() {
    let wf = load_workflow();
    let on = wf.get("on").expect("workflow has no 'on' block");
    assert!(
        on.get("workflow_dispatch").is_some(),
        "workflow missing workflow_dispatch trigger (YAML 'on' block: {on:?})"
    );
}
