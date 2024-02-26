use std::{cell::RefCell, rc::Rc};

use dslab_async_mp::{context::Context, message::Message, process::Process, system::System};
use env_logger::Builder;
use log::LevelFilter;

fn init_logger(level: LevelFilter) {
    Builder::new().filter(None, level).init();
}

#[derive(Clone, Default)]
struct SleepProcess {
    work_started_cnt: usize,
    work_ended_cnt: usize,
    shared: Rc<RefCell<usize>>,
}

impl SleepProcess {
    async fn work(&mut self, ctx: Context) {
        self.work_started_cnt += 1;

        ctx.send_local(Message {
            tip: "LAZY_WORK_BREAKPOINT".to_owned(),
            data: "1".to_owned(),
        });

        ctx.sleep(1.0).await;

        ctx.send_local(Message {
            tip: "LAZY_WORK_BREAKPOINT".to_owned(),
            data: "2".to_owned(),
        });

        ctx.sleep(2.0).await;

        ctx.send_local(Message {
            tip: "LAZY_WORK_BREAKPOINT".to_owned(),
            data: "3".to_owned(),
        });

        self.work_ended_cnt += 1;

        // Update shared.
        *self.shared.borrow_mut() += 1;
    }
}

impl Process for SleepProcess {
    fn on_message(&mut self, _msg: Message, _from: String, ctx: Context) -> Result<(), String> {
        ctx.set_timer("TIMER", 1.0);
        Ok(())
    }

    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        if msg.tip == "SEND_REQUEST" {
            let other_process = msg.data;

            let msg = Message {
                tip: "HELLO_TO".to_owned(),
                data: other_process.clone(),
            };

            ctx.send(msg, other_process);
        } else {
            let stat = Message {
                tip: "STAT".to_owned(),
                data: format!(
                    "work_started_cnt={}, work_ended_cnt={}",
                    self.work_started_cnt, self.work_ended_cnt
                ),
            };

            ctx.send_local(stat);
        }

        Ok(())
    }

    fn on_timer(&mut self, timer: String, ctx: Context) -> Result<(), String> {
        assert_eq!(timer, "TIMER");

        let ctx_clone = ctx.clone();
        ctx.spawn(self.work(ctx_clone));
        Ok(())
    }
}

fn main() {
    // Init system.
    init_logger(LevelFilter::Debug);

    let mut sys = System::new(12345);

    let first_proc = Box::new(SleepProcess::default());
    sys.add_node("first_node");
    sys.add_process("first_proc", first_proc, "first_node");

    let second_proc = Box::new(SleepProcess::default());
    sys.add_node("second_node");
    sys.add_process("second_proc", second_proc, "second_node");

    // Send local messages to start interaction.
    sys.send_local_message(
        "first_proc",
        Message {
            tip: "SEND_REQUEST".to_owned(),
            data: "second_proc".to_owned(),
        },
    );

    sys.send_local_message(
        "second_proc",
        Message {
            tip: "SEND_REQUEST".to_owned(),
            data: "first_proc".to_owned(),
        },
    );

    // Start interaction.
    sys.step_until_no_events();

    // Request stat from processes.
    sys.send_local_message(
        "first_proc",
        Message {
            tip: "STAT".to_owned(),
            data: "".to_owned(),
        },
    );

    sys.send_local_message(
        "second_proc",
        Message {
            tip: "STAT".to_owned(),
            data: "".to_owned(),
        },
    );

    // Handle local requests.
    sys.step_until_no_events();

    // Check there are local requests.
    let event_list = sys.read_local_messages("first_proc");
    assert!(!event_list.is_empty());

    // Check there are local requests.
    let event_list = sys.read_local_messages("second_proc");
    assert!(!event_list.is_empty());
}
