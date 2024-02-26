//! Process context.

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use dslab_core::SimulationContext;
use rand::Rng;
use rand_pcg::Pcg64;

use crate::events::{ActivityFinished, SleepFinished, SleepStarted};
use crate::message::Message;
use crate::node::{ProcessEvent, TimerBehavior};

/// Proxy for interaction of a process with the system.
/// Clones of [Context] shares the same state.
#[derive(Clone)]
pub struct Context {
    proc_name: String,
    time: f64,
    rng: Rc<RefCell<Box<dyn RandomProvider>>>,
    actions_holder: Rc<RefCell<Vec<ProcessEvent>>>,
    sim_ctx: Rc<RefCell<SimulationContext>>,
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
        clock_skew: f64,
    ) -> Self {
        let time = sim_ctx.borrow().time() + clock_skew;
        Self {
            proc_name,
            time,
            rng: Rc::new(RefCell::new(Box::new(SimulationRng {
                sim_ctx: sim_ctx.clone(),
            }))),
            actions_holder,
            sim_ctx,
        }
    }

    /// Returns the current time from the local node clock.
    pub fn time(&self) -> f64 {
        self.time
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
        });
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
}
