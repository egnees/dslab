use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use dslab_compute::multicore::{CompFailed, CompFinished, CompStarted, Compute};
use dslab_core::async_core::shared_state::DetailsKey;
use dslab_core::{async_core::task::Task, cast, event::EventId, log_debug, Event, EventHandler, Id, SimulationContext};
use log::debug;

use futures::future::{self, FutureExt};
use futures::select;

use serde::Serialize;
use serde_json::json;

use sugars::{rc, refcell};

use crate::events::{Start, TakeTask, TaskCompleted, TaskRequest};

#[derive(Serialize)]
struct TaskInfo {
    flops: u64,
    memory: u64,
    cores: u32,
}

pub struct Worker {
    id: Id,
    compute: Rc<RefCell<Compute>>,
    compute_id: Id,
    ctx: SimulationContext,
    tasks_queue: RefCell<VecDeque<TaskInfo>>,
}

impl Worker {
    pub fn new(compute: Rc<RefCell<Compute>>, compute_id: Id, ctx: SimulationContext) -> Self {
        Self {
            id: ctx.id(),
            compute,
            compute_id,
            ctx,
            tasks_queue: refcell!(VecDeque::new()),
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    fn on_start(&self) {
        log_debug!(self.ctx, "Worker started");
        self.ctx.spawn(self.work_loop());
    }

    fn on_task_request(&self, task_info: TaskInfo) {
        if self.tasks_queue.borrow().is_empty() {
            self.ctx.emit_self_now(TakeTask {});
        }

        log_debug!(self.ctx, format!("Received task: {}", json!(&task_info)));

        self.tasks_queue.borrow_mut().push_back(task_info);
    }

    async fn work_loop(&self) {
        let mut tasks_completed = 0;
        loop {
            if self.tasks_queue.borrow().is_empty() {
                self.ctx.async_handle_self::<TakeTask>().await;
            }

            let task_info = self.tasks_queue.borrow_mut().pop_front().unwrap();

            while !self.try_start_process_task(&task_info).await {
                self.ctx.async_handle_self::<TaskCompleted>().await;
            }

            tasks_completed += 1;

            log_debug!(self.ctx, format!("work_loop : {} tasks completed", tasks_completed));
        }
    }

    async fn try_start_process_task(&self, task_info: &TaskInfo) -> bool {
        let key = self.run_task(task_info);

        select! {
            _ = self.ctx.async_detailed_handle_event::<CompStarted>(self.compute_id, key).fuse() => {

                log_debug!(self.ctx, format!("try_process_task : task with key {} started", key));

                self.ctx.spawn(self.process_task(key));

                true
            },
            (_, failed) = self.ctx.async_detailed_handle_event::<CompFailed>(self.compute_id, key).fuse() => {
                log_debug!(self.ctx, format!("try_process_task : task with key {} failed: {}", key, json!(failed)));
                false
            }
        }
    }

    async fn process_task(&self, key: DetailsKey) {
        self.ctx
            .async_detailed_handle_event::<CompFinished>(self.compute_id, key)
            .await;

        log_debug!(self.ctx, format!("process_task : task with key {} completed", key));

        self.ctx.emit_self_now(TaskCompleted {});
    }

    fn run_task(&self, task_info: &TaskInfo) -> DetailsKey {
        self.compute.borrow_mut().run(
            task_info.flops,
            task_info.memory,
            task_info.cores,
            task_info.cores,
            dslab_compute::multicore::CoresDependency::Linear,
            self.id(),
        ) as DetailsKey
    }
}

impl EventHandler for Worker {
    fn on(&mut self, event: Event) {
        cast!(match event.data {
            TaskRequest { flops, cores, memory } => {
                self.on_task_request(TaskInfo { flops, cores, memory });
            }
            Start {} => {
                self.on_start();
            }
            TakeTask {} => {}
            TaskCompleted {} => {}
        })
    }
}