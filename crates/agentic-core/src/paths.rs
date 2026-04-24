use std::path::{Path, PathBuf};

use crate::{CoreError, Result};

#[derive(Debug)]
pub struct Paths {
    root: PathBuf,
    data_root: PathBuf,
}

impl Paths {
    /// Resolve from OS conventions via `directories::ProjectDirs`.
    /// Returns `CoreError::Config` if `$HOME` is unset (no ProjectDirs available).
    pub fn from_os() -> Result<Self> {
        let pd = directories::ProjectDirs::from("", "", "agentic").ok_or_else(|| {
            CoreError::Config("could not resolve OS project directories ($HOME unset?)".into())
        })?;
        Ok(Self {
            root: pd.config_dir().to_path_buf(),
            data_root: pd.data_dir().to_path_buf(),
        })
    }

    /// Deterministic test-only constructor.
    /// `config_dir()` resolves to `base/config`, `data_dir()` to `base/data`.
    pub fn for_tests(base: &Path) -> Self {
        Self {
            root: base.join("config"),
            data_root: base.join("data"),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.root
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_root
    }

    pub fn log_dir(&self) -> PathBuf {
        self.data_root.join("logs")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("settings.toml")
    }

    pub fn db_file(&self) -> PathBuf {
        self.data_root.join("state.db")
    }

    /// Create config_dir, data_dir, and log_dir idempotently.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(&self.data_root)?;
        std::fs::create_dir_all(self.log_dir())?;
        Ok(())
    }

    /// Path to the directory for a specific run: `<data_dir>/runs/<run_id>`.
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.data_root.join("runs").join(run_id)
    }

    /// Path to the directory for a specific step:
    /// `<data_dir>/runs/<run_id>/step-<NN>-<agent>`.
    /// `seq` is zero-padded to 2 digits.
    pub fn step_dir(&self, run_id: &str, seq: usize, agent: &str) -> PathBuf {
        self.run_dir(run_id).join(format!("step-{seq:02}-{agent}"))
    }

    /// Create the step directory (including all parents) and return its path.
    pub fn ensure_step_dir(
        &self,
        run_id: &str,
        seq: usize,
        agent: &str,
    ) -> std::io::Result<PathBuf> {
        let p = self.step_dir(run_id, seq, agent);
        std::fs::create_dir_all(&p)?;
        Ok(p)
    }
}
