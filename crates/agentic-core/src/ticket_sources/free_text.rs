use async_trait::async_trait;

use super::{Ticket, TicketSource, TicketSourceError};
use crate::events::{TicketKind, TicketRef};

/// `TicketSource` impl for `TicketKind::FreeText`. The reference IS the
/// body. Title is synthesized from the first line (or first 80 chars).
pub struct FreeTextTicketSource;

#[async_trait]
impl TicketSource for FreeTextTicketSource {
    async fn fetch(&self, reference: &TicketRef) -> Result<Ticket, TicketSourceError> {
        if !matches!(reference.kind, TicketKind::FreeText) {
            return Err(TicketSourceError::KindMismatch {
                expected: "FreeText",
                actual: reference.kind,
            });
        }
        let body = reference.reference.clone();
        let title = synthesize_title(&body);
        Ok(Ticket {
            title,
            body,
            comments: Vec::new(),
            ac_field: None,
            url: None,
        })
    }
}

fn synthesize_title(body: &str) -> String {
    let first_line = body.lines().next().unwrap_or("(empty)").trim();
    if first_line.is_empty() {
        return "(empty)".to_string();
    }
    if first_line.chars().count() <= 80 {
        first_line.to_string()
    } else {
        format!("{}…", &first_line.chars().take(77).collect::<String>())
    }
}
