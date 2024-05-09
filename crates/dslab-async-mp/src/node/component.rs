//! Node implementation.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use colored::*;

use dslab_core::{cast, Event, EventHandler, Id, SimulationContext};
use dslab_storage::storage::Storage;

use crate::log::log_entry::LogEntry;
use crate::log::logger::Logger;
use crate::network::event::{MessageDelivered, TaggedMessageDelivered};
use crate::network::message::Message;
use crate::network::model::Network;
use crate::process::context::Context;
use crate::process::data::ProcessData;
use crate::process::event::TimerFired;
use crate::process::process::Process;
use crate::storage::file_manager::FileManager;

use super::interaction::InteractionBlock;

struct ProcessEntry {
    pub proc_impl: Box<dyn Process>,
    pub data: Rc<RefCell<ProcessData>>,
}

enum State {
    Running,
    Shut,
    Crashed,
}

/// Represents a node which is connected to the network and hosts one or more processes.
pub struct Node {
    /// Identifier of simulation component.
    pub id: Id,
    control: Rc<RefCell<InteractionBlock>>,
    processes: HashMap<String, ProcessEntry>,
    state: State,
}

impl Node {
    pub(crate) fn new(
        name: String,
        ctx: SimulationContext,
        net: Rc<RefCell<Network>>,
        logger: Rc<RefCell<Logger>>,
        storage_model: Rc<RefCell<dyn Storage>>,
    ) -> Self {
        let id = ctx.id();

        let file_manager = FileManager::new(storage_model, ctx.clone());
        let control_block = InteractionBlock {
            network: net,
            file_manager,
            logger,
            ctx,
            clock_skew: 0.,
            node_name: name,
        };
        let control = Rc::new(RefCell::new(control_block));

        let state = State::Running;

        Self {
            id,
            control,
            processes: HashMap::new(),
            state,
        }
    }

    /// Returns the node name.
    pub fn name(&self) -> String {
        self.control.borrow().node_name.clone()
    }

    /// Sets the node clock skew.
    pub fn set_clock_skew(&mut self, clock_skew: f64) {
        self.control.borrow_mut().clock_skew = clock_skew;
    }

    /// Returns the node clock skew.
    pub(crate) fn clock_skew(&self) -> f64 {
        self.control.borrow().clock_skew
    }

    /// Returns true if the node is crashed.
    pub fn is_crashed(&self) -> bool {
        match self.state {
            State::Running => false,
            State::Shut => false,
            State::Crashed => true,
        }
    }

    /// Returns true if node is shutdown.
    pub fn is_shut(&self) -> bool {
        match self.state {
            State::Running => false,
            State::Shut => true,
            State::Crashed => false,
        }
    }

    /// Marks the node as shut.
    /// Storage will not be crashed.
    /// All pending events to the node will be cancelled.
    pub fn shutdown(&mut self) {
        match self.state {
            State::Running => {
                self.state = State::Shut; // Processes will be removed on rerunning.
                let name = self.control.borrow().node_name.clone();
                self.control.borrow().network.borrow_mut().disconnect_node(&name);
            }
            State::Shut => panic!("trying to shutdown turned off node"),
            State::Crashed => panic!("trying to shutdown crashed node"),
        }
    }

    /// Run node after shutdown.
    pub fn rerun(&mut self) {
        match self.state {
            State::Running => panic!("trying to rerun running node"),
            State::Shut => {
                // Remove process on rerunning to allow working with them after shutdown.
                self.processes.clear();
                self.state = State::Running;
                let name = self.control.borrow().node_name.clone();
                self.control.borrow().network.borrow_mut().connect_node(&name);
            }
            State::Crashed => panic!("trying to rerun crashed node"),
        }
    }

    /// Marks the node and storage as crashed.
    /// All pending events to the node will be cancelled.
    pub fn crash(&mut self) {
        // Node in every state can be crashed.
        match self.state {
            State::Crashed => {}
            _ => {
                self.control.borrow_mut().file_manager.crash_storage();
                let name = self.control.borrow().node_name.clone();
                self.control.borrow().network.borrow_mut().disconnect_node(&name);
                self.state = State::Crashed;
            }
        }
    }

    /// Recovers the node and storage after crash.
    pub fn recover(&mut self) {
        match self.state {
            State::Running => panic!("trying to recover running node"),
            State::Shut => panic!("trying to recover turned off, but not crashed node."),
            State::Crashed => {
                // processes are cleared on recover instead of the crash
                // to allow working with processes after the crash (i.e. examine event log)
                self.processes.clear();
                self.control.borrow_mut().file_manager.recover_storage();
                let name = self.control.borrow().node_name.clone();
                self.control.borrow().network.borrow_mut().connect_node(&name);
                self.state = State::Running;
            }
        }
    }

    /// Spawns new process on the node.
    pub fn add_process(&mut self, name: &str, proc: Box<dyn Process>) {
        let proc_data = ProcessData::new(name.to_owned(), self.control.clone());
        self.processes.insert(
            name.to_string(),
            ProcessEntry {
                proc_impl: proc,
                data: Rc::new(RefCell::new(proc_data)),
            },
        );
    }

    /// Returns a local process by its name.
    pub fn get_process(&self, name: &str) -> Option<&dyn Process> {
        self.processes.get(name).map(|entry| &*entry.proc_impl)
    }

    /// Returns the names of all local processes.
    pub fn process_names(&self) -> Vec<String> {
        self.processes.keys().cloned().collect()
    }

    /// Sends a local message to the process.
    pub fn send_local_message(&mut self, proc: String, msg: Message) {
        self.on_local_message_received(proc, msg);
    }

    /// Reads and returns the local messages produced by the process.
    ///
    /// Returns `None` if there are no messages.   
    pub fn read_local_messages(&mut self, proc: &str) -> Option<Vec<Message>> {
        let proc_data = self.processes.get_mut(proc).unwrap().data.clone();
        let mut proc_data = proc_data.borrow_mut();
        if proc_data.local_messages.is_empty() {
            None
        } else {
            let len = proc_data.local_messages.len();
            Some(proc_data.local_messages.drain(0..len).collect())
        }
    }

    /// Returns a copy of the local messages produced by the process.
    ///
    /// In contrast to [`Self::read_local_messages`], this method does not drain the process outbox.
    pub fn local_outbox(&self, proc: &str) -> Vec<Message> {
        self.processes.get(proc).unwrap().data.borrow().local_messages.clone()
    }

    /// Returns the number of messages sent by the process.
    pub fn sent_message_count(&self, proc: &str) -> u64 {
        self.processes[proc].data.borrow().send_message_cnt
    }

    /// Returns the number of messages received by the process.
    pub fn received_message_count(&self, proc: &str) -> u64 {
        self.processes[proc].data.borrow().received_message_cnt
    }

    fn on_local_message_received(&mut self, proc: String, msg: Message) {
        let time = self.control.borrow().ctx.time();
        let name = self.control.borrow().node_name.clone();

        let msg_id = {
            let proc_entry = self.processes.get(&proc).unwrap();
            proc_entry.data.borrow_mut().received_local_messages_count += 1;
            proc_entry.data.borrow().received_local_messages_count
        };
        let msg_id = self.get_local_message_id(&proc, msg_id);

        self.control
            .borrow()
            .logger
            .borrow_mut()
            .log(LogEntry::LocalMessageReceived {
                time,
                msg_id,
                node: name,
                proc: proc.to_string(),
                msg: msg.clone(),
            });

        let proc_entry = self.processes.get_mut(&proc).unwrap();
        let ctx = Context::new(proc_entry.data.clone());
        proc_entry
            .proc_impl
            .on_local_message(msg, ctx)
            .map_err(|e| self.handle_process_error(e, proc.clone()))
            .unwrap();
    }

    fn on_message_received(&mut self, msg_id: u64, proc: String, msg: Message, from: String, from_node: String) {
        let control = self.control.borrow();
        let time = control.ctx.time();
        let name = control.node_name.clone();

        control.logger.borrow_mut().log(LogEntry::MessageReceived {
            time,
            msg_id: msg_id.to_string(),
            src_proc: from.clone(),
            src_node: from_node,
            dst_proc: proc.clone(),
            dst_node: name,
            msg: msg.clone(),
        });

        let proc_entry = self.processes.get_mut(&proc).unwrap();
        proc_entry.data.borrow_mut().received_message_cnt += 1;
        let ctx = Context::new(proc_entry.data.clone());
        let _ = proc_entry
            .proc_impl
            .on_message(msg, from, ctx)
            .map_err(|e| self.handle_process_error(e, proc));
    }

    fn on_timer_fired(&mut self, proc: String, timer: String) {
        let control = self.control.borrow();
        let time = control.ctx.time();
        let name = control.node_name.clone();
        let proc_entry = self.processes.get_mut(&proc).unwrap();
        let timer_id = proc_entry.data.borrow_mut().pending_timers.remove(&timer).unwrap();
        control.logger.borrow_mut().log(LogEntry::TimerFired {
            time,
            timer_id: timer_id.to_string(),
            timer_name: timer.clone(),
            node: name,
            proc: proc.clone(),
        });
        let ctx = Context::new(proc_entry.data.clone());
        let _ = proc_entry
            .proc_impl
            .on_timer(timer, ctx)
            .map_err(|e| self.handle_process_error(e, proc));
    }

    fn get_local_message_id(&self, proc: &str, local_message_count: u64) -> String {
        format!("{}-{}-{}", self.control.borrow().node_name, proc, local_message_count)
    }

    fn handle_process_error(&self, err: String, proc: String) -> &str {
        eprintln!(
            "{}",
            format!(
                "\n!!! Error when calling process '{}' on node '{}':\n\n{}",
                proc,
                self.control.borrow().node_name,
                err
            )
            .red()
        );
        "Error when calling process"
    }
}

impl EventHandler for Node {
    fn on(&mut self, event: Event) {
        cast!(match event.data {
            MessageDelivered {
                msg_id,
                msg,
                src_proc,
                src_node,
                dst_proc,
                dst_node: _,
            } => {
                let network_id = self.control.borrow().network.borrow().id();
                if network_id != event.src {
                    self.on_message_received(msg_id, dst_proc, msg, src_proc, src_node);
                }
            }
            TaggedMessageDelivered {
                msg_id,
                msg,
                src_proc,
                src_node,
                dst_proc,
                dst_node: _,
                tag: _,
            } => {
                assert!(event.src != self.control.borrow().network.borrow().id());
                self.on_message_received(msg_id, dst_proc, msg, src_proc, src_node);
            }
            TimerFired {
                time: _,
                name,
                node: _,
                proc,
            } => {
                self.on_timer_fired(proc, name);
            }
        })
    }
}
