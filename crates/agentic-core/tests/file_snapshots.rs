use std::fs;
use std::io::Write as _;
use std::path::Path;

use agentic_core::backends::file_snapshots::{FileSnapshotter, SkipReason};
use agentic_core::{Event, EventEnvelope};
use tempfile::TempDir;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_sink() -> (
    broadcast::Sender<EventEnvelope>,
    broadcast::Receiver<EventEnvelope>,
) {
    broadcast::channel(64)
}

fn collect_file_change_events(
    rx: &mut broadcast::Receiver<EventEnvelope>,
) -> Vec<(std::path::PathBuf, String, String)> {
    let mut events = Vec::new();
    while let Ok(env) = rx.try_recv() {
        if let Event::FileChange {
            path,
            before_hash,
            after_hash,
        } = env.event
        {
            events.push((path, before_hash, after_hash));
        }
    }
    events
}

// ---------------------------------------------------------------------------
// Test 1: modified file emits FileChange with different hashes
// ---------------------------------------------------------------------------

#[test]
fn file_modified_emits_file_change_with_different_hashes() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("hello.txt");

    fs::write(&file_path, b"original content").unwrap();

    let mut snapshotter = FileSnapshotter::new(dir.path().to_path_buf());
    snapshotter.capture(&file_path).unwrap();

    // Mutate after capture
    fs::write(&file_path, b"modified content").unwrap();

    let diff_path = dir.path().join("file_changes.diff");
    let (sink, mut rx) = make_sink();

    let report = snapshotter
        .finalize(&diff_path, &sink, "run-1", Some("step-1"))
        .unwrap();

    let events = collect_file_change_events(&mut rx);
    assert_eq!(events.len(), 1, "expected one FileChange event");

    let (path, before_hash, after_hash) = &events[0];
    assert_eq!(path, &file_path);
    assert_ne!(before_hash, after_hash, "hashes must differ after mutation");
    assert_eq!(before_hash.len(), 64, "blake3 hex is 64 chars");
    assert_eq!(after_hash.len(), 64, "blake3 hex is 64 chars");

    assert!(
        report.changed_paths.contains(&file_path),
        "changed_paths should include the modified file"
    );

    // diff file should exist and contain a unified-diff section
    let diff_content = fs::read_to_string(&diff_path).unwrap();
    assert!(
        diff_content.contains("---") && diff_content.contains("+++"),
        "diff file should contain unified-diff headers"
    );
    assert!(
        diff_content.contains("original") || diff_content.contains("modified"),
        "diff should reference the changed content"
    );
}

// ---------------------------------------------------------------------------
// Test 2: created file emits FileChange with empty/absent before_hash
// ---------------------------------------------------------------------------

#[test]
fn file_created_emits_file_change_with_absent_before_hash() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("new_file.txt");

    // Capture BEFORE the file exists
    let mut snapshotter = FileSnapshotter::new(dir.path().to_path_buf());
    snapshotter.capture(&file_path).unwrap();

    // Now create the file
    fs::write(&file_path, b"brand new content").unwrap();

    let diff_path = dir.path().join("file_changes.diff");
    let (sink, mut rx) = make_sink();

    let report = snapshotter
        .finalize(&diff_path, &sink, "run-2", Some("step-2"))
        .unwrap();

    let events = collect_file_change_events(&mut rx);
    assert_eq!(events.len(), 1, "expected one FileChange event");

    let (path, before_hash, after_hash) = &events[0];
    assert_eq!(path, &file_path);
    // before_hash should be the sentinel for "absent" (empty string)
    assert_eq!(
        before_hash, "",
        "before_hash should be empty for newly created file"
    );
    assert_eq!(after_hash.len(), 64, "after_hash should be a blake3 hex");

    assert!(report.changed_paths.contains(&file_path));
}

// ---------------------------------------------------------------------------
// Test 3: deleted file emits FileChange with empty/absent after_hash
// ---------------------------------------------------------------------------

#[test]
fn file_deleted_emits_file_change_with_absent_after_hash() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("to_delete.txt");

    fs::write(&file_path, b"will be deleted").unwrap();

    let mut snapshotter = FileSnapshotter::new(dir.path().to_path_buf());
    snapshotter.capture(&file_path).unwrap();

    // Delete after capture
    fs::remove_file(&file_path).unwrap();

    let diff_path = dir.path().join("file_changes.diff");
    let (sink, mut rx) = make_sink();

    let report = snapshotter
        .finalize(&diff_path, &sink, "run-3", Some("step-3"))
        .unwrap();

    let events = collect_file_change_events(&mut rx);
    assert_eq!(events.len(), 1, "expected one FileChange event");

    let (path, before_hash, after_hash) = &events[0];
    assert_eq!(path, &file_path);
    assert_eq!(before_hash.len(), 64, "before_hash should be a blake3 hex");
    assert_eq!(
        after_hash, "",
        "after_hash should be empty for deleted file"
    );

    assert!(report.changed_paths.contains(&file_path));
}

// ---------------------------------------------------------------------------
// Test 4: binary/large file emits FileChange with hashes but no diff section
// ---------------------------------------------------------------------------

#[test]
fn binary_file_over_1mb_skipped_from_diff_but_hashed() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("big_binary.bin");

    // Write 1.5 MB of non-UTF-8 bytes (0xFF repeated)
    let big_content: Vec<u8> = vec![0xFF_u8; 1_572_864]; // 1.5 MB
    fs::write(&file_path, &big_content).unwrap();

    let mut snapshotter = FileSnapshotter::new(dir.path().to_path_buf());
    snapshotter.capture(&file_path).unwrap();

    // Mutate: write slightly different content
    let mut modified = big_content.clone();
    modified[0] = 0xFE;
    fs::write(&file_path, &modified).unwrap();

    let diff_path = dir.path().join("file_changes.diff");
    let (sink, mut rx) = make_sink();

    let report = snapshotter
        .finalize(&diff_path, &sink, "run-4", Some("step-4"))
        .unwrap();

    // FileChange event MUST be emitted (with hashes)
    let events = collect_file_change_events(&mut rx);
    assert_eq!(events.len(), 1, "expected one FileChange event for binary file");

    let (_path, before_hash, after_hash) = &events[0];
    assert_eq!(before_hash.len(), 64, "before_hash should be a blake3 hex");
    assert_eq!(after_hash.len(), 64, "after_hash should be a blake3 hex");
    assert_ne!(before_hash, after_hash);

    // The diff file should NOT contain a diff section for this binary/large file
    // It may not exist at all, or be empty, or contain only a skip marker — but
    // NOT contain unified-diff +/- lines for the binary content.
    let diff_exists = diff_path.exists();
    if diff_exists {
        let diff_content = fs::read_to_string(&diff_path).unwrap_or_default();
        // Should not contain actual binary diff lines
        // Allow a skip comment line, but not "@@" hunk headers for this file
        assert!(
            !diff_content.contains("@@"),
            "diff file should not contain hunk headers for binary/large file"
        );
    }

    // The skipped_paths should record this file
    assert!(
        report
            .skipped_paths
            .iter()
            .any(|(p, _)| p == Path::new(&file_path)),
        "skipped_paths should include the binary/large file"
    );
    // Verify SkipReason is TooLarge or NonUtf8 (both valid here — file is both)
    let skip_reason = report
        .skipped_paths
        .iter()
        .find(|(p, _)| p == Path::new(&file_path))
        .map(|(_, r)| r);
    assert!(
        matches!(
            skip_reason,
            Some(SkipReason::TooLarge(_)) | Some(SkipReason::NonUtf8)
        ),
        "skip reason should be TooLarge or NonUtf8"
    );
}

// ---------------------------------------------------------------------------
// Test 5: unchanged file does NOT emit FileChange
// ---------------------------------------------------------------------------

#[test]
fn unchanged_file_does_not_emit_file_change() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("unchanged.txt");

    fs::write(&file_path, b"same content").unwrap();

    let mut snapshotter = FileSnapshotter::new(dir.path().to_path_buf());
    snapshotter.capture(&file_path).unwrap();

    // No mutation — file stays the same

    let diff_path = dir.path().join("file_changes.diff");
    let (sink, mut rx) = make_sink();

    let report = snapshotter
        .finalize(&diff_path, &sink, "run-5", Some("step-5"))
        .unwrap();

    let events = collect_file_change_events(&mut rx);
    assert_eq!(
        events.len(),
        0,
        "no FileChange should be emitted for unchanged file"
    );

    assert!(
        report.unchanged_paths.contains(&file_path),
        "unchanged_paths should include the file"
    );
}
