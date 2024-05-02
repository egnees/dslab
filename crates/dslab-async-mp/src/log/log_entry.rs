//! Definition of events for logging.

use dslab_core::Id;

use crate::{log::util::t, network::message::Message};

use serde::Serialize;

/// Represents a logged event.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum LogEntry {
    NodeStarted {
        time: f64,
        node: String,
        node_id: Id,
    },
    ProcessStarted {
        time: f64,
        node: String,
        proc: String,
    },
    LocalMessageSent {
        time: f64,
        msg_id: String,
        node: String,
        proc: String,
        msg: Message,
    },
    LocalMessageReceived {
        time: f64,
        msg_id: String,
        node: String,
        proc: String,
        msg: Message,
    },
    MessageSent {
        time: f64,
        msg_id: String,
        src_node: String,
        src_proc: String,
        dst_node: String,
        dst_proc: String,
        msg: Message,
    },
    MessageReceived {
        time: f64,
        msg_id: String,
        #[serde(skip_serializing)]
        src_node: String,
        #[serde(skip_serializing)]
        src_proc: String,
        #[serde(skip_serializing)]
        dst_node: String,
        #[serde(skip_serializing)]
        dst_proc: String,
        #[serde(skip_serializing)]
        msg: Message,
    },
    MessageDropped {
        time: f64,
        msg_id: String,
        #[serde(skip_serializing)]
        src_node: String,
        #[serde(skip_serializing)]
        src_proc: String,
        #[serde(skip_serializing)]
        dst_node: String,
        #[serde(skip_serializing)]
        dst_proc: String,
        #[serde(skip_serializing)]
        msg: Message,
    },
    NodeDisconnected {
        time: f64,
        node: String,
    },
    NodeConnected {
        time: f64,
        node: String,
    },
    NodeCrashed {
        time: f64,
        node: String,
    },
    NodeRecovered {
        time: f64,
        node: String,
    },
    NodeShutdown {
        time: f64,
        node: String,
    },
    NodeReran {
        time: f64,
        node: String,
    },
    TimerSet {
        time: f64,
        timer_id: String,
        timer_name: String,
        node: String,
        proc: String,
        delay: f64,
    },
    TimerFired {
        time: f64,
        timer_id: String,
        #[serde(skip_serializing)]
        timer_name: String,
        #[serde(skip_serializing)]
        node: String,
        #[serde(skip_serializing)]
        proc: String,
    },
    TimerCancelled {
        time: f64,
        timer_id: String,
        #[serde(skip_serializing)]
        timer_name: String,
        #[serde(skip_serializing)]
        node: String,
        #[serde(skip_serializing)]
        proc: String,
    },
    /// Link between a pair of nodes is disabled.
    LinkDisabled {
        time: f64,
        from: String,
        to: String,
    },
    /// Link between a pair of nodes is enabled.
    LinkEnabled {
        time: f64,
        from: String,
        to: String,
    },
    /// Dropping of incoming messages for a node is enabled.
    DropIncoming {
        time: f64,
        node: String,
    },
    /// Dropping of incoming messages for a node is disabled.
    PassIncoming {
        time: f64,
        node: String,
    },
    /// Dropping of outgoing messages for a node is enabled.
    DropOutgoing {
        time: f64,
        node: String,
    },
    /// Dropping of outgoing messages for a node is enabled.
    PassOutgoing {
        time: f64,
        node: String,
    },
    /// Network partition is occurred between two groups of nodes.
    NetworkPartition {
        time: f64,
        /// First group of nodes.
        group1: Vec<String>,
        /// Second group of nodes.
        group2: Vec<String>,
    },
    /// Network is reset to normal state (all links are working).
    NetworkReset {
        time: f64,
    },
    /// Requested reading file from storage.
    ReadFromFile {
        time: f64,
        node: String,
        request_id: u64,
        file_name: String,
        bytes: u64,
    },
    /// Requested writing file to storage.
    WriteToFile {
        time: f64,
        node: String,
        request_id: u64,
        file_name: String,
        bytes: u64,
    },
    /// Read request to file succeed.
    ReadRequestSucceed {
        time: f64,
        node: String,
        request_id: u64,
        file_name: String,
        bytes: u64,
    },
    /// Read request to file failed.
    ReadRequestFailed {
        time: f64,
        node: String,
        request_id: u64,
        file_name: String,
        reason: String,
        bytes: u64,
    },
    /// Write request to file succeed.
    WriteRequestSucceed {
        time: f64,
        node: String,
        request_id: u64,
        file_name: String,
        bytes: u64,
    },
    /// Write request to file failed.
    WriteRequestFailed {
        time: f64,
        node: String,
        request_id: u64,
        file_name: String,
        reason: String,
        bytes: u64,
    },
}

use colored::Colorize;

impl LogEntry {
    /// Prints log entry to console.
    pub fn print(&self) {
        match self {
            LogEntry::NodeStarted { .. } => {
                // t!(format!("{:>9.3} - node started: {}", time, node));
            }
            LogEntry::ProcessStarted { .. } => {
                // t!(format!("{:>9.3} - process started: {} @ {}", time, proc, node));
            }
            LogEntry::LocalMessageSent {
                time,
                msg_id: _,
                node: _,
                proc,
                msg,
            } => {
                t!(format!("{:>9.3} {:>10} >>> {:<10} {:?}", time, proc, "local", msg).green());
            }
            LogEntry::LocalMessageReceived {
                time,
                msg_id: _,
                node: _,
                proc,
                msg,
            } => {
                t!(format!("{:>9.3} {:>10} <<< {:<10} {:?}", time, proc, "local", msg).cyan());
            }
            LogEntry::MessageSent {
                time,
                msg_id: _,
                src_node: _,
                src_proc,
                dst_node: _,
                dst_proc,
                msg,
            } => {
                t!(format!("{:>9.3} {:>10} --> {:<10} {:?}", time, src_proc, dst_proc, msg));
            }
            LogEntry::MessageReceived {
                time,
                msg_id: _,
                src_proc,
                src_node: _,
                dst_proc,
                dst_node: _,
                msg,
            } => {
                t!(format!("{:>9.3} {:>10} <-- {:<10} {:?}", time, dst_proc, src_proc, msg))
            }
            LogEntry::MessageDropped {
                time: _,
                msg_id: _,
                src_proc,
                src_node: _,
                dst_proc,
                dst_node: _,
                msg,
            } => {
                t!(format!(
                    "{:>9} {:>10} --x {:<10} {:?} <-- message dropped",
                    "!!!", src_proc, dst_proc, msg
                )
                .red());
            }
            LogEntry::NodeConnected { time, node } => {
                t!(format!("{:>9.3} - connected node: {}", time, node).green());
            }
            LogEntry::NodeDisconnected { time, node } => {
                t!(format!("{:>9.3} - disconnected node: {}", time, node).red());
            }
            LogEntry::NodeCrashed { time, node } => {
                t!(format!("{:>9.3} - node crashed: {}", time, node).red());
            }
            LogEntry::NodeRecovered { time, node } => {
                t!(format!("{:>9.3} - node recovered: {}", time, node).green());
            }
            LogEntry::TimerSet { .. } => {}
            LogEntry::TimerFired {
                time,
                timer_id: _,
                timer_name,
                node: _,
                proc,
            } => {
                t!(format!("{:>9.3} {:>10} !-- {:<10}", time, proc, timer_name).yellow());
            }
            LogEntry::TimerCancelled { .. } => {}
            LogEntry::LinkDisabled { time, from, to } => {
                t!(format!("{:>9.3} - disabled link: {:>10} --> {:<10}", time, from, to).red());
            }
            LogEntry::LinkEnabled { time, from, to } => {
                t!(format!("{:>9.3} - enabled link: {:>10} --> {:<10}", time, from, to).green());
            }
            LogEntry::DropIncoming { time, node } => {
                t!(format!("{:>9.3} - drop messages to {}", time, node).red());
            }
            LogEntry::PassIncoming { time, node } => {
                t!(format!("{:>9.3} - pass messages to {}", time, node).green());
            }
            LogEntry::DropOutgoing { time, node } => {
                t!(format!("{:>9.3} - drop messages from {}", time, node).red());
            }
            LogEntry::PassOutgoing { time, node } => {
                t!(format!("{:>9.3} - pass messages from {}", time, node).green());
            }
            LogEntry::NetworkPartition { time, group1, group2 } => {
                t!(format!("{:>9.3} - network partition: {:?} -x- {:?}", time, group1, group2).red());
            }
            LogEntry::NetworkReset { time } => {
                t!(format!("{:>9.3} - network reset, all problems healed", time).green());
            }
            LogEntry::NodeShutdown { time, node } => {
                t!(format!("{:>9.3} - node shutdown: {}", time, node).red());
            }
            LogEntry::NodeReran { time, node } => {
                t!(format!("{:>9.3} - node reran: {}", time, node).green());
            }
            LogEntry::ReadFromFile {
                time,
                node,
                request_id,
                file_name,
                bytes,
            } => {
                t!(format!(
                    "{:>9.3} {:>10} ({}) {} <-[{}]- {:<10}",
                    time, "", request_id, node, bytes, file_name
                )
                .blue());
            }
            LogEntry::WriteToFile {
                time,
                node,
                request_id,
                file_name,
                bytes,
            } => {
                t!(format!(
                    "{:>9.3} {:>10} ({}) {} -[{}]-> {:<10}",
                    time, "", request_id, node, bytes, file_name
                )
                .blue());
            }
            LogEntry::ReadRequestSucceed {
                time,
                node,
                request_id,
                file_name,
                bytes,
            } => t!(format!(
                "{:>9.3} {:>10} ({}) {} <-[{}]- {:<10} SUCCEED",
                time, "", request_id, node, bytes, file_name
            )
            .green()),
            LogEntry::ReadRequestFailed {
                time,
                node,
                request_id,
                file_name,
                reason,
                bytes,
            } => t!(format!(
                "{:>9.3} {:>10} ({}) {} <-[{}]- {:<10} !!! FAILED [{}]",
                time, "", request_id, node, bytes, file_name, reason
            )
            .red()),
            LogEntry::WriteRequestSucceed {
                time,
                node,
                request_id,
                file_name,
                bytes,
            } => t!(format!(
                "{:>9.3} {:>10} ({}) {} -[{}]-> {:<10} SUCCEED",
                time, "", request_id, node, bytes, file_name
            )
            .green()),
            LogEntry::WriteRequestFailed {
                time,
                node,
                request_id,
                file_name,
                reason,
                bytes,
            } => t!(format!(
                "{:>9.3} {:>10} ({}) {} -[{}]-> {:<10} !!! FAILED [{}]",
                time, "", request_id, node, bytes, file_name, reason
            )
            .red()),
        }
    }
}
