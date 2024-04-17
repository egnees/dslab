//! Process context.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use dslab_core::async_core::AwaitResult;
use dslab_core::SimulationContext;
use rand::Rng;
use rand_pcg::Pcg64;

use crate::events::{ActivityFinished, MessageAck, SleepFinished, SleepStarted};
use crate::message::Message;
use crate::network::Network;
use crate::node::{ProcessEvent, TimerBehavior};
use crate::storage::{CreateFileError, DeleteFileError, ReadError, Storage, WriteError};

/// Proxy for interaction of a process with the system.
/// Clones of [Context] shares the same state.
#[derive(Clone)]
pub struct Context {
    proc_name: String,
    clock_skew: f64,
    rng: Rc<RefCell<Box<dyn RandomProvider>>>,
    actions_holder: Rc<RefCell<Vec<ProcessEvent>>>,
    sim_ctx: Rc<RefCell<SimulationContext>>,
    net: Rc<RefCell<Network>>,
    storage: Rc<RefCell<Storage>>,
}

trait RandomProvider {
    fn rand(&mut self) -> f64;
}

struct SimulationRng {
    sim_ctx: Rc<RefCell<SimulationContext>>,
}

impl RandomProvider for SimulationRng {
    fn rand(&mut self) -> f64 {
        self.sim_ctx.borrow().rand()
    }
}

impl RandomProvider for Pcg64 {
    fn rand(&mut self) -> f64 {
        self.gen_range(0.0..1.0)
    }
}

impl Context {
    /// Creates a context used in simulation mode.
    pub fn from_simulation(
        proc_name: String,
        actions_holder: Rc<RefCell<Vec<ProcessEvent>>>,
        sim_ctx: Rc<RefCell<SimulationContext>>,
        net: Rc<RefCell<Network>>,
        clock_skew: f64,
        storage: Rc<RefCell<Storage>>,
    ) -> Self {
        Self {
            proc_name,
            clock_skew,
            rng: Rc::new(RefCell::new(Box::new(SimulationRng {
                sim_ctx: sim_ctx.clone(),
            }))),
            actions_holder,
            sim_ctx,
            net,
            storage,
        }
    }

    /// Returns the current time from the local node clock.
    pub fn time(&self) -> f64 {
        self.sim_ctx.borrow().time() + self.clock_skew
    }

    /// Returns a random float in the range `[0, 1)`.
    pub fn rand(&self) -> f64 {
        self.rng.borrow_mut().rand()
    }

    /// Sends a message to a process.
    pub fn send(&self, msg: Message, dst: String) {
        assert!(
            msg.tip.len() <= 50,
            "Message type length exceeds the limit of 50 characters"
        );
        self.actions_holder.borrow_mut().push(ProcessEvent::MessageSent {
            msg,
            src: self.proc_name.clone(),
            dst,
            reliable: false,
        });
    }

    /// Sends a message to a process reliable.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered.
    /// - Ok if message was delivered
    pub async fn send_reliable(&self, msg: Message, dst: String) -> Result<(), String> {
        assert!(
            msg.tip.len() <= 50,
            "Message type length exceeds the limit of 50 characters"
        );

        self.actions_holder.borrow_mut().push(ProcessEvent::MessageSent {
            msg: msg.clone(),
            src: self.proc_name.clone(),
            dst: dst.clone(),
            reliable: true,
        });

        let event_key = self.net.borrow_mut().send_with_ack(msg, &self.proc_name, &dst);

        self.sim_ctx.borrow().recv_event_by_key::<MessageAck>(event_key).await;

        Ok(())
    }

    /// Sends a message to a process reliable.
    /// If message will not be delivered in specified timeout, error will be returned.
    /// It is guaranteed that message will be delivered exactly once and without corruption.
    ///
    /// # Returns
    ///
    /// - Error if message was not delivered in specified timeout.
    /// - Ok if message was delivered
    pub async fn send_reliable_timeout(&self, msg: Message, dst: String, timeout: f64) -> Result<(), String> {
        assert!(
            msg.tip.len() <= 50,
            "Message type length exceeds the limit of 50 characters"
        );

        self.actions_holder.borrow_mut().push(ProcessEvent::MessageSent {
            msg: msg.clone(),
            src: self.proc_name.clone(),
            dst: dst.clone(),
            reliable: true,
        });

        let event_key = self.net.borrow_mut().send_with_ack(msg, &self.proc_name, &dst);

        println!("timeout is {}", timeout);
        println!("simulation time is {}", self.time());

        let send_result = self
            .sim_ctx
            .borrow()
            .recv_event_by_key::<MessageAck>(event_key)
            .with_timeout(timeout)
            .await;

        match send_result {
            AwaitResult::Timeout(info) => {
                println!("after, timeout is {}", timeout);
                println!("simulation time is {}", self.time());
                Err(format!("timeout: {}", info.timeout))
            }
            AwaitResult::Ok((_, ack)) => {
                if ack.delivered {
                    Ok(())
                } else {
                    Err("message not delivered".into())
                }
            }
        }
    }

    /// Sends a local message.
    pub fn send_local(&self, msg: Message) {
        assert!(
            msg.tip.len() <= 50,
            "Message type length exceeds the limit of 50 characters"
        );
        self.actions_holder
            .borrow_mut()
            .push(ProcessEvent::LocalMessageSent { msg });
    }

    /// Sets a timer with overriding delay of existing active timer.
    pub fn set_timer(&self, name: &str, delay: f64) {
        assert!(name.len() <= 50, "Timer name length exceeds the limit of 50 characters");
        self.actions_holder.borrow_mut().push(ProcessEvent::TimerSet {
            name: name.to_string(),
            delay,
            behavior: TimerBehavior::OverrideExisting,
        });
    }

    /// Sets a timer without overriding delay of existing active timer.
    pub fn set_timer_once(&self, name: &str, delay: f64) {
        assert!(name.len() <= 50, "Timer name length exceeds the limit of 50 characters");
        self.actions_holder.borrow_mut().push(ProcessEvent::TimerSet {
            name: name.to_string(),
            delay,
            behavior: TimerBehavior::SetOnce,
        });
    }

    /// Cancels a timer.
    pub fn cancel_timer(&self, name: &str) {
        self.actions_holder
            .borrow_mut()
            .push(ProcessEvent::TimerCancelled { name: name.to_string() });
    }

    /// Sleep for `duration` seconds.
    pub async fn sleep(&self, duration: f64) {
        self.sim_ctx.borrow().emit_self_now(SleepStarted {
            proc: self.proc_name.clone(),
            duration,
        });

        self.sim_ctx.borrow().sleep(duration).await;

        self.sim_ctx.borrow().emit_self_now(SleepFinished {
            proc: self.proc_name.clone(),
        });
    }

    /// Spawn async activity.
    pub fn spawn(&self, future: impl Future<Output = ()>) {
        // Clone context to not produce multiple borrows.
        let ctx_clone = self.sim_ctx.clone();

        let process_name = self.proc_name.clone();

        self.sim_ctx.borrow().spawn(async move {
            future.await;

            // Emit event about async activity ended.
            ctx_clone
                .borrow()
                .emit_self_now(ActivityFinished { proc: process_name });
        });
    }

    /// Create file with specified name.
    ///
    /// If there is file with such name already exists,
    /// error will be returned.
    pub async fn create_file(&self, name: &str) -> Result<(), CreateFileError> {
        self.storage.borrow_mut().create_file(name).await
    }

    /// Delete file with specified name.
    pub async fn delete_file(&self, name: &str) -> Result<(), DeleteFileError> {
        self.storage.borrow_mut().delete_file(name).await
    }

    /// Read file from specified offset to the specified buffer.
    pub async fn read(&self, file: &str, offset: usize, buf: &mut [u8]) -> Result<usize, ReadError> {
        self.storage.borrow_mut().read(file, offset, buf).await
    }

    /// Read file with specified name.
    ///
    /// If there is no such file in the storage, error will be returned.
    pub async fn read_all(&self, name: &str) -> Result<Vec<u8>, ReadError> {
        self.storage.borrow_mut().read_all(name).await
    }

    /// Append data to file with specified name.
    ///
    /// If there is no enough space in the storage or file with such name not exists,
    /// error will be returned.
    pub async fn append(&self, name: &str, data: &[u8]) -> Result<(), WriteError> {
        self.storage.borrow_mut().append(name, data).await
    }
}

unsafe impl Send for Context {}

unsafe impl Sync for Context {}
