//! Definition of the process data.
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dslab_core::event::EventId;

use crate::node::interaction::InteractionBlock;

use crate::network::message::Message;

/// Intended for collect common info from different instances of context of the single process.
#[derive(Clone)]
pub struct ProcessData {
    /// Name of the process.
    pub process_name: String,
    /// Pending timers (name -> simulation id).
    pub pending_timers: HashMap<String, EventId>,
    /// Local messages.
    pub local_messages: Vec<Message>,
    /// Send messages count.
    pub send_message_cnt: u64,
    /// Received messages count.
    pub received_message_cnt: u64,
    /// Total number of received local messages.
    pub received_local_messages_count: u64,
    /// Total number of sent local messages.
    pub send_local_messages_count: u64,
    /// Control block for interaction with simulation.
    pub control_block: Rc<RefCell<InteractionBlock>>,
}

impl ProcessData {
    pub fn new(process_name: String, control_block: Rc<RefCell<InteractionBlock>>) -> Self {
        Self {
            process_name,
            pending_timers: HashMap::new(),
            local_messages: Vec::new(),
            send_message_cnt: 0,
            received_message_cnt: 0,
            received_local_messages_count: 0,
            send_local_messages_count: 0,
            control_block,
        }
    }
}
