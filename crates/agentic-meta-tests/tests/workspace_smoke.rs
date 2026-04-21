use std::process::Command;

#[test]
fn cargo_metadata_loads_workspace() {
    // Resolve workspace root: CARGO_MANIFEST_DIR is crates/agentic-meta-tests,
    // so two levels up lands at the workspace root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve workspace root from CARGO_MANIFEST_DIR");

    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(workspace_root)
        .output()
        .expect("failed to spawn cargo metadata");

    assert!(
        output.status.success(),
        "cargo metadata exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("workspace_members"),
        "cargo metadata output did not contain 'workspace_members'.\nstdout: {}",
        stdout
    );
}
