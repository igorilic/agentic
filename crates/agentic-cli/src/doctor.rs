use std::io::{self, Write};
use std::path::PathBuf;

/// Abstraction over PATH look-up so callers can inject stubs in tests.
pub trait WhichProbe: Send + Sync {
    fn find(&self, bin: &str) -> Option<PathBuf>;
}

/// Production implementation that delegates to the `which` crate.
pub struct SystemWhichProbe;

impl WhichProbe for SystemWhichProbe {
    fn find(&self, bin: &str) -> Option<PathBuf> {
        which::which(bin).ok()
    }
}

/// The four tools whose presence is checked by `agentic-cli doctor`.
const CHECKED_BINS: &[&str] = &["claude", "copilot", "gh", "glab"];

/// Write a human-readable table of tool presence to `out`.
pub fn run_doctor(probe: &dyn WhichProbe, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{:<10}  status", "tool")?;
    writeln!(out, "{}", "-".repeat(40))?;
    for bin in CHECKED_BINS {
        match probe.find(bin) {
            Some(path) => writeln!(out, "{:<10}  found at {}", bin, path.display())?,
            None => writeln!(out, "{:<10}  not found", bin)?,
        }
    }
    Ok(())
}
