//! Shared CLI subprocess helper for ticket sources that shell out to
//! the `gh` or `glab` binaries.

use std::path::PathBuf;
use std::process::Stdio;

use tokio::process::Command;

use super::TicketSourceError;

/// Run a CLI subprocess and return stdout as a `String`.
///
/// Error mapping:
/// - Binary not on PATH → `Transport` ("CLI binary not found …")
/// - Non-zero exit with stderr containing "not found" / "404" → `NotFound`
/// - Other non-zero exit → `Transport`
/// - Successful exit (exit 0) → `Ok(stdout)`
pub(super) async fn run_cli(
    binary: &PathBuf,
    args: &[&str],
    reference: &str,
) -> Result<String, TicketSourceError> {
    let output = Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TicketSourceError::Transport {
                    source: format!(
                        "CLI binary not found: {} — is it installed and on PATH?",
                        binary.display()
                    )
                    .into(),
                }
            } else {
                TicketSourceError::Transport {
                    source: Box::new(e),
                }
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        if stderr.contains("not found") || stderr.contains("404") || stderr.contains("notfound") {
            return Err(TicketSourceError::NotFound {
                reference: reference.to_string(),
            });
        }
        return Err(TicketSourceError::Transport {
            source: format!(
                "{} exited {}: {}",
                binary.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
