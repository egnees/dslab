//! Simulation events.

use serde::Serialize;

use crate::message::Message;

/// Message is received.
#[derive(Clone, Serialize)]
pub struct MessageReceived {
    /// Message identifier.
    pub id: u64,
    /// Received message.
    pub msg: Message,
    /// Name of sender process.
    pub src: String,
    /// Name of sender node.
    pub src_node: String,
    /// Name of destination process.
    pub dst: String,
    /// Name of destination node.
    pub dst_node: String,
}

/// Timer is fired.
#[derive(Clone, Serialize)]
pub struct TimerFired {
    /// Name of process that set the timer.
    pub proc: String,
    /// Timer name.
    pub timer: String,
}

/// Async activity fall asleep.
#[derive(Clone, Serialize)]
pub struct SleepStarted {
    /// Name of process fall asleep.
    pub proc: String,
    /// Duration of sleep in seconds.
    pub duration: f64,
}

/// Async activity wake up.
#[derive(Clone, Serialize)]
pub struct SleepFinished {
    /// Name of wake up process.
    pub proc: String,
}

/// Async activity ended.
#[derive(Clone, Serialize)]
pub struct ActivityFinished {
    /// Process name.
    pub proc: String,
}

/// Previously sent message is delivered or else.
/// Event is returned to the process who waits for the ack on sent message.
#[derive(Debug, Clone, Serialize)]
pub struct MessageAck {
    /// Message identifier.
    pub id: u64,
    /// If messages was delivered or not.
    pub delivered: bool,
}
