use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dslab_core::{async_core::EventKey, Id, Simulation};
use dslab_network::{
    models::{ConstantBandwidthNetworkModel, SharedBandwidthNetworkModel},
    DataTransferCompleted, Network, NetworkModel,
};
use dslab_storage::{
    disk::DiskBuilder,
    events::{DataReadCompleted, DataWriteCompleted},
};

use crate::{context::VirtualContext, message::Message, node::Node, process::Process};

pub struct System {
    node_map: HashMap<Id, Rc<RefCell<Node>>>,
    sim: Simulation,
    network: Rc<RefCell<Network>>,
    started: bool,
}

impl System {
    const LOCAL_LATENCY: f64 = 0.0;
    const LOCAL_BANDWIDTH: i32 = 10000;
    const NETWORK_LATENCY: f64 = 0.5;
    const NETWORK_BANDWIDTH: i32 = 1000;
    const DISK_CAPACITY: i32 = 1000;
    const DISK_READ_BANDWIDTH: f64 = 2000.;
    const DISK_WRITE_BANDWIDTH: f64 = 2000.;

    pub fn new(seed: u64) -> Self {
        let mut sim = Simulation::new(seed);

        sim.register_key_getter_for::<DataTransferCompleted>(|e| e.dt.id as EventKey);
        sim.register_key_getter_for::<DataReadCompleted>(|e| e.request_id as EventKey);
        sim.register_key_getter_for::<DataWriteCompleted>(|e| e.request_id as EventKey);

        let network_model: Box<dyn NetworkModel> = Box::new(SharedBandwidthNetworkModel::new(
            Self::NETWORK_BANDWIDTH as f64,
            Self::NETWORK_LATENCY,
        ));

        let network_ctx = sim.create_context("net");
        let network = Rc::new(RefCell::new(Network::new(network_model, network_ctx)));
        sim.add_handler("net", network.clone());

        Self {
            node_map: HashMap::new(),
            sim,
            network,
            started: false,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.started {
            return Err("System already started".to_string());
        }

        self.started = true;

        for node in self.node_map.values() {
            node.borrow_mut().start();
        }

        Ok(())
    }

    pub fn add_process<P: Process + 'static>(&mut self, process: P) -> Id {
        let nodes_cnt = self.node_map.len();

        // Create node ctx.
        let node_name = format!("node-{}", nodes_cnt);
        let node_ctx = self.sim.create_context(&node_name);

        // Create disk, corresponding to node.
        let disk_name = format!("disk-{}", nodes_cnt);
        let disk_ctx = self.sim.create_context(&disk_name);
        let disk = Rc::new(RefCell::new(DiskBuilder::simple(1000, 1000.0, 1000.0).build(disk_ctx)));
        self.sim.add_handler(disk_name, disk.clone());

        // Create virtual context for node.
        let virtual_context = VirtualContext::new(node_ctx, disk, self.network.clone());

        // Create node.
        let boxed_process = Box::new(process);
        let node = Node::new(boxed_process, Rc::new(RefCell::new(virtual_context)));
        let node_ref = Rc::new(RefCell::new(node));

        // Add node in simulation.
        let handler_id = self.sim.add_handler(node_name, node_ref.clone());
        node_ref.borrow_mut().set_process_id(handler_id);

        // Add node to node map.
        self.node_map.insert(handler_id, node_ref);

        // Add node to the network.
        let host_name = format!("host-{}", nodes_cnt);
        self.network.borrow_mut().add_node(
            &host_name,
            Box::new(SharedBandwidthNetworkModel::new(
                Self::LOCAL_BANDWIDTH as f64,
                Self::LOCAL_LATENCY,
            )),
        );

        // Set location for the node in network.
        self.network.borrow_mut().set_location(handler_id, host_name.as_str());

        // Return handler id as process id.
        handler_id
    }

    pub fn step_until_no_events(&mut self) {
        self.sim.step_until_no_events()
    }

    pub fn step_for_duration(&mut self, duration: f64) -> bool {
        self.sim.step_for_duration(duration)
    }

    pub fn steps(&mut self, step_count: u64) -> bool {
        self.sim.steps(step_count)
    }

    pub fn step(&mut self) -> bool {
        self.sim.step()
    }

    pub fn read_local_msg(&mut self, process_id: Id) -> Result<Option<Message>, String> {
        if let Some(node) = self.node_map.get(&process_id) {
            Ok(node.borrow_mut().read_local_msg())
        } else {
            Err("No such process.".to_owned())
        }
    }

    pub fn send_local_msg(&mut self, process_id: Id, msg: &Message) -> Result<(), String> {
        if let Some(node) = self.node_map.get(&process_id) {
            node.borrow_mut().send_local_msg(msg);
            Ok(())
        } else {
            Err("No such process.".to_owned())
        }
    }

    pub fn read_local_messages(&mut self, process_id: Id) -> Result<Vec<Message>, String> {
        if let Some(node) = self.node_map.get(&process_id) {
            let mut messages = Vec::new();

            while let Some(msg) = node.borrow_mut().read_local_msg() {
                messages.push(msg);
            }

            Ok(messages)
        } else {
            Err("No such process.".to_owned())
        }
    }

    pub fn get_time(&self) -> f64 {
        self.sim.time()
    }
}
