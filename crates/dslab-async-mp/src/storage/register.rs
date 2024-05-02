//! Definition of utils for register storage in the simulation.

use dslab_core::{async_core::EventKey, Simulation};
use dslab_storage::events::{DataReadCompleted, DataReadFailed, DataWriteCompleted, DataWriteFailed};

use super::event::StorageCrashedRequestInterrupt;

/// Register possible storage events in the simulation.
pub fn register_storage_key_getters(sim: &mut Simulation) {
    sim.register_key_getter_for::<DataReadCompleted>(|e| e.request_id as EventKey);
    sim.register_key_getter_for::<DataReadFailed>(|e| e.request_id as EventKey);
    sim.register_key_getter_for::<DataWriteCompleted>(|e| e.request_id as EventKey);
    sim.register_key_getter_for::<DataWriteFailed>(|e| e.request_id as EventKey);
    sim.register_key_getter_for::<StorageCrashedRequestInterrupt>(|e| e.request_id as EventKey);
}
