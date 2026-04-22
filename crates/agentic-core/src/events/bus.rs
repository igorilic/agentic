pub const DEFAULT_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct EventBus {
    _sender: tokio::sync::broadcast::Sender<crate::events::EventEnvelope>,
}

impl EventBus {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn with_capacity(_capacity: usize) -> Self {
        unimplemented!()
    }

    pub fn subscribe(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::events::EventEnvelope> {
        unimplemented!()
    }

    pub fn publish(&self, _envelope: crate::events::EventEnvelope) -> usize {
        unimplemented!()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        unimplemented!()
    }
}
