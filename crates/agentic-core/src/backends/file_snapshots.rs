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
use std::io::{BufReader, Read as _, Write as _};
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
    /// When `Some`, before-state blobs are persisted to `<snapshot_dir>/<hash>`.
    snapshot_dir: Option<PathBuf>,
}

impl Default for FileSnapshotter {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSnapshotter {
    /// Create a new snapshotter. Paths are tracked absolutely and are not
    /// restricted to any workspace root. Before-states are kept in memory
    /// only — nothing is persisted to disk.
    pub fn new() -> Self {
        Self {
            before: HashMap::new(),
            snapshot_dir: None,
        }
    }

    /// Create a snapshotter that persists before-state blobs to
    /// `snapshot_dir/<hash>`. The directory is created on first write.
    /// Callers that need `vscode.diff` support (Phase 3) should use this
    /// constructor so the extension can fetch the before-content later.
    pub fn with_store(snapshot_dir: PathBuf) -> Self {
        Self {
            before: HashMap::new(),
            snapshot_dir: Some(snapshot_dir),
        }
    }

    /// Capture the current on-disk state of `path` as the "before" snapshot.
    /// Call this **before** mutations are applied to the path.
    ///
    /// **Idempotent**: if `path` has already been captured in this snapshotter
    /// session, this call is a no-op. The first-captured state is preserved so
    /// that repeated `ToolUseStart` events for the same file do not overwrite
    /// the genuine pre-edit snapshot with an intermediate state.
    ///
    /// When a `snapshot_dir` is configured via [`Self::with_store`], the raw
    /// bytes are persisted to `<snapshot_dir>/<hash>` (idempotent — skipped if
    /// the file already exists). [`FileState::Skipped`] entries (too-large /
    /// binary) are not persisted.
    pub fn capture(&mut self, path: &Path) -> std::io::Result<()> {
        if self.before.contains_key(path) {
            return Ok(());
        }
        let state = read_file_state(path)?;

        // Persist blob when a store dir is configured and state has bytes.
        if let Some(ref snap_dir) = self.snapshot_dir {
            persist_snapshot(snap_dir, &state)?;
        }

        self.before.insert(path.to_path_buf(), state);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Public utility
    // ------------------------------------------------------------------

    /// Read the raw bytes of a previously-persisted snapshot blob.
    ///
    /// `snapshot_dir` must be the same directory passed to
    /// [`FileSnapshotter::with_store`]. Returns `Err(NotFound)` when no
    /// blob exists for the given `hash`.
    pub fn snapshot_dir(&self) -> Option<&Path> {
        self.snapshot_dir.as_deref()
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

            // For files exceeding the diff size limit, stream-hash without
            // loading the full contents into RAM. This avoids a 500 MB
            // allocation before deciding to skip.
            if size > MAX_DIFF_FILE_SIZE {
                let hash = blake3_stream_hex(path)?;
                return Ok(FileState::Skipped {
                    hash,
                    reason: SkipReason::TooLarge(size),
                });
            }

            let bytes = fs::read(path)?;
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

/// Hash a file by streaming it through a [`blake3::Hasher`] in chunks.
/// This avoids loading the entire file into memory, which is important for
/// files larger than [`MAX_DIFF_FILE_SIZE`].
fn blake3_stream_hex(path: &Path) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536]; // 64 KiB read buffer
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
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

/// Persist a before-state blob to `<snapshot_dir>/<hash>`.
///
/// Skips `FileState::Absent` (nothing to write) and
/// `FileState::Skipped` (no text bytes; binary/large files are not diffable
/// anyway so the extension has no use for the raw bytes).
///
/// Idempotent: if `<snapshot_dir>/<hash>` already exists, this function
/// returns `Ok(())` without re-writing.
fn persist_snapshot(snapshot_dir: &Path, state: &FileState) -> std::io::Result<()> {
    let (hash, bytes): (&str, &[u8]) = match state {
        FileState::Present {
            hash,
            text: Some(text),
        } => (hash.as_str(), text.as_bytes()),
        // Absent or Skipped — nothing to persist.
        _ => return Ok(()),
    };

    validate_hash(hash)?;
    fs::create_dir_all(snapshot_dir)?;
    let dest = snapshot_dir.join(hash);
    if dest.exists() {
        return Ok(());
    }
    fs::write(&dest, bytes)
}

/// Read the raw bytes of a previously-persisted snapshot blob.
///
/// `snapshot_dir` is the directory passed to
/// [`FileSnapshotter::with_store`]. Returns an `Err` with
/// `ErrorKind::NotFound` when no blob exists for the given `hash`,
/// or `ErrorKind::InvalidInput` if `hash` contains non-hex characters
/// (defence-in-depth: callers come across the napi boundary so an
/// attacker-controlled `hash = "../../etc/passwd"` must not traverse).
pub fn read_snapshot(snapshot_dir: &Path, hash: &str) -> std::io::Result<Vec<u8>> {
    validate_hash(hash)?;
    let path = snapshot_dir.join(hash);
    fs::read(&path)
}

/// Reject any `hash` that contains characters outside `[0-9a-f]` or
/// that exceeds the longest hash length we'd ever expect (256 hex
/// chars = 1024 bits, far above blake3's 256-bit output). The set is
/// deliberately tighter than "no path separators" — the storage
/// contract is "blake3 hex" and anything else is a bug or attack.
fn validate_hash(hash: &str) -> std::io::Result<()> {
    if hash.is_empty() || hash.len() > 256 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "snapshot hash has invalid length",
        ));
    }
    if !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "snapshot hash must be lowercase hex",
        ));
    }
    Ok(())
}
