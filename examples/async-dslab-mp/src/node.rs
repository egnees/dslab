//! Represents node, on which process is stored.

use dslab_core::{cast, EventHandler, Id};
use dslab_network::DataTransferCompleted;

use crate::{
    actions::LocalMessageSentAction,
    events::NetworkMessageReceived,
    message::Message,
    process::{Process, ProcessContext},
};

pub struct Node {
    process: Box<dyn Process>,
    ctx: ProcessContext,
    local_messages: Vec<Message>,
}

impl Node {
    pub fn set_process_id(&mut self, process_id: Id) {
        self.ctx.borrow_mut().set_process_id(process_id);
    }

    pub fn new(process: Box<dyn Process>, ctx: ProcessContext) -> Self {
        Self {
            process,
            ctx,
            local_messages: Vec::new(),
        }
    }

    pub fn send_local_msg(&mut self, msg: &Message) {
        self.process.on_local_message(msg, self.ctx.clone()).unwrap();
    }

    pub fn read_local_msg(&mut self) -> Option<Message> {
        self.local_messages.pop()
    }

    pub fn start(&mut self) {
        self.process.on_start(self.ctx.clone()).unwrap();
    }
}

impl EventHandler for Node {
    fn on(&mut self, event: dslab_core::Event) {
        cast!(match event.data {
            LocalMessageSentAction { msg } => {
                self.local_messages.push(msg);
            }
            DataTransferCompleted { dt: _ } => {}
            NetworkMessageReceived { from, msg } => {
                self.process.on_message(&msg, from, self.ctx.clone()).unwrap();
            }
        });
    }
}
