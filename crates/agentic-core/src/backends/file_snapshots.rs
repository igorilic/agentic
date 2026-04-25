//! File snapshot diffing for tool-use edits.
//!
//! [`FileSnapshotter`] records the before-state of a set of paths, then after
//! mutations are applied it:
//!
//! 1. Computes blake3 hashes for before and after states.
//! 2. Emits [`Event::FileChange`] events on the provided [`EventSink`] for
//!    every path whose state changed (including newly created and deleted files).
//! 3. Writes a unified-diff patch to `file_changes.diff` using the [`similar`]
//!    crate.
//!
//! # Skipping large / non-UTF-8 files
//!
//! Files that exceed [`MAX_DIFF_FILE_SIZE`] bytes, or whose contents are not
//! valid UTF-8, are still hashed and a [`Event::FileChange`] is emitted, but
//! they are excluded from the unified-diff output. The [`FinalizeReport`]
//! records them under [`FinalizeReport::skipped_paths`].
//!
//! Files whose before and after states are identical are not emitted and are
//! recorded under [`FinalizeReport::unchanged_paths`].
//!
//! # `file_changes.diff` format
//!
//! The diff file is a concatenation of unified-diff sections produced by
//! [`similar::unified_diff`]. Each section begins with the standard
//! `--- <path>` / `+++ <path>` / `@@` headers. Skipped files produce **no
//! entry** in the diff file.

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use similar::TextDiff;

use crate::backends::EventSink;
use crate::events::{Event, EventEnvelope};

/// Maximum file size (in bytes) eligible for diff generation.
/// Files above this threshold are hashed but excluded from the unified diff.
pub const MAX_DIFF_FILE_SIZE: u64 = 1_048_576; // 1 MiB

/// Why a file was excluded from the unified-diff output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// File exceeds [`MAX_DIFF_FILE_SIZE`].
    TooLarge(u64),
    /// File contents are not valid UTF-8.
    NonUtf8,
}

/// Internal state for one tracked path.
#[derive(Debug)]
pub enum FileState {
    /// File was present; bytes are stored only when small enough and UTF-8.
    Present { hash: String, text: Option<String> },
    /// File did not exist at capture time.
    Absent,
    /// File was too large or non-UTF-8 — hash stored, but no text for diff.
    Skipped { hash: String, reason: SkipReason },
}

/// Result of [`FileSnapshotter::finalize`].
#[derive(Debug, Default)]
pub struct FinalizeReport {
    /// Paths whose before and after states differed (events emitted).
    pub changed_paths: Vec<PathBuf>,
    /// Paths that were identical before and after (no event emitted).
    pub unchanged_paths: Vec<PathBuf>,
    /// Paths skipped from diff generation, with the reason.
    pub skipped_paths: Vec<(PathBuf, SkipReason)>,
}

/// Records before-states of filesystem paths, then after mutations are applied,
/// computes diffs and emits [`Event::FileChange`] events.
pub struct FileSnapshotter {
    before: HashMap<PathBuf, FileState>,
}

impl Default for FileSnapshotter {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSnapshotter {
    /// Create a new snapshotter. Paths are tracked absolutely and are not
    /// restricted to any workspace root.
    pub fn new() -> Self {
        Self {
            before: HashMap::new(),
        }
    }

    /// Capture the current on-disk state of `path` as the "before" snapshot.
    /// Call this **before** mutations are applied to the path.
    ///
    /// **Idempotent**: if `path` has already been captured in this snapshotter
    /// session, this call is a no-op. The first-captured state is preserved so
    /// that repeated `ToolUseStart` events for the same file do not overwrite
    /// the genuine pre-edit snapshot with an intermediate state.
    pub fn capture(&mut self, path: &Path) -> std::io::Result<()> {
        if self.before.contains_key(path) {
            return Ok(());
        }
        let state = read_file_state(path)?;
        self.before.insert(path.to_path_buf(), state);
        Ok(())
    }

    /// Compute after-states, emit [`Event::FileChange`] events, and write the
    /// unified-diff file at `diff_path`.
    ///
    /// Returns a [`FinalizeReport`] summarising changed, unchanged, and skipped
    /// paths.
    pub fn finalize(
        self,
        diff_path: &Path,
        sink: &EventSink,
        run_id: &str,
        step_id: Option<&str>,
    ) -> std::io::Result<FinalizeReport> {
        let mut report = FinalizeReport::default();
        let mut diff_sections: Vec<String> = Vec::new();

        let mut paths: Vec<PathBuf> = self.before.keys().cloned().collect();
        paths.sort();
        let mut before_map = self.before;
        for path in paths {
            let before_state = before_map.remove(&path).expect("path was in keys");
            let after_state = read_file_state(&path)?;

            let before_hash = state_hash(&before_state);
            let after_hash = state_hash(&after_state);

            // Unchanged — no event, no diff
            if before_hash == after_hash {
                report.unchanged_paths.push(path);
                continue;
            }

            // Emit FileChange event
            let event = Event::FileChange {
                path: path.clone(),
                before_hash: before_hash.clone(),
                after_hash: after_hash.clone(),
            };
            let envelope =
                EventEnvelope::now(run_id.to_string(), step_id.map(str::to_string), event);
            // Best-effort send; ignore lagged-receiver errors
            let _ = sink.send(envelope);

            report.changed_paths.push(path.clone());

            // Determine if we should generate a diff section
            let skip_reason = skip_reason_for(&before_state, &after_state);
            if let Some(reason) = skip_reason {
                report.skipped_paths.push((path, reason));
                continue;
            }

            // Generate unified diff
            let before_text = state_text(&before_state);
            let after_text = state_text(&after_state);
            let path_str = path.to_string_lossy();
            let section = build_unified_diff(&path_str, before_text, after_text);
            diff_sections.push(section);
        }

        // Write diff file (only if there are sections)
        if !diff_sections.is_empty() {
            let mut file = fs::File::create(diff_path)?;
            for section in &diff_sections {
                file.write_all(section.as_bytes())?;
            }
        }

        Ok(report)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn read_file_state(path: &Path) -> std::io::Result<FileState> {
    match fs::metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            let bytes = fs::read(path)?;

            if size > MAX_DIFF_FILE_SIZE {
                let hash = blake3_hex(&bytes);
                return Ok(FileState::Skipped {
                    hash,
                    reason: SkipReason::TooLarge(size),
                });
            }

            match std::str::from_utf8(&bytes) {
                Ok(text) => {
                    let hash = blake3_hex(&bytes);
                    Ok(FileState::Present {
                        hash,
                        text: Some(text.to_string()),
                    })
                }
                Err(_) => {
                    let hash = blake3_hex(&bytes);
                    Ok(FileState::Skipped {
                        hash,
                        reason: SkipReason::NonUtf8,
                    })
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileState::Absent),
        Err(e) => Err(e),
    }
}

fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Return the hash string for a state, or `""` for `Absent`.
fn state_hash(state: &FileState) -> String {
    match state {
        FileState::Present { hash, .. } => hash.clone(),
        FileState::Skipped { hash, .. } => hash.clone(),
        FileState::Absent => String::new(),
    }
}

/// Return the text content for diff generation (empty string for absent files).
fn state_text(state: &FileState) -> &str {
    match state {
        FileState::Present { text: Some(t), .. } => t.as_str(),
        FileState::Present { text: None, .. } => "",
        FileState::Absent => "",
        FileState::Skipped { .. } => "",
    }
}

/// If either before or after state is Skipped, return the SkipReason.
/// Absent + Present transitions are diffable and return None.
fn skip_reason_for(before: &FileState, after: &FileState) -> Option<SkipReason> {
    match (before, after) {
        (FileState::Skipped { reason, .. }, _) => Some(reason.clone()),
        (_, FileState::Skipped { reason, .. }) => Some(reason.clone()),
        _ => None,
    }
}

fn build_unified_diff(path: &str, before: &str, after: &str) -> String {
    let diff = TextDiff::from_lines(before, after);
    let before_label = format!("a/{path}");
    let after_label = format!("b/{path}");
    diff.unified_diff()
        .context_radius(3)
        .header(&before_label, &after_label)
        .to_string()
}
