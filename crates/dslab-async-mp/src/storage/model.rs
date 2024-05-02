pub use dslab_storage::storage::Storage as StorageModel;

use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use dslab_core::SimulationContext;

use super::event::StorageCrashedRequestInterrupt;

/// Represents state of the storage with associated model.
pub struct ModelWrapper {
    available: bool,
    model: Rc<RefCell<dyn StorageModel>>,
    owner_ctx: SimulationContext,
    requests_registry: BTreeSet<u64>,
}

impl ModelWrapper {
    /// Create new wrapper for the storage model.
    pub fn new(model: Rc<RefCell<dyn StorageModel>>, owner_ctx: SimulationContext) -> Self {
        Self {
            available: true,
            model,
            owner_ctx,
            requests_registry: BTreeSet::new(),
        }
    }

    /// Crash storage.
    pub fn crash(&mut self) {
        assert!(self.available, "trying to crash not available storage");
        self.owner_ctx.cancel_events(|e| e.dst == self.owner_ctx.id()); // cancel all future events from storage.
        for request_id in self.requests_registry.iter() {
            self.owner_ctx.emit_self_now(StorageCrashedRequestInterrupt {
                request_id: *request_id,
            });
        }
        self.requests_registry.clear();
        self.available = false;
    }

    /// Recover crashed storage.
    pub fn recover(&mut self) {
        assert!(!self.available, "trying to recover not crashed storage");
        let used_space = self.model.borrow().used_space();
        self.model.borrow_mut().mark_free(used_space).unwrap();
        self.available = true;
    }

    /// Check if storage is not crashed.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Read requested number of bytes from storage.
    pub fn read(&mut self, bytes: u64) -> u64 {
        let request_id = self.model.borrow_mut().read(bytes, self.owner_ctx.id());
        self.register_request(request_id);
        request_id
    }

    /// Write requested number of bytes to storage.
    pub fn write(&mut self, bytes: u64) -> u64 {
        let request_id = self.model.borrow_mut().write(bytes, self.owner_ctx.id());
        self.register_request(request_id);
        request_id
    }

    /// Returns free space.
    pub fn free_space(&self) -> u64 {
        self.model.borrow().free_space()
    }

    fn register_request(&mut self, request_id: u64) {
        self.requests_registry.insert(request_id);
    }

    /// Mark request as processed.
    pub fn mark_request_as_processed(&mut self, request_id: u64) {
        self.requests_registry.remove(&request_id);
    }
}
