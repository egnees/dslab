use serde::Serialize;

/// Represents possible storage events.

#[derive(Clone, Serialize)]
pub struct StorageCrashedRequestInterrupt {
    /// Request id.
    pub request_id: u64,
}
