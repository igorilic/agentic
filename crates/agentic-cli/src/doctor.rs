use std::io::{self, Write};
use std::path::PathBuf;

/// Abstraction over PATH look-up so callers can inject stubs in tests.
pub trait WhichProbe: Send + Sync {
    fn find(&self, bin: &str) -> Result<Option<PathBuf>, which::Error>;
}

/// Production implementation that delegates to the `which` crate.
pub struct SystemWhichProbe;

impl WhichProbe for SystemWhichProbe {
    fn find(&self, bin: &str) -> Result<Option<PathBuf>, which::Error> {
        match which::which(bin) {
            Ok(path) => Ok(Some(path)),
            Err(which::Error::CannotFindBinaryPath) => Ok(None),
            Err(e) => Err(e),
        }
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
            Ok(Some(path)) => writeln!(out, "{:<10}  found at {}", bin, path.display())?,
            Ok(None) => writeln!(out, "{:<10}  not found", bin)?,
            Err(e) => writeln!(out, "{:<10}  error: {}", bin, e)?,
        }
    }
    Ok(())
}
