use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use crate::auth::SecretStore;
use crate::auth::SecretStoreError;

#[derive(Debug, thiserror::Error)]
pub enum GhDelegateError {
    /// `gh` binary not found on PATH or not executable.
    #[error("gh CLI binary not available: {0}")]
    GhNotAvailable(String),
    /// `gh auth status` reports no logged-in account.
    #[error("no existing gh session")]
    NoExistingSession,
    /// `gh auth token` printed nothing or empty.
    #[error("gh auth token returned empty output")]
    EmptyToken,
    /// SecretStore failed to persist.
    #[error("secret store: {0}")]
    SecretStore(#[from] SecretStoreError),
    /// Subprocess spawn or wait failed.
    #[error("subprocess: {0}")]
    Subprocess(String),
}

pub struct GhDelegate {
    /// Path to the `gh` binary. Defaults to "gh" (resolved via PATH).
    /// Override via `with_binary` for tests using a fake-gh shell script.
    binary: PathBuf,
}

impl GhDelegate {
    pub fn new() -> Self {
        Self {
            binary: PathBuf::from("gh"),
        }
    }

    pub fn with_binary(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    /// Check that `gh` exists and reports a valid session via `gh auth status`.
    pub async fn check_session(&self) -> Result<(), GhDelegateError> {
        let output = Command::new(&self.binary)
            .args(["auth", "status"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GhDelegateError::GhNotAvailable(format!(
                        "binary not found: {}",
                        self.binary.display()
                    ))
                } else {
                    GhDelegateError::Subprocess(e.to_string())
                }
            })?;

        if !output.status.success() {
            return Err(GhDelegateError::NoExistingSession);
        }
        Ok(())
    }

    /// Capture the token via `gh auth token` and store it in `secrets` under `key`.
    /// Calls `check_session` first to fail fast on missing gh / no session.
    pub async fn import_token(
        &self,
        secrets: &dyn SecretStore,
        key: &str,
    ) -> Result<(), GhDelegateError> {
        self.check_session().await?;

        let output = Command::new(&self.binary)
            .args(["auth", "token"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| GhDelegateError::Subprocess(e.to_string()))?;

        if !output.status.success() {
            return Err(GhDelegateError::Subprocess(format!(
                "gh auth token exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            return Err(GhDelegateError::EmptyToken);
        }

        secrets.set(key, &token)?;
        Ok(())
    }
}

impl Default for GhDelegate {
    fn default() -> Self {
        Self::new()
    }
}
