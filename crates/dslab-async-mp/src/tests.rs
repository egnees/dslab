use crate::{context::Context, message::Message, process::Process, system::System};

#[derive(Clone)]
struct ReliableSender {
    pair: String,
}

impl Process for ReliableSender {
    fn on_message(&mut self, msg: Message, _: String, ctx: Context) -> Result<(), String> {
        ctx.clone().send_local(msg);

        Ok(())
    }

    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let pair = self.pair.clone();
        ctx.clone().spawn(async move {
            _ = ctx.send_reliable(msg, pair).await;
        });

        Ok(())
    }

    fn on_timer(&mut self, _: String, _: Context) -> Result<(), String> {
        panic!("should be no timers")
    }
}

#[test]
fn reliable_works() {
    let mut system = System::new(12345);

    let sender1 = ReliableSender { pair: "s2".to_owned() };

    let sender2 = ReliableSender { pair: "s1".to_owned() };

    system.add_node("1");
    system.add_node("2");

    system.add_process("s1", Box::new(sender1), "1");
    system.add_process("s2", Box::new(sender2), "2");

    system.network().connect_node("1");
    system.network().connect_node("2");

    system.network().set_delays(0.5, 10.);
    system.network().set_corrupt_rate(0.5);
    system.network().set_drop_rate(0.9999);

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

    assert!(!system.read_local_messages("s1").is_empty());
    assert!(!system.read_local_messages("s2").is_empty());

    system.network().make_partition(&["s1"], &["s2"]);

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

    assert!(system.read_local_messages("s2").is_empty());
    assert!(system.read_local_messages("s2").is_empty());
}
