use std::{cell::RefCell, rc::Rc};

use dslab_core::{async_core::AwaitResult, cast, Event, EventHandler, Id, Simulation, SimulationContext};
use futures::{select, FutureExt};

use crate::{
    log::{init::enable_console_log, logger::Logger},
    network::{
        event::{MessageDelivered, MessageDropped, TaggedMessageDelivered},
        register::register_network_key_getters,
    },
};

use super::{message::Message, model::Network, tag::Tag};

struct NodeStub {
    pub received_msg: u64,
    pub dropped_msg: u64,
    network_id: Id,
}

impl EventHandler for NodeStub {
    fn on(&mut self, event: Event) {
        cast!(match event.data {
            MessageDelivered {
                msg_id: _,
                msg: _,
                src_proc: _,
                src_node: _,
                dst_proc: _,
                dst_node: _,
            } => {
                // Ignore outdated delivery event from network.
                if event.src != self.network_id {
                    self.received_msg += 1;
                }
            }
            MessageDropped {
                msg_id: _,
                msg: _,
                src_proc: _,
                src_node: _,
                dst_proc: _,
                dst_node: _,
            } => {
                self.dropped_msg += 1;
            }
        })
    }
}

#[test]
fn network_works() {
    let logger = Rc::new(RefCell::new(Logger::default()));

    let mut sim = Simulation::new(12345);
    register_network_key_getters(&mut sim);

    let network_ctx = sim.create_context("network");

    let network = Rc::new(RefCell::new(Network::new(network_ctx.clone(), logger)));
    sim.add_handler("network", network.clone());

    let node1 = Rc::new(RefCell::new(NodeStub {
        received_msg: 0,
        dropped_msg: 0,
        network_id: network_ctx.id(),
    }));
    sim.add_handler("node1", node1.clone());
    let node1_ctx = sim.create_context("node1");

    let node2 = Rc::new(RefCell::new(NodeStub {
        received_msg: 0,
        dropped_msg: 0,
        network_id: network_ctx.id(),
    }));
    sim.add_handler("node2", node2.clone());
    let node2_ctx = sim.create_context("node2");

    network.borrow_mut().add_node("node1".to_owned(), node1_ctx.id());
    network.borrow_mut().add_node("node2".to_owned(), node2_ctx.id());

    network
        .borrow_mut()
        .set_proc_location("proc1".to_owned(), "node1".to_owned());
    network
        .borrow_mut()
        .set_proc_location("proc2".to_owned(), "node2".to_owned());

    network.borrow_mut().connect_node("node1");
    network.borrow_mut().connect_node("node2");

    network.borrow_mut().set_delays(0.5, 1.0);
    network.borrow_mut().set_drop_rate(0.0);
    network.borrow_mut().set_corrupt_rate(0.0);
    network.borrow_mut().set_dupl_rate(0.0);

    network
        .borrow_mut()
        .send_message(Message::new("tip", "msg"), "proc1", "proc2");

    network
        .borrow_mut()
        .send_message(Message::new("tip", "msg"), "proc2", "proc1");

    sim.step_until_no_events();

    assert_eq!(node1.borrow().received_msg, 1);
    assert_eq!(node1.borrow().dropped_msg, 0);

    assert_eq!(node2.borrow().received_msg, 1);
    assert_eq!(node2.borrow().dropped_msg, 0);

    // Send reliable with big enough timeout.
    network.borrow_mut().set_drop_rate(0.99);

    {
        let node1_ctx = node1_ctx.clone();
        let network = network.clone();
        node1_ctx.clone().spawn(async move {
            let event_key =
                network
                    .borrow_mut()
                    .send_message_with_ack(Message::new("tip", "data"), "proc1", "proc2", None);

            let send_result = node1_ctx
                .recv_event_by_key::<MessageDelivered>(event_key)
                .with_timeout(10.0)
                .await;

            match send_result {
                AwaitResult::Timeout(_) => panic!("got unexpected timeout"),
                AwaitResult::Ok(_) => {}
            }
        });
    }
    sim.step_until_no_events();
    assert_eq!(node1.borrow().received_msg, 1);
    assert_eq!(node2.borrow().received_msg, 2);

    // Send reliable with too small timeout.
    {
        let node1_ctx = node1_ctx.clone();
        let network = network.clone();
        node1_ctx.clone().spawn(async move {
            let event_key =
                network
                    .borrow_mut()
                    .send_message_with_ack(Message::new("tip", "data"), "proc1", "proc2", None);

            let send_result = node1_ctx
                .recv_event_by_key::<MessageDelivered>(event_key)
                .with_timeout(0.005)
                .await;

            match send_result {
                AwaitResult::Timeout(_) => {}
                AwaitResult::Ok(_) => panic!("too fast message send"),
            }
        });
    }
    sim.step_until_no_events();

    // Check message will be received latter.
    assert_eq!(node1.borrow().received_msg, 1);
    assert_eq!(node1.borrow().dropped_msg, 0);
    assert_eq!(node2.borrow().received_msg, 3);

    // Disconnect node and check send reliable will not deliver message and will send `message dropped` event.
    network.borrow_mut().disconnect_node("node1");

    // Send reliable from disconnected node.
    {
        let node1_ctx = node1_ctx.clone();
        let network = network.clone();
        node1_ctx.clone().spawn(async move {
            let event_key =
                network
                    .borrow_mut()
                    .send_message_with_ack(Message::new("tip", "data"), "proc1", "proc2", None);

            let send_result = node1_ctx
                .recv_event_by_key::<MessageDelivered>(event_key)
                .with_timeout(10.0)
                .await;

            match send_result {
                AwaitResult::Timeout(_) => {}
                AwaitResult::Ok(_) => panic!("message sent but node is disconnected"),
            }
        });
    }
    sim.step_until_no_events();

    assert_eq!(node1.borrow().received_msg, 1);
    assert_eq!(node1.borrow().dropped_msg, 1);
    assert_eq!(node2.borrow().received_msg, 3);
    assert_eq!(node2.borrow().dropped_msg, 0);

    // Send reliable from disconnected node to itself.
    {
        let node1_ctx = node1_ctx.clone();
        let network = network.clone();
        node1_ctx.clone().spawn(async move {
            let event_key =
                network
                    .borrow_mut()
                    .send_message_with_ack(Message::new("tip", "data"), "proc1", "proc1", None);

            let send_result = node1_ctx
                .recv_event_by_key::<MessageDelivered>(event_key)
                .with_timeout(0.005)
                .await;

            match send_result {
                AwaitResult::Timeout(_) => panic!("can not send message to itself from disconnected node"),
                AwaitResult::Ok(_) => {}
            }
        });
    }
    sim.step_until_no_events();

    assert_eq!(node1.borrow().received_msg, 2);
    assert_eq!(node1.borrow().dropped_msg, 1);
}

struct TaggedNodeStub {
    pub received_tagged_msg_handler: u64,
    pub received_msg_handler: u64,
    pub received_msg_async: Rc<RefCell<u64>>,
    pub dropped_msg: Rc<RefCell<u64>>,
    pub proc_name: String,
    net: Rc<RefCell<Network>>,
    ctx: SimulationContext,
}

impl EventHandler for TaggedNodeStub {
    fn on(&mut self, event: Event) {
        cast!(match event.data {
            TaggedMessageDelivered {
                msg_id: _,
                msg: _,
                src_proc: _,
                src_node: _,
                dst_proc: _,
                dst_node: _,
                tag: _,
            } => {
                self.received_tagged_msg_handler += 1;
            }
            MessageDelivered {
                msg_id: _,
                msg,
                src_proc,
                src_node: _,
                dst_proc,
                dst_node: _,
            } => {
                if event.src != self.net.borrow().id() {
                    self.received_msg_handler += 1;
                    let tag = u64::from_str_radix(&msg.data, 10).unwrap();
                    self.net
                        .borrow_mut()
                        .send_message_with_ack(msg, &self.proc_name, &src_proc, Some(tag));
                }
            }
            MessageDropped {
                msg_id: _,
                msg: _,
                src_proc: _,
                src_node: _,
                dst_proc: _,
                dst_node: _,
            } => {
                let mut drop = self.dropped_msg.borrow_mut();
                *drop = *drop + 1;
            }
        });
    }
}

impl TaggedNodeStub {
    fn new(name: String, ctx: SimulationContext, net: Rc<RefCell<Network>>) -> Self {
        Self {
            received_tagged_msg_handler: 0,
            received_msg_handler: 0,
            received_msg_async: Rc::new(RefCell::new(0)),
            dropped_msg: Rc::new(RefCell::new(0)),
            proc_name: name,
            net,
            ctx,
        }
    }

    fn spawn_send_recv_tag(&mut self, tag: Tag, to: String) {
        let msg = Message::new("tag", &tag.to_string());
        let net = self.net.clone();
        let ctx_clone = self.ctx.clone();
        let name = self.proc_name.clone();
        let cnt_recv = self.received_msg_async.clone();
        let cnt_dropped = self.dropped_msg.clone();
        self.ctx.spawn(async move {
            let event_id = net.borrow_mut().send_message_with_ack(msg, &name, &to, None);
            select! {
                e = ctx_clone.recv_event_by_key::<TaggedMessageDelivered>(tag).fuse() => {
                    assert_eq!(e.1.tag, tag);
                    let mut cnt_recv = cnt_recv.borrow_mut();
                    *cnt_recv = *cnt_recv + 1;
                }
                _ = ctx_clone.recv_event_by_key::<MessageDropped>(event_id).fuse() => {
                    let mut dropped = cnt_dropped.borrow_mut();
                    *dropped = *dropped + 1;
                }
            }
        });
    }
}

#[test]
fn send_recv_tag_works() {
    let logger = Rc::new(RefCell::new(Logger::default()));

    let mut sim = Simulation::new(12345);
    register_network_key_getters(&mut sim);

    let network_ctx = sim.create_context("network");

    let network = Rc::new(RefCell::new(Network::new(network_ctx.clone(), logger)));
    sim.add_handler("network", network.clone());

    let node1_ctx = sim.create_context("node1");
    let node1_id = node1_ctx.id();
    let node1 = Rc::new(RefCell::new(TaggedNodeStub::new(
        "proc1".to_owned(),
        node1_ctx,
        network.clone(),
    )));
    sim.add_handler("node1", node1.clone());

    let node2_ctx = sim.create_context("node2");
    let node2_id = node2_ctx.id();
    let node2 = Rc::new(RefCell::new(TaggedNodeStub::new(
        "proc2".to_owned(),
        node2_ctx,
        network.clone(),
    )));
    sim.add_handler("node2", node2.clone());

    network.borrow_mut().add_node("node1".to_owned(), node1_id);
    network.borrow_mut().add_node("node2".to_owned(), node2_id);

    network
        .borrow_mut()
        .set_proc_location("proc1".to_owned(), "node1".to_owned());
    network
        .borrow_mut()
        .set_proc_location("proc2".to_owned(), "node2".to_owned());

    network.borrow_mut().connect_node("node1");
    network.borrow_mut().connect_node("node2");

    network.borrow_mut().set_delays(0.5, 1.0);
    network.borrow_mut().set_drop_rate(0.0);
    network.borrow_mut().set_corrupt_rate(0.0);
    network.borrow_mut().set_dupl_rate(0.0);

    node1.borrow_mut().spawn_send_recv_tag(15, "proc2".to_owned());

    sim.step_until_no_events();

    assert_eq!(node2.borrow().received_msg_handler, 1);
    assert_eq!(*node2.borrow().received_msg_async.borrow(), 0);
    assert_eq!(node1.borrow().received_tagged_msg_handler, 0);
    assert_eq!(*node1.borrow().dropped_msg.borrow(), 0);
    assert_eq!(*node1.borrow().received_msg_async.borrow(), 1);
}
