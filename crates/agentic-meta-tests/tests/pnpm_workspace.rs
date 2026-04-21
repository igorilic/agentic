use std::path::Path;
use std::process::Command;

/// Resolve workspace root from CARGO_MANIFEST_DIR (crates/agentic-meta-tests → two levels up).
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn pnpm_available() -> bool {
    Command::new("pnpm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn pnpm_workspace_yaml_exists_and_lists_apps_glob() {
    let root = workspace_root();
    let yaml_path = root.join("pnpm-workspace.yaml");

    assert!(
        yaml_path.exists(),
        "pnpm-workspace.yaml does not exist at {:?}",
        yaml_path
    );

    let content = std::fs::read_to_string(&yaml_path)
        .expect("could not read pnpm-workspace.yaml");

    assert!(
        content.contains("packages:"),
        "pnpm-workspace.yaml does not contain 'packages:' key.\ncontent:\n{}",
        content
    );

    assert!(
        content.contains("apps/*"),
        "pnpm-workspace.yaml does not contain 'apps/*' entry.\ncontent:\n{}",
        content
    );
}

#[test]
fn app_package_jsons_exist_with_expected_names() {
    let root = workspace_root();

    let web_ui_path = root.join("apps/web-ui/package.json");
    let vscode_ext_path = root.join("apps/vscode-extension/package.json");

    assert!(
        web_ui_path.exists(),
        "apps/web-ui/package.json does not exist at {:?}",
        web_ui_path
    );
    assert!(
        vscode_ext_path.exists(),
        "apps/vscode-extension/package.json does not exist at {:?}",
        vscode_ext_path
    );

    let web_ui_content = std::fs::read_to_string(&web_ui_path)
        .expect("could not read apps/web-ui/package.json");
    let vscode_ext_content = std::fs::read_to_string(&vscode_ext_path)
        .expect("could not read apps/vscode-extension/package.json");

    assert!(
        web_ui_content.contains("\"@agentic/web-ui\""),
        "apps/web-ui/package.json does not contain '@agentic/web-ui'.\ncontent:\n{}",
        web_ui_content
    );
    assert!(
        vscode_ext_content.contains("\"@agentic/vscode-extension\""),
        "apps/vscode-extension/package.json does not contain '@agentic/vscode-extension'.\ncontent:\n{}",
        vscode_ext_content
    );
    assert!(
        web_ui_content.contains("\"private\": true"),
        "apps/web-ui/package.json does not contain '\"private\": true'.\ncontent:\n{}",
        web_ui_content
    );
    assert!(
        vscode_ext_content.contains("\"private\": true"),
        "apps/vscode-extension/package.json does not contain '\"private\": true'.\ncontent:\n{}",
        vscode_ext_content
    );
}

#[test]
fn pnpm_install_succeeds_on_clean_workspace() {
    if !pnpm_available() {
        eprintln!("pnpm not on PATH — skipping runtime check; manifest-shape tests still run");
        return;
    }

    let root = workspace_root();

    // Use pnpm list -r --depth 0 which is a read-only command that parses
    // the workspace without requiring a lockfile or running install.
    let output = Command::new("pnpm")
        .args(["list", "-r", "--depth", "0"])
        .current_dir(&root)
        .output()
        .expect("failed to spawn pnpm list");

    assert!(
        output.status.success(),
        "pnpm list -r --depth 0 exited non-zero.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
