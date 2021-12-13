mod common;
mod compute;
mod master;
mod network;
mod task;
mod worker;

use rand::prelude::*;
use rand_pcg::Pcg64;
use sugars::{rc, refcell};

use crate::common::Start;
use crate::compute::Compute;
use crate::master::{Master, ReportStatus};
use crate::network::{Network, NETWORK_ID};
use crate::task::TaskRequest;
use crate::worker::Worker;
use core::actor::ActorId;
use core::sim::Simulation;

fn main() {
    // params
    let host_count = 5;
    let latency = 0.5;
    let bandwidth = 1000.;
    let task_count = 20;
    let seed = 123;

    let mut sim = Simulation::new(seed);
    let mut rand = Pcg64::seed_from_u64(seed);
    let admin = ActorId::from("admin");
    let client = ActorId::from("client");

    // create network and add hosts
    let net = rc!(refcell!(Network::new(latency, bandwidth)));
    sim.add_actor(NETWORK_ID, net.clone());
    for i in 0..host_count {
        net.borrow_mut().add_host(&format!("host{}", i));
    }
    let hosts = net.borrow().get_hosts();

    // create and start master on host0
    let host = &hosts[0];
    let master_id = format!("/{}/master", host);
    let master = sim.add_actor(&master_id, rc!(refcell!(Master::new(net.clone()))));
    net.borrow_mut().set_actor_host(master.clone(), host);
    sim.add_event_now(Start {}, admin.clone(), master.clone());

    // create and start workers
    for i in 0..host_count {
        let host = &hosts[i];
        let compute_id = format!("/{}/compute", host);
        let compute = rc!(refcell!(Compute::new(
            &compute_id,
            rand.gen_range(1..10),
            rand.gen_range(1..4)
        )));
        sim.add_actor(&compute_id, compute.clone());
        let worker = sim.add_actor(
            &format!("/{}/worker", host),
            rc!(refcell!(Worker::new(compute.clone(), net.clone(), master.clone()))),
        );
        net.borrow_mut().set_actor_host(worker.clone(), host);
        sim.add_event_now(Start {}, admin.clone(), worker.clone());
    }

    // let workers to register on master
    sim.step_until_no_events();

    // submit tasks
    for i in 0..task_count {
        let task = TaskRequest {
            id: i,
            comp_size: rand.gen_range(10..100),
            input_size: rand.gen_range(100..1000),
            output_size: rand.gen_range(10..100),
        };
        sim.add_event_now(task, client.clone(), master.clone());
    }

    // enable status reporting
    sim.add_event_now(ReportStatus {}, admin.clone(), master.clone());

    // run until completion
    sim.step_until_no_events();
}
