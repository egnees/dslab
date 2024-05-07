//! Definition of context of the simulation for process.

use std::{cell::RefCell, rc::Rc};

use dslab_core::async_core::AwaitResult;
use futures::{select, Future, FutureExt};

use crate::{
    log::log_entry::LogEntry,
    network::{
        event::{MessageDelivered, MessageDropped, TaggedMessageDelivered},
        message::Message,
        result::{SendError, SendResult},
        tag::Tag,
    },
    process::event::TimerFired,
    storage::{file::File, result::StorageResult},
};

use super::data::ProcessData;

/// Represents proxy between user process and simulation.
#[derive(Clone)]
pub struct Context {
    commons: Rc<RefCell<ProcessData>>,
}

impl Context {
    /// Create new context.
    pub fn new(commons: Rc<RefCell<ProcessData>>) -> Self {
        Self { commons }
    }

    /// Get node local time.
    pub fn time(&self) -> f64 {
        self.commons.borrow().control_block.borrow().ctx.time()
            + self.commons.borrow().control_block.borrow().clock_skew
    }

    /// Returns a random float in the range `[0, 1)`.
    pub fn rand(&self) -> f64 {
        self.commons.borrow().control_block.borrow().ctx.rand()
    }

    /// Send message to the other process.
    pub fn send(&self, msg: Message, dst_proc: &str) {
        assert!(
            msg.tip.len() <= 50,
            "Message type length exceeds the limit of 50 characters"
        );
        // Update send message count.
        {
            let mut commons = self.commons.borrow_mut();
            let to_self = commons.process_name != dst_proc;
            if !to_self {
                commons.send_message_cnt += 1;
            }
        }
        let from = &self.commons.borrow().process_name;
        self.commons
            .borrow()
            .control_block
            .borrow()
            .network
            .borrow_mut()
            .send_message(msg, from, dst_proc);
    }

    /// Send message to the other process and wait for the acknowledgement.
    pub async fn send_with_ack<'a>(&'a self, msg: Message, dst_proc: &'a str, timeout: f64) -> SendResult<()> {
        self.send_with_ack_tagged(msg, None, dst_proc, timeout).await
    }

    /// Send message with key.
    pub async fn send_with_tag<'a>(
        &'a self,
        msg: Message,
        tag: Tag,
        dst_proc: &'a str,
        timeout: f64,
    ) -> SendResult<()> {
        self.send_with_ack_tagged(msg, Some(tag), dst_proc, timeout).await
    }

    async fn send_with_ack_tagged<'a>(
        &'a self,
        msg: Message,
        tag: Option<Tag>,
        dst_proc: &'a str,
        timeout: f64,
    ) -> SendResult<()> {
        let from = self.commons.borrow().process_name.clone();
        // Update send message count.
        if from != dst_proc {
            self.commons.borrow_mut().send_message_cnt += 1;
        }
        let event_id = self
            .commons
            .borrow()
            .control_block
            .borrow()
            .network
            .borrow_mut()
            .send_message_with_ack(msg, &from, dst_proc, tag);

        let network_id = self.commons.borrow().control_block.borrow().network.borrow().id();
        let ctx = self.commons.borrow().control_block.borrow().ctx.clone();

        select! {
            result = ctx.recv_event_by_key_from::<MessageDelivered>(network_id, event_id).with_timeout(timeout).fuse() => {
                match result {
                    AwaitResult::Timeout(_) => Err(SendError::Timeout),
                    AwaitResult::Ok(_) => {
                        Ok(())
                    },
                }
            },
            _ = ctx.recv_event_by_key_from::<MessageDropped>(network_id, event_id).fuse() => {
                Err(SendError::NotSent)
            }
        }
    }

    /// Send message without tag to the other process and wait for the message with tag.
    async fn send_recv_tag<'a>(&'a self, msg: Message, tag: Tag, dst_proc: &'a str, timeout: f64) -> SendResult<()> {
        let from = self.commons.borrow().process_name.clone();
        // Update send message count.
        if from != dst_proc {
            self.commons.borrow_mut().send_message_cnt += 1;
        }
        let event_id = self
            .commons
            .borrow()
            .control_block
            .borrow()
            .network
            .borrow_mut()
            .send_message_with_ack(msg, &from, dst_proc, None);

        let network_id = self.commons.borrow().control_block.borrow().network.borrow().id();
        let ctx = self.commons.borrow().control_block.borrow().ctx.clone();

        select! {
            result = ctx.recv_event_by_key::<TaggedMessageDelivered>(tag).with_timeout(timeout).fuse() => {
                match result {
                    AwaitResult::Timeout(_) => Err(SendError::Timeout),
                    AwaitResult::Ok(_) => Ok(()),
                }
            },
            _ = ctx.recv_event_by_key_from::<MessageDropped>(network_id, event_id).fuse() => {
                Err(SendError::NotSent)
            }
        }
    }

    /// Send local message.
    pub fn send_local(&self, msg: Message) {
        self.commons.borrow_mut().send_local_messages_count += 1;
        self.commons.borrow_mut().local_messages.push(msg);
    }

    /// Sets a timer with overriding delay of existing active timer.
    pub fn set_timer(&self, name: &str, delay: f64) {
        assert!(name.len() <= 50, "Timer name length exceeds the limit of 50 characters");
        let timer_exists = self.commons.borrow_mut().pending_timers.contains_key(name);
        if timer_exists {
            self.cancel_timer(name);
        }

        let mut commons = self.commons.borrow_mut();
        let proc = commons.process_name.clone();

        let event_id = {
            let control_block = commons.control_block.borrow();
            let node = control_block.node_name.clone();
            let time = control_block.ctx.time();
            let event = TimerFired {
                time,
                name: name.to_owned(),
                node: node.clone(),
                proc: proc.clone(),
            };
            let event_id = control_block.ctx.emit_self(event, delay);

            control_block.logger.borrow_mut().log(LogEntry::TimerSet {
                time,
                timer_id: event_id.to_string(),
                timer_name: name.to_owned(),
                node,
                proc,
                delay,
            });

            event_id
        };

        commons.pending_timers.insert(name.to_owned(), event_id);
    }

    /// Sets a timer without overriding delay of existing active timer.
    pub fn set_timer_once(&self, name: &str, delay: f64) {
        assert!(name.len() <= 50, "Timer name length exceeds the limit of 50 characters");
        let timer_exists = self.commons.borrow_mut().pending_timers.contains_key(name);
        if timer_exists {
            return;
        }
        self.set_timer(name, delay)
    }

    /// Cancels a timer.
    pub fn cancel_timer(&self, name: &str) {
        let timer_exists = self.commons.borrow_mut().pending_timers.contains_key(name);
        if !timer_exists {
            return;
        }
        let mut commons = self.commons.borrow_mut();
        let proc = commons.process_name.clone();
        let event_id = commons.pending_timers.remove(name).unwrap();
        let control_block = commons.control_block.borrow();
        let time = control_block.ctx.time();
        let node = control_block.node_name.clone();
        control_block.ctx.cancel_event(event_id);
        control_block.logger.borrow_mut().log(LogEntry::TimerCancelled {
            time,
            timer_id: event_id.to_string(),
            timer_name: name.to_owned(),
            node,
            proc,
        });
    }

    /// Spawn async activity.
    pub fn spawn(&self, future: impl Future<Output = ()>) {
        // Clone context to not produce multiple borrows.
        self.commons.borrow().control_block.borrow().ctx.spawn(future);
        // FIXME: fix lifetimes here
    }

    /// Create file with specified name.
    pub fn create_file(&self, name: &str) -> StorageResult<File> {
        self.commons
            .borrow()
            .control_block
            .borrow_mut()
            .file_manager
            .create_file(name)
    }

    /// Check if file with specified name exists.
    pub fn file_exists(&self, name: &str) -> StorageResult<bool> {
        self.commons
            .borrow()
            .control_block
            .borrow_mut()
            .file_manager
            .file_exists(name)
    }

    /// Open file with specified name.
    pub fn open_file(&self, name: &str) -> StorageResult<File> {
        self.commons
            .borrow()
            .control_block
            .borrow_mut()
            .file_manager
            .open_file(name)
    }
}
