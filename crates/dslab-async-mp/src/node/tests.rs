use std::{cell::RefCell, ops::Add, rc::Rc};

use dslab_core::Simulation;
use dslab_storage::disk::DiskBuilder;

use crate::{
    log::{init::enable_console_log, logger::Logger},
    network::{message::Message, model::Network, register::register_network_key_getters},
    process::{context::Context, process::Process},
    storage::register::register_storage_key_getters,
};

use super::component::Node;

struct ProcessStub {
    pub received_msg_cnt: Rc<RefCell<u64>>,
}

impl Process for ProcessStub {
    fn on_message(&mut self, _msg: Message, _from: String, _ctx: Context) -> Result<(), String> {
        let mut x = *self.received_msg_cnt.borrow_mut();
        x += 1;
        *self.received_msg_cnt.borrow_mut() = x;
        Ok(())
    }

    fn on_local_message(&mut self, _msg: Message, _ctx: Context) -> Result<(), String> {
        unimplemented!()
    }

    fn on_timer(&mut self, _timer: String, _ctx: Context) -> Result<(), String> {
        unimplemented!()
    }
}

#[test]
fn node_works_with_crash() {
    let mut sim = Simulation::new(12345);
    register_network_key_getters(&mut sim);
    register_storage_key_getters(&mut sim);

    let logger = Rc::new(RefCell::new(Logger::default()));

    let net_ctx = sim.create_context("net");
    let net = Rc::new(RefCell::new(Network::new(net_ctx, logger.clone())));

    let disk1_ctx = sim.create_context("disk1");
    let disk1 = DiskBuilder::simple(1024, 10.0, 20.0).build(disk1_ctx);
    let disk1 = Rc::new(RefCell::new(disk1));
    sim.add_handler("disk1", disk1.clone());
    let node1_ctx = sim.create_context("node1");
    let node1 = Node::new(
        "node1".to_owned(),
        node1_ctx.clone(),
        net.clone(),
        logger.clone(),
        disk1,
    );
    let node1 = Rc::new(RefCell::new(node1));
    let proc1_received = Rc::new(RefCell::new(0));
    node1.borrow_mut().add_process(
        "proc1",
        Box::new(ProcessStub {
            received_msg_cnt: proc1_received.clone(),
        }),
    );
    sim.add_handler("node1", node1.clone());

    let disk2_ctx = sim.create_context("disk2");
    let disk2 = DiskBuilder::simple(1024, 10.0, 20.0).build(disk2_ctx);
    let node2_ctx = sim.create_context("node2");
    let node2 = Node::new(
        "node2".to_owned(),
        node2_ctx.clone(),
        net.clone(),
        logger.clone(),
        Rc::new(RefCell::new(disk2)),
    );
    let node2 = Rc::new(RefCell::new(node2));
    let proc2_received = Rc::new(RefCell::new(0));
    node2.borrow_mut().add_process(
        "proc2",
        Box::new(ProcessStub {
            received_msg_cnt: proc2_received.clone(),
        }),
    );
    sim.add_handler("node2", node2.clone());

    net.borrow_mut().add_node("node1".to_owned(), node1.borrow().id);
    net.borrow_mut().add_node("node2".to_owned(), node2.borrow().id);

    net.borrow_mut()
        .set_proc_location("proc1".to_owned(), "node1".to_owned());
    net.borrow_mut()
        .set_proc_location("proc2".to_owned(), "node2".to_owned());

    net.borrow_mut().set_delays(0.5, 1.0);
    net.borrow_mut().set_corrupt_rate(0.0);

    net.borrow_mut()
        .send_message(Message::new("tip", "data"), "proc1", "proc2");

    sim.step_until_no_events();

    assert_eq!(*proc1_received.borrow(), 0);
    assert_eq!(*proc2_received.borrow(), 1);

    net.borrow_mut()
        .send_message(Message::new("tip", "data"), "proc1", "proc2");

    node2.borrow_mut().crash();

    sim.step_until_no_events();

    assert_eq!(*proc1_received.borrow(), 0);
    assert_eq!(*proc2_received.borrow(), 1);

    net.borrow_mut()
        .send_message(Message::new("tip", "data"), "proc1", "proc2");

    sim.step_until_no_events();

    assert_eq!(*proc1_received.borrow(), 0);
    assert_eq!(*proc2_received.borrow(), 1);

    node2.borrow_mut().recover();

    node2.borrow_mut().add_process(
        "proc2",
        Box::new(ProcessStub {
            received_msg_cnt: proc2_received.clone(),
        }),
    );

    node1.borrow_mut().add_process(
        "proc2",
        Box::new(ProcessStub {
            received_msg_cnt: proc2_received.clone(),
        }),
    );

    net.borrow_mut()
        .send_message(Message::new("tip", "data"), "proc1", "proc2");

    sim.step_until_no_events();

    assert_eq!(*proc1_received.borrow(), 0);
    assert_eq!(*proc2_received.borrow(), 2);

    node2.borrow_mut().shutdown();

    net.borrow_mut()
        .send_message(Message::new("tip", "data"), "proc1", "proc2");

    sim.step_until_no_events();

    assert_eq!(*proc1_received.borrow(), 0);
    assert_eq!(*proc2_received.borrow(), 2);
}
