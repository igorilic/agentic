use std::fs;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/copilot")
}

#[test]
fn at_least_three_copilot_fixtures_exist() {
    let dir = fixtures_dir();
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("fixtures dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "jsonl"))
        .collect();
    assert!(
        entries.len() >= 3,
        "expected at least 3 jsonl fixtures under {}, found {}",
        dir.display(),
        entries.len()
    );
}

#[test]
fn every_copilot_fixture_is_non_empty() {
    let dir = fixtures_dir();
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().map_or(false, |ext| ext == "jsonl") {
            let size = entry.metadata().unwrap().len();
            assert!(size > 0, "fixture {} is empty", entry.path().display());
        }
    }
}
