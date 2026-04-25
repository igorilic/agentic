use std::fs;
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

    let mut snapshotter = FileSnapshotter::new();
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

    // diff file should exist and contain a unified-diff section with --- and +++ headers
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
    let mut snapshotter = FileSnapshotter::new();
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

    // F4: assert the diff file content contains expected markers for a create
    let diff_content = fs::read_to_string(&diff_path).expect("diff file should exist");
    assert!(
        diff_content.contains("---"),
        "create diff should have --- header: {diff_content}"
    );
    assert!(
        diff_content.contains("+++"),
        "create diff should have +++ header: {diff_content}"
    );
    assert!(
        diff_content.contains('+'),
        "create diff should contain added lines: {diff_content}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: deleted file emits FileChange with empty/absent after_hash
// ---------------------------------------------------------------------------

#[test]
fn file_deleted_emits_file_change_with_absent_after_hash() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("to_delete.txt");

    fs::write(&file_path, b"will be deleted").unwrap();

    let mut snapshotter = FileSnapshotter::new();
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

    // F4: assert the diff file content contains expected markers for a delete
    let diff_content = fs::read_to_string(&diff_path).expect("diff file should exist");
    assert!(
        diff_content.contains("---"),
        "delete diff should have --- header: {diff_content}"
    );
    assert!(
        diff_content.contains("+++"),
        "delete diff should have +++ header: {diff_content}"
    );
    assert!(
        diff_content.contains('-'),
        "delete diff should contain removed lines: {diff_content}"
    );
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

    let mut snapshotter = FileSnapshotter::new();
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
    assert_eq!(
        events.len(),
        1,
        "expected one FileChange event for binary file"
    );

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

    let mut snapshotter = FileSnapshotter::new();
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

// ---------------------------------------------------------------------------
// Test 6: multi-file diff output is deterministic (F2)
// ---------------------------------------------------------------------------

#[test]
fn multi_file_diff_output_is_deterministic() {
    let dir = TempDir::new().unwrap();

    // Create three files in non-alphabetical order
    let zebra = dir.path().join("zebra.txt");
    let alpha = dir.path().join("alpha.txt");
    let middle = dir.path().join("middle.txt");

    fs::write(&zebra, b"zebra original\n").unwrap();
    fs::write(&alpha, b"alpha original\n").unwrap();
    fs::write(&middle, b"middle original\n").unwrap();

    // First run: capture, mutate, finalize
    let mut snap1 = FileSnapshotter::new();
    snap1.capture(&zebra).unwrap();
    snap1.capture(&alpha).unwrap();
    snap1.capture(&middle).unwrap();

    fs::write(&zebra, b"zebra modified\n").unwrap();
    fs::write(&alpha, b"alpha modified\n").unwrap();
    fs::write(&middle, b"middle modified\n").unwrap();

    let diff_path1 = dir.path().join("changes1.diff");
    let (sink1, _rx1) = broadcast::channel(64);
    snap1
        .finalize(&diff_path1, &sink1, "run-det-1", Some("step-1"))
        .unwrap();

    // Reset files for second run
    fs::write(&zebra, b"zebra original\n").unwrap();
    fs::write(&alpha, b"alpha original\n").unwrap();
    fs::write(&middle, b"middle original\n").unwrap();

    // Second run (captures in different order to stress HashMap non-determinism)
    let mut snap2 = FileSnapshotter::new();
    snap2.capture(&middle).unwrap();
    snap2.capture(&zebra).unwrap();
    snap2.capture(&alpha).unwrap();

    fs::write(&zebra, b"zebra modified\n").unwrap();
    fs::write(&alpha, b"alpha modified\n").unwrap();
    fs::write(&middle, b"middle modified\n").unwrap();

    let diff_path2 = dir.path().join("changes2.diff");
    let (sink2, _rx2) = broadcast::channel(64);
    snap2
        .finalize(&diff_path2, &sink2, "run-det-2", Some("step-2"))
        .unwrap();

    let content1 = fs::read_to_string(&diff_path1).unwrap();
    let content2 = fs::read_to_string(&diff_path2).unwrap();

    assert_eq!(
        content1, content2,
        "diff output must be byte-identical regardless of capture order"
    );

    // Verify sections appear in sorted order: alpha < middle < zebra
    let pos_alpha = content1
        .find("alpha")
        .expect("alpha section should be present");
    let pos_middle = content1
        .find("middle")
        .expect("middle section should be present");
    let pos_zebra = content1
        .find("zebra")
        .expect("zebra section should be present");

    assert!(
        pos_alpha < pos_middle,
        "alpha should appear before middle in sorted diff"
    );
    assert!(
        pos_middle < pos_zebra,
        "middle should appear before zebra in sorted diff"
    );
}

// ---------------------------------------------------------------------------
// Test 7: modify diff contains valid unified-diff markers (patch-applicability)
// ---------------------------------------------------------------------------

#[test]
fn modify_diff_contains_unified_diff_markers() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("patch_test.txt");

    fs::write(&file_path, b"line one\nline two\nline three\n").unwrap();

    let mut snapshotter = FileSnapshotter::new();
    snapshotter.capture(&file_path).unwrap();

    fs::write(&file_path, b"line one\nline two modified\nline three\n").unwrap();

    let diff_path = dir.path().join("file_changes.diff");
    let (sink, _rx) = broadcast::channel(64);

    snapshotter
        .finalize(&diff_path, &sink, "run-patch", Some("step-patch"))
        .unwrap();

    let diff_content = fs::read_to_string(&diff_path).expect("diff file should exist");

    assert!(
        diff_content.contains("---"),
        "diff must contain --- header: {diff_content}"
    );
    assert!(
        diff_content.contains("+++"),
        "diff must contain +++ header: {diff_content}"
    );
    assert!(
        diff_content.contains("@@"),
        "diff must contain @@ hunk header: {diff_content}"
    );
}
