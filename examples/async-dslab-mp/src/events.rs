//! Represents events which can be produced in system.

use dslab_core::Id;
use serde::Serialize;

use crate::message::Message;

/// Represents events which can be produced in system.
/// There will be timer events and network events in future.
/// For now we not need them, because there are no timers for now,
/// And network interaction is done only by async functions and futures for now.
#[derive(Clone, Debug, Serialize)]
pub struct LocalMessageReceivedEvent {
    pub msg: Message,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkMessageReceived {
    pub from: Id,
    pub msg: Message,
}
