use async_trait::async_trait;

use crate::events::{TicketKind, TicketRef};
use super::{Ticket, TicketSource, TicketSourceError};

/// `TicketSource` impl for `TicketKind::FreeText`. The reference IS the
/// body. Title is synthesized from the first line (or first 80 chars).
pub struct FreeTextTicketSource;

#[async_trait]
impl TicketSource for FreeTextTicketSource {
    async fn fetch(&self, reference: &TicketRef) -> Result<Ticket, TicketSourceError> {
        todo!("implement FreeTextTicketSource::fetch")
    }
}

fn synthesize_title(body: &str) -> String {
    todo!("implement synthesize_title")
}
