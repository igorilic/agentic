use agentic_core::events::{TicketKind, TicketRef};
use agentic_core::ticket_sources::{FreeTextTicketSource, Ticket, TicketSource, TicketSourceError};

fn make_ref(kind: TicketKind, reference: &str) -> TicketRef {
    TicketRef {
        kind,
        reference: reference.to_string(),
        title: None,
    }
}

#[tokio::test]
async fn free_text_returns_body_as_is() {
    let src = FreeTextTicketSource;
    let r = make_ref(TicketKind::FreeText, "create README.md\n\nWith hello world.");
    let ticket: Ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.body, "create README.md\n\nWith hello world.");
    assert!(ticket.comments.is_empty());
    assert!(ticket.ac_field.is_none());
    assert!(ticket.url.is_none());
}

#[tokio::test]
async fn free_text_synthesizes_title_from_first_line() {
    let src = FreeTextTicketSource;
    let r = make_ref(TicketKind::FreeText, "fix the bug in main.rs\n\nDetails follow.");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "fix the bug in main.rs");
}

#[tokio::test]
async fn free_text_truncates_long_first_line_to_80_chars_with_ellipsis() {
    let src = FreeTextTicketSource;
    let long = "a".repeat(200);
    let r = make_ref(TicketKind::FreeText, &long);
    let ticket = src.fetch(&r).await.unwrap();
    // 77 chars of content + 1 char ellipsis = 78 visual chars, but byte count varies.
    assert!(ticket.title.starts_with("aaaa"));
    assert!(ticket.title.ends_with("…"));
    assert_eq!(ticket.title.chars().count(), 78);
}

#[tokio::test]
async fn free_text_handles_empty_body() {
    let src = FreeTextTicketSource;
    let r = make_ref(TicketKind::FreeText, "");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.body, "");
    assert_eq!(ticket.title, "(empty)");
}

#[tokio::test]
async fn free_text_rejects_non_free_text_kind() {
    let src = FreeTextTicketSource;
    let r = make_ref(TicketKind::GithubIssue, "owner/repo#1");
    let result = src.fetch(&r).await;
    assert!(matches!(
        result,
        Err(TicketSourceError::KindMismatch {
            expected: "FreeText",
            ..
        })
    ));
}
