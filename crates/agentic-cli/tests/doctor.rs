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
    fn find(&self, bin: &str) -> Result<Option<PathBuf>, which::Error> {
        Ok(self.results.get(bin).and_then(|v| v.clone()))
    }
}

/// A probe that returns `Err` for every binary it is asked about.
struct ErrProbe;

impl WhichProbe for ErrProbe {
    fn find(&self, _bin: &str) -> Result<Option<PathBuf>, which::Error> {
        Err(which::Error::CannotFindBinaryPath)
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

#[test]
fn mixed_found_and_not_found_lines_render_distinctly() {
    // claude + gh found at distinct paths, copilot + glab not found (absent from map → None)
    let probe = StubProbe::new(HashMap::from([
        ("claude", Some(PathBuf::from("/usr/local/bin/claude"))),
        ("gh", Some(PathBuf::from("/opt/homebrew/bin/gh"))),
    ]));
    let mut out = Vec::new();
    run_doctor(&probe, &mut out).unwrap();
    let s = String::from_utf8(out).unwrap();

    // claude: found with correct path
    let claude_line = s.lines().find(|l| l.contains("claude")).unwrap();
    assert!(
        claude_line.to_lowercase().contains("found"),
        "claude should be found, got: {claude_line}"
    );
    assert!(
        claude_line.contains("/usr/local/bin/claude"),
        "claude line should contain path, got: {claude_line}"
    );

    // gh: found with correct path
    let gh_line = s.lines().find(|l| l.contains("gh")).unwrap();
    assert!(
        gh_line.contains("/opt/homebrew/bin/gh"),
        "gh line should contain path, got: {gh_line}"
    );

    // copilot: not found
    let copilot_line = s.lines().find(|l| l.contains("copilot")).unwrap();
    assert!(
        copilot_line.contains("not found"),
        "copilot should be not found, got: {copilot_line}"
    );

    // glab: not found
    let glab_line = s.lines().find(|l| l.contains("glab")).unwrap();
    assert!(
        glab_line.contains("not found"),
        "glab should be not found, got: {glab_line}"
    );
}

#[test]
fn probe_error_renders_error_line() {
    let probe = ErrProbe;
    let mut out = Vec::new();
    run_doctor(&probe, &mut out).expect("run_doctor should succeed even when probe errors");
    let text = String::from_utf8(out).unwrap();
    let claude_line = text
        .lines()
        .find(|l| l.contains("claude"))
        .expect("output should contain a line mentioning 'claude'");
    assert!(
        claude_line.contains("error"),
        "expected 'error' in line when probe fails: {claude_line}"
    );
}
