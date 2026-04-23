use std::collections::HashMap;
use std::path::PathBuf;

use agentic_cli::doctor::{WhichProbe, run_doctor};

struct StubProbe {
    results: HashMap<&'static str, Option<PathBuf>>,
}

impl StubProbe {
    fn new(results: HashMap<&'static str, Option<PathBuf>>) -> Self {
        Self { results }
    }
}

impl WhichProbe for StubProbe {
    fn find(&self, bin: &str) -> Option<PathBuf> {
        self.results.get(bin).and_then(|v| v.clone())
    }
}

fn all_not_found() -> StubProbe {
    let mut m = HashMap::new();
    m.insert("claude", None);
    m.insert("copilot", None);
    m.insert("gh", None);
    m.insert("glab", None);
    StubProbe::new(m)
}

fn all_found_at(base: &'static str) -> StubProbe {
    let mut m = HashMap::new();
    m.insert("claude", Some(PathBuf::from(format!("{base}/claude"))));
    m.insert("copilot", Some(PathBuf::from(format!("{base}/copilot"))));
    m.insert("gh", Some(PathBuf::from(format!("{base}/gh"))));
    m.insert("glab", Some(PathBuf::from(format!("{base}/glab"))));
    StubProbe::new(m)
}

#[test]
fn not_found_line_contains_claude_and_not_found() {
    let probe = all_not_found();
    let mut out = Vec::new();
    run_doctor(&probe, &mut out).expect("run_doctor should succeed");
    let text = String::from_utf8(out).unwrap();
    let claude_line = text
        .lines()
        .find(|l| l.contains("claude"))
        .expect("output should contain a line mentioning 'claude'");
    assert!(
        claude_line.to_lowercase().contains("not found"),
        "expected 'not found' in line: {claude_line}"
    );
}

#[test]
fn found_line_contains_claude_found_and_path() {
    let probe = all_found_at("/usr/local/bin");
    let mut out = Vec::new();
    run_doctor(&probe, &mut out).expect("run_doctor should succeed");
    let text = String::from_utf8(out).unwrap();
    let claude_line = text
        .lines()
        .find(|l| l.contains("claude"))
        .expect("output should contain a line mentioning 'claude'");
    assert!(
        claude_line.to_lowercase().contains("found"),
        "expected 'found' in line: {claude_line}"
    );
    assert!(
        claude_line.contains("/usr/local/bin/claude"),
        "expected path '/usr/local/bin/claude' in line: {claude_line}"
    );
}

#[test]
fn output_contains_all_four_bins() {
    let probe = all_not_found();
    let mut out = Vec::new();
    run_doctor(&probe, &mut out).expect("run_doctor should succeed");
    let text = String::from_utf8(out).unwrap();
    for bin in &["claude", "copilot", "gh", "glab"] {
        assert!(
            text.contains(bin),
            "expected output to contain '{bin}'; got:\n{text}"
        );
    }
}
