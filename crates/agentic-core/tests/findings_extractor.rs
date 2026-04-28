use agentic_core::findings::extractor::{FindingDraft, extract_findings};

#[test]
fn returns_empty_when_no_fence_present() {
    let text = "Some review notes. Nothing structured here.";
    assert_eq!(extract_findings(text), Vec::<FindingDraft>::new());
}

#[test]
fn extracts_a_single_finding_from_a_fenced_json_block() {
    let text = "Review summary.\n\
        \n\
        ```agentic-findings\n\
        [\n\
          {\n\
            \"finding_id\": \"f1\",\n\
            \"severity\": \"warning\",\n\
            \"file\": \"src/main.rs\",\n\
            \"line\": 42,\n\
            \"message\": \"missing-error-handling\"\n\
          }\n\
        ]\n\
        ```\n";
    let out = extract_findings(text);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].finding_id, "f1");
    assert_eq!(out[0].severity, "warning");
    assert_eq!(out[0].file.as_deref(), Some("src/main.rs"));
    assert_eq!(out[0].line, Some(42));
    assert_eq!(out[0].message, "missing-error-handling");
}

#[test]
fn extracts_multiple_findings_in_one_block() {
    let text = "blah\n```agentic-findings\n\
        [\n\
          {\"finding_id\":\"f1\",\"severity\":\"warning\",\"message\":\"a\"},\n\
          {\"finding_id\":\"f2\",\"severity\":\"error\",\"message\":\"b\",\"file\":\"src/x.rs\",\"line\":10}\n\
        ]\n```";
    let out = extract_findings(text);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].finding_id, "f1");
    assert!(out[0].file.is_none());
    assert_eq!(out[1].finding_id, "f2");
    assert_eq!(out[1].file.as_deref(), Some("src/x.rs"));
}

#[test]
fn returns_empty_on_malformed_json_inside_fence() {
    // Don't crash, don't surface partial data. Real reviewer agents will
    // sometimes produce invalid JSON; we drop the block entirely so callers
    // don't insert garbage.
    let text = "```agentic-findings\nthis is not json\n```";
    assert_eq!(extract_findings(text), vec![]);
}

#[test]
fn ignores_other_fenced_blocks() {
    let text = "Here is some code:\n```rust\nfn main() {}\n```\n\
        And here's the findings list:\n\
        ```agentic-findings\n[{\"finding_id\":\"x\",\"severity\":\"info\",\"message\":\"m\"}]\n```\n\
        And another code fence:\n```python\nprint('hi')\n```";
    let out = extract_findings(text);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].finding_id, "x");
}

#[test]
fn returns_empty_when_findings_array_is_empty() {
    let text = "```agentic-findings\n[]\n```";
    assert_eq!(extract_findings(text), vec![]);
}

#[test]
fn last_fenced_block_wins_when_multiple_agentic_findings_blocks_present() {
    // Reviewer might emit a draft, reconsider, then emit a final list.
    // Take the last block — that's the agent's final answer.
    let text = "First draft:\n\
        ```agentic-findings\n[{\"finding_id\":\"draft\",\"severity\":\"info\",\"message\":\"m\"}]\n```\n\
        Final:\n\
        ```agentic-findings\n[{\"finding_id\":\"final\",\"severity\":\"warning\",\"message\":\"m\"}]\n```\n";
    let out = extract_findings(text);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].finding_id, "final");
}
