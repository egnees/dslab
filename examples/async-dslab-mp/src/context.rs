use std::{cell::RefCell, rc::Rc};

use dslab_core::{async_core::EventKey, Id, SimulationContext};
use dslab_network::{DataTransferCompleted, Network};
use dslab_storage::{
    disk::Disk,
    events::{DataReadCompleted, DataWriteCompleted},
    storage::Storage,
};
use futures::Future;

use crate::{
    actions::LocalMessageSentAction, events::NetworkMessageReceived, filesystem::Filesystem, message::Message,
};

/// Corresponds to the simulation implementation of context trait.
/// Every process own its context.
pub struct VirtualContext {
    /// Corresponds to context of process component.
    ctx: SimulationContext,
    /// Represents filesystem.
    filesystem: Rc<RefCell<Filesystem>>,
    /// Corresponds to the process disk.
    disk: Rc<RefCell<Disk>>,
    /// Corresponds to the network.
    net: Rc<RefCell<Network>>,
    /// Corresponds to the process component id.
    process_id: Id,
}

impl VirtualContext {
    /// Create virtual context.
    pub fn new(ctx: SimulationContext, disk: Rc<RefCell<Disk>>, net: Rc<RefCell<Network>>) -> Self {
        Self {
            ctx,
            filesystem: Rc::new(RefCell::new(Filesystem::default())),
            disk,
            net,
            process_id: 0,
        }
    }

    /// Allows to set valid process id.
    pub fn set_process_id(&mut self, process_id: Id) {
        self.process_id = process_id;
    }

    /// Allows to send message reliable.
    /// Add timeout here (?).
    pub async fn send_msg_reliable(&self, msg: Message, to: Id) -> Result<(), String> {
        let transfer_id =
            self.net
                .borrow_mut()
                .transfer_data(self.process_id, to, msg.get_raw_data().len() as f64, self.process_id);

        self.ctx
            .recv_event_by_key::<DataTransferCompleted>(transfer_id as EventKey)
            .await;

        println!("sent message from {} to {}", self.process_id, to);

        self.ctx.emit_now(
            NetworkMessageReceived {
                from: self.process_id,
                msg,
            },
            to,
        );

        Ok(())
    }

    pub fn spawn(&self, future: impl Future<Output = ()>) {
        self.ctx.spawn(future);
    }

    /// Allows to get current time.
    pub fn get_time(&self) -> f64 {
        self.ctx.time()
    }

    /// Allows to send local message.
    pub fn send_local_msg(&self, msg: Message) -> Result<(), String> {
        let action = LocalMessageSentAction { msg };
        self.ctx.emit_self_now(action);
        Ok(())
    }

    pub async fn read_from_file(&self, filename: &str, offset: usize, len: usize) -> Result<String, String> {
        if !self.filesystem.borrow().contains_file(filename) {
            Err("File does not exists.".to_string())
        } else {
            if !self.filesystem.borrow().can_be_read(filename, offset, len) {
                Err("Bad read offset and len".to_string())
            } else {
                let read_id = self.disk.borrow_mut().read(len as u64, self.process_id as Id);
                self.ctx.recv_event_by_key::<DataReadCompleted>(read_id).await;
                self.filesystem.borrow().read_file(filename, offset, len)
            }
        }
    }

    pub async fn append_to_file(&self, filename: &str, info: &str) -> Result<usize, String> {
        if !self.filesystem.borrow().contains_file(filename) {
            Err("File does not exists.".to_string())
        } else {
            let write_id = self
                .disk
                .borrow_mut()
                .write(filename.len() as u64, self.process_id as Id);
            self.ctx.recv_event_by_key::<DataWriteCompleted>(write_id).await;
            self.filesystem.borrow_mut().append_to_file(filename, info)
        }
    }

    pub fn create_file(&self, filename: &str) -> Result<(), String> {
        self.filesystem.borrow_mut().create_file(filename)
    }
}
