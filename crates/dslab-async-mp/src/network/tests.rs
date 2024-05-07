use std::{cell::RefCell, rc::Rc};

use dslab_core::{async_core::AwaitResult, cast, Event, EventHandler, Id, Simulation};

use crate::{
    log::logger::Logger,
    network::{
        event::{MessageDelivered, MessageDropped},
        register::register_network_key_getters,
    },
};

use super::{message::Message, model::Network};

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
