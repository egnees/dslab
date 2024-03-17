use crate::{context::Context, message::Message, process::Process, system::System};

#[derive(Clone)]
struct ReliableSender {
    name: String,
    pair: String,
}

impl Process for ReliableSender {
    fn on_message(&mut self, msg: Message, from: String, ctx: Context) -> Result<(), String> {
        let pair = self.pair.clone();
        ctx.clone().spawn(async move {
            ctx.send_reliable(msg, pair).await.unwrap();
        });

        Ok(())
    }

    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let pair = self.pair.clone();
        ctx.clone().spawn(async move {
            ctx.send_reliable(msg, pair).await.unwrap();
        });

        Ok(())
    }

    fn on_timer(&mut self, _timer: String, _ctx: Context) -> Result<(), String> {
        todo!()
    }
}

#[test]
fn reliable_works() {
    let mut system = System::new(12345);

    let sender1 = ReliableSender {
        name: "s1".to_owned(),
        pair: "s2".to_owned(),
    };

    let sender2 = ReliableSender {
        name: "s2".to_owned(),
        pair: "s1".to_owned(),
    };

    system.add_node("1");
    system.add_node("2");

    system.add_process("s1", Box::new(sender1), "1");
    system.add_process("s2", Box::new(sender2), "2");

    system.network().connect_node("1");
    system.network().connect_node("2");

    system.network().set_corrupt_rate(0.8);
    system.network().set_delays(0.5, 10.);
    system.network().set_corrupt_rate(0.5);

    system.send_local_message(
        "s1",
        Message {
            tip: "M".to_owned(),
            data: "Hello".to_owned(),
        },
    );

    system.send_local_message(
        "s1",
        Message {
            tip: "M".to_owned(),
            data: "Echo".to_owned(),
        },
    );

    system.send_local_message(
        "s2",
        Message {
            tip: "M".to_owned(),
            data: "s2 hello".to_owned(),
        },
    );

    system.send_local_message(
        "s2",
        Message {
            tip: "M".to_owned(),
            data: "s2 helloooo".to_owned(),
        },
    );

    system.step_for_duration(50.);

    assert!(system.sent_message_count("s1") > 0);
    assert!(system.sent_message_count("s2") > 0);
}
