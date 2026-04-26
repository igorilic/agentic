use std::fs;
use std::path::PathBuf;

fn conf_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("agentic-tauri")
        .join("tauri.conf.json")
}

#[test]
fn tauri_conf_identifier_is_io_agentic_app() {
    let path = conf_path();
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        v.get("identifier").and_then(|x| x.as_str()),
        Some("io.agentic.app"),
        "tauri.conf.json identifier must be io.agentic.app"
    );
}

#[test]
fn tauri_conf_product_name_is_agentic() {
    let path = conf_path();
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        v.get("productName").and_then(|x| x.as_str()),
        Some("Agentic"),
        "tauri.conf.json productName must be Agentic"
    );
}

#[test]
fn agentic_tauri_is_workspace_member() {
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("cargo metadata failed");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let packages = json.get("packages").and_then(|p| p.as_array()).unwrap();
    let names: Vec<&str> = packages
        .iter()
        .filter_map(|p| p.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        names.contains(&"agentic-tauri"),
        "expected agentic-tauri in workspace; got: {names:?}"
    );
}
