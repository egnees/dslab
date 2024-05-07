//! Utils for registering network in the simulation.
use super::event::{MessageDelivered, MessageDropped, TaggedMessageDelivered};
use dslab_core::{async_core::EventKey, Simulation};

/// Register possible network events in the simulation.
pub fn register_network_key_getters(sim: &mut Simulation) {
    sim.register_key_getter_for::<MessageDelivered>(|e| e.msg_id as EventKey);
    sim.register_key_getter_for::<MessageDropped>(|e| e.msg_id as EventKey);
    sim.register_key_getter_for::<TaggedMessageDelivered>(|e| e.tag as EventKey);
}
