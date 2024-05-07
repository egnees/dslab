//! Network model implementation.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use dslab_core::async_core::EventKey;
use lazy_static::lazy_static;
use regex::Regex;

use dslab_core::{Event, SimulationContext};
use dslab_core::{EventHandler, Id};

use crate::log::log_entry::LogEntry;
use crate::log::logger::Logger;

use super::event::{MessageDelivered, MessageDropped, TaggedMessageDelivered};
use super::message::Message;
use super::tag::Tag;

/// Represents a network that transmits messages between processes located on different nodes.
pub struct Network {
    min_delay: f64,
    max_delay: f64,
    drop_rate: f64,
    dupl_rate: f64,
    corrupt_rate: f64,
    node_ids: HashMap<String, Id>,
    proc_locations: HashMap<String, String>,
    drop_incoming: HashSet<String>,
    drop_outgoing: HashSet<String>,
    disabled_links: HashSet<(String, String)>,
    network_message_count: u64,
    traffic: u64,
    ctx: SimulationContext,
    logger: Rc<RefCell<Logger>>,
    next_event_id: u64,
}

impl Network {
    pub(crate) fn new(ctx: SimulationContext, logger: Rc<RefCell<Logger>>) -> Self {
        Self {
            min_delay: 1.,
            max_delay: 1.,
            drop_rate: 0.,
            dupl_rate: 0.,
            corrupt_rate: 0.,
            node_ids: HashMap::new(),
            proc_locations: HashMap::new(),
            drop_incoming: HashSet::new(),
            drop_outgoing: HashSet::new(),
            disabled_links: HashSet::new(),
            network_message_count: 0,
            traffic: 0,
            ctx,
            logger,
            next_event_id: 0,
        }
    }

    /// Returns id of the network component in the simulation.
    pub fn id(&self) -> Id {
        self.ctx.id()
    }

    /// Adds a new node to the network.
    pub fn add_node(&mut self, name: String, id: Id) {
        self.node_ids.insert(name, id);
    }

    /// Returns a map with process locations (process -> node).
    pub fn proc_locations(&self) -> &HashMap<String, String> {
        &self.proc_locations
    }

    /// Sets the process location.
    pub fn set_proc_location(&mut self, proc: String, node: String) {
        self.proc_locations.insert(proc, node);
    }

    /// Returns the maximum network delay.
    pub fn max_delay(&self) -> f64 {
        self.max_delay
    }

    /// Sets the fixed network delay.
    pub fn set_delay(&mut self, delay: f64) {
        self.min_delay = delay;
        self.max_delay = delay;
    }

    /// Sets the minimum and maximum network delays.
    pub fn set_delays(&mut self, min_delay: f64, max_delay: f64) {
        self.min_delay = min_delay;
        self.max_delay = max_delay;
    }

    /// Returns the message drop probability.
    pub fn drop_rate(&self) -> f64 {
        self.drop_rate
    }

    /// Sets the message drop probability.
    pub fn set_drop_rate(&mut self, drop_rate: f64) {
        self.drop_rate = drop_rate;
    }

    /// Returns the message duplication probability.
    pub fn dupl_rate(&self) -> f64 {
        self.dupl_rate
    }

    /// Sets the message duplication probability.
    pub fn set_dupl_rate(&mut self, dupl_rate: f64) {
        self.dupl_rate = dupl_rate;
    }

    /// Returns the message corruption probability.
    pub fn corrupt_rate(&self) -> f64 {
        self.corrupt_rate
    }

    /// Sets the message corruption probability.
    pub fn set_corrupt_rate(&mut self, corrupt_rate: f64) {
        self.corrupt_rate = corrupt_rate;
    }

    /// Returns nodes with enabled dropping of incoming messages.
    pub fn get_drop_incoming(&self) -> &HashSet<String> {
        &self.drop_incoming
    }

    /// Enables dropping of incoming messages for a node.
    pub fn drop_incoming(&mut self, node: &str) {
        self.drop_incoming.insert(node.to_string());

        self.logger.borrow_mut().log(LogEntry::DropIncoming {
            time: self.ctx.time(),
            node: node.to_string(),
        });
    }

    /// Disables dropping of incoming messages for a node.
    pub fn pass_incoming(&mut self, node: &str) {
        self.drop_incoming.remove(node);

        self.logger.borrow_mut().log(LogEntry::PassIncoming {
            time: self.ctx.time(),
            node: node.to_string(),
        });
    }

    /// Returns nodes with enabled dropping of outgoing messages.
    pub fn get_drop_outgoing(&self) -> &HashSet<String> {
        &self.drop_outgoing
    }

    /// Enables dropping of outgoing messages for a node.
    pub fn drop_outgoing(&mut self, node: &str) {
        self.drop_outgoing.insert(node.to_string());

        self.logger.borrow_mut().log(LogEntry::DropOutgoing {
            time: self.ctx.time(),
            node: node.to_string(),
        });
    }

    /// Disables dropping of outgoing messages for a node.
    pub fn pass_outgoing(&mut self, node: &str) {
        self.drop_outgoing.remove(node);

        self.logger.borrow_mut().log(LogEntry::PassOutgoing {
            time: self.ctx.time(),
            node: node.to_string(),
        });
    }

    /// Disconnects a node from the network.
    ///
    /// Equivalent to enabling dropping of both incoming and outgoing messages for a node.
    pub fn disconnect_node(&mut self, node: &str) {
        self.drop_incoming.insert(node.to_string());
        self.drop_outgoing.insert(node.to_string());

        self.logger.borrow_mut().log(LogEntry::NodeDisconnected {
            time: self.ctx.time(),
            node: node.to_string(),
        });
    }

    /// Connects a node to a network.
    ///
    /// Equivalent to disabling dropping of both incoming and outgoing messages for a node.
    pub fn connect_node(&mut self, node: &str) {
        self.drop_incoming.remove(node);
        self.drop_outgoing.remove(node);

        self.logger.borrow_mut().log(LogEntry::NodeConnected {
            time: self.ctx.time(),
            node: node.to_string(),
        });
    }

    /// Returns disabled links.
    pub fn disabled_links(&self) -> &HashSet<(String, String)> {
        &self.disabled_links
    }

    /// Disables link between nodes `from` and `to` by dropping all messages sent in this direction.
    pub fn disable_link(&mut self, from: &str, to: &str) {
        self.disabled_links.insert((from.to_string(), to.to_string()));

        self.logger.borrow_mut().log(LogEntry::LinkDisabled {
            time: self.ctx.time(),
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    /// Enables link between nodes `from` and `to`.
    pub fn enable_link(&mut self, from: &str, to: &str) {
        self.disabled_links.remove(&(from.to_string(), to.to_string()));

        self.logger.borrow_mut().log(LogEntry::LinkEnabled {
            time: self.ctx.time(),
            from: from.to_string(),
            to: to.to_string(),
        });
    }

    /// Creates a network partition between two groups of nodes.
    pub fn make_partition(&mut self, group1: &[&str], group2: &[&str]) {
        for n1 in group1 {
            for n2 in group2 {
                self.disabled_links.insert((n1.to_string(), n2.to_string()));
                self.disabled_links.insert((n2.to_string(), n1.to_string()));
            }
        }

        self.logger.borrow_mut().log(LogEntry::NetworkPartition {
            time: self.ctx.time(),
            group1: group1.iter().map(|&node| node.to_string()).collect(),
            group2: group2.iter().map(|&node| node.to_string()).collect(),
        });
    }

    /// Resets the network links by enabling all links
    /// and disabling dropping of incoming/outgoing messages for all nodes.
    ///
    /// Note that this does not affect the `drop_rate` setting.
    pub fn reset(&mut self) {
        self.disabled_links.clear();
        self.drop_incoming.clear();
        self.drop_outgoing.clear();

        self.logger
            .borrow_mut()
            .log(LogEntry::NetworkReset { time: self.ctx.time() });
    }

    /// Returns the total number of messages sent via the network.
    pub fn network_message_count(&self) -> u64 {
        self.network_message_count
    }

    /// Returns the total size of messages sent via the network.
    pub fn traffic(&self) -> u64 {
        self.traffic
    }

    fn message_is_dropped(&self, src: &String, dst: &String) -> bool {
        self.ctx.rand() < self.drop_rate
            || self.drop_outgoing.contains(src)
            || self.drop_incoming.contains(dst)
            || self.disabled_links.contains(&(src.clone(), dst.clone()))
    }

    fn corrupt_if_needed(&self, msg: Message) -> Message {
        if self.ctx.rand() < self.corrupt_rate {
            lazy_static! {
                static ref RE: Regex = Regex::new(r#""[^"]+""#).unwrap();
            }
            let corrupted_data = RE.replace_all(&msg.data, "\"\"").to_string();
            Message::new(msg.tip, corrupted_data)
        } else {
            msg
        }
    }

    fn get_message_count(&self) -> u32 {
        if self.ctx.rand() >= self.dupl_rate {
            1
        } else {
            (self.ctx.rand() * 2.).ceil() as u32 + 1
        }
    }

    fn next_msg_event(&mut self, src_proc: String, dst_proc: String, msg: Message) -> MessageDelivered {
        let src_node = self.proc_locations.get(&src_proc).unwrap().clone();
        let dst_node = self.proc_locations.get(&dst_proc).unwrap().clone();
        let event_id = self.next_event_id;

        self.next_event_id += 1;

        MessageDelivered {
            msg_id: event_id,
            msg,
            src_proc,
            src_node,
            dst_proc,
            dst_node,
        }
    }

    /// Sends a message between two processes.
    pub(crate) fn send_message(&mut self, msg: Message, src_proc: &str, dst_proc: &str) {
        let msg_size = msg.size();
        let mut potential_event = self.next_msg_event(src_proc.to_owned(), dst_proc.to_owned(), msg);
        let src_node_id = *self.node_ids.get(&potential_event.src_node).unwrap();
        let dst_node_id = *self.node_ids.get(&potential_event.dst_node).unwrap();

        self.log_message_sent(&potential_event);

        // local communication inside a node is reliable and fast
        if potential_event.src_node == potential_event.dst_node {
            self.ctx.emit_as(potential_event, src_node_id, dst_node_id, 0.);
            // communication between different nodes can be faulty
        } else {
            if !self.message_is_dropped(&potential_event.src_node, &potential_event.dst_node) {
                potential_event.msg = self.corrupt_if_needed(potential_event.msg);
                let msg_count = self.get_message_count();
                if msg_count == 1 {
                    let delay = self.min_delay + self.ctx.rand() * (self.max_delay - self.min_delay);
                    self.ctx.emit_as(potential_event, src_node_id, dst_node_id, delay);
                } else {
                    for _ in 0..msg_count {
                        let delay = self.min_delay + self.ctx.rand() * (self.max_delay - self.min_delay);
                        self.ctx
                            .emit_as(potential_event.clone(), src_node_id, dst_node_id, delay);
                    }
                }
            } else {
                self.log_message_dropped(&potential_event.into());
            }

            self.network_message_count += 1;
            self.traffic += msg_size as u64;
        }
    }

    /// Reliable send message between two processes.
    /// It is guaranteed that message will be delivered exactly once and will not be corrupted.
    /// If two processes are not connected by the network, then error will be returned.
    pub(crate) fn send_message_with_ack(&mut self, msg: Message, src: &str, dst: &str, tag: Option<Tag>) -> EventKey {
        let msg_size = msg.size();
        let potential_event = self.next_msg_event(src.to_owned(), dst.to_owned(), msg);
        let event_key = potential_event.msg_id as EventKey;
        let src_node_id = *self.node_ids.get(&potential_event.src_node).unwrap();
        let dst_node_id = *self.node_ids.get(&potential_event.dst_node).unwrap();

        self.log_message_sent(&potential_event);

        let msg_dropped = src_node_id != dst_node_id
            && (self.drop_outgoing.contains(&potential_event.src_node)
                || self.drop_incoming.contains(&potential_event.dst_node)
                || self
                    .disabled_links
                    .contains(&(potential_event.src_node.clone(), potential_event.dst_node.clone())));

        if msg_dropped {
            let delay = self.min_delay + self.ctx.rand() * (self.max_delay - self.min_delay);
            let event: MessageDropped = potential_event.into();

            self.log_message_dropped(&event);

            self.ctx.emit_as(event, self.ctx.id(), src_node_id, delay);
        } else {
            let msg_delay = if src_node_id == dst_node_id {
                0.
            } else {
                self.min_delay + 2.0 * self.ctx.rand() * (self.max_delay - self.min_delay)
            };

            self.ctx
                .emit_as(potential_event.clone(), self.ctx.id(), src_node_id, msg_delay);

            if let Some(tag) = tag {
                let event = TaggedMessageDelivered {
                    msg_id: potential_event.msg_id,
                    msg: potential_event.msg,
                    src_proc: potential_event.src_proc,
                    src_node: potential_event.src_node,
                    dst_proc: potential_event.dst_proc,
                    dst_node: potential_event.dst_node,
                    tag,
                };
                self.ctx.emit_as(event, src_node_id, dst_node_id, msg_delay);
            } else {
                self.ctx.emit_as(potential_event, src_node_id, dst_node_id, msg_delay);
            }
        }

        self.network_message_count += 1;
        self.traffic += msg_size as u64;

        event_key
    }

    fn log_message_sent(&self, potential_event: &MessageDelivered) {
        self.logger.borrow_mut().log(LogEntry::MessageSent {
            time: self.ctx.time(),
            msg_id: potential_event.msg_id.to_string(),
            src_node: potential_event.src_node.clone(),
            src_proc: potential_event.src_proc.clone(),
            dst_node: potential_event.dst_node.clone(),
            dst_proc: potential_event.dst_proc.clone(),
            msg: potential_event.msg.clone(),
        });
    }

    fn log_message_dropped(&self, potential_event: &MessageDropped) {
        self.logger.borrow_mut().log(LogEntry::MessageDropped {
            time: self.ctx.time(),
            msg_id: potential_event.msg_id.to_string(),
            src_node: potential_event.src_node.clone(),
            src_proc: potential_event.src_proc.clone(),
            dst_node: potential_event.dst_node.clone(),
            dst_proc: potential_event.dst_proc.clone(),
            msg: potential_event.msg.clone(),
        })
    }
}

impl EventHandler for Network {
    fn on(&mut self, _event: Event) {
        // do nothing
    }
}
