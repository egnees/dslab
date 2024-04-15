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

#[derive(Clone, Default)]
struct StorageTester {}

impl Process for StorageTester {
    fn on_message(&mut self, _msg: Message, _from: String, _ctx: Context) -> Result<(), String> {
        panic!("should not receive messages");
    }

    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        match msg.tip.as_str() {
            "read_all" => {
                let name = msg.data;
                ctx.clone().spawn(async move {
                    let content = ctx.read_all(&name).await.unwrap();
                    ctx.send_local(Message {
                        tip: "ok".into(),
                        data: String::from_utf8(content).unwrap(),
                    })
                });
            }
            "append" => {
                let name_data: Vec<&str> = msg.data.split(":").collect();
                let name = name_data[0].to_owned();
                let data = name_data[1].to_owned();
                ctx.clone().spawn(async move {
                    ctx.append(&name, data.as_bytes()).await.unwrap();
                    ctx.send_local(Message {
                        tip: "ok".into(),
                        data: String::new(),
                    })
                });
            }
            "create_file" => {
                let name = msg.data;
                ctx.clone().spawn(async move {
                    ctx.create_file(&name).await.unwrap();
                    ctx.send_local(Message {
                        tip: "ok".into(),
                        data: String::new(),
                    })
                });
            }
            _ => panic!("unexpected tip"),
        }

        Ok(())
    }

    fn on_timer(&mut self, _timer: String, _ctx: Context) -> Result<(), String> {
        panic!("no timers");
    }
}

#[test]
fn storage() {
    let mut sys = System::new(12345);

    sys.add_node_with_storage("node", 1024 * 1024);

    sys.add_process("p", Box::new(StorageTester::default()), "node");

    sys.send_local_message(
        "p",
        Message {
            tip: "create_file".into(),
            data: "file1".into(),
        },
    );

    let msg = sys.step_until_local_message("p").unwrap();
    assert_eq!(msg.len(), 1);
    assert_eq!(msg[0].tip, "ok");

    sys.send_local_message(
        "p",
        Message {
            tip: "append".into(),
            data: "file1:string1\n".into(),
        },
    );

    let msg = sys.step_until_local_message("p").unwrap();
    assert_eq!(msg.len(), 1);
    assert_eq!(msg[0].tip, "ok");

    sys.send_local_message(
        "p",
        Message {
            tip: "append".into(),
            data: "file1:string2\n".into(),
        },
    );

    let msg = sys.step_until_local_message("p").unwrap();
    assert_eq!(msg.len(), 1);
    assert_eq!(msg[0].tip, "ok");

    sys.send_local_message(
        "p",
        Message {
            tip: "read_all".into(),
            data: "file1".into(),
        },
    );

    let msg = sys.step_until_local_message("p").unwrap();
    assert_eq!(msg.len(), 1);
    assert_eq!(msg[0].tip, "ok");
    let data = &msg[0].data;
    assert_eq!(data, "string1\nstring2\n");

    assert!(sys.time() > 0.0);

    sys.crash_node("node");
    sys.step_until_no_events();

    sys.recover_node("node");
    sys.step_until_no_events();

    sys.add_process("p", Box::new(StorageTester::default()), "node");

    sys.send_local_message(
        "p",
        Message {
            tip: "read_all".into(),
            data: "file1".into(),
        },
    );

    let msg = sys.step_until_local_message("p").unwrap();
    assert_eq!(msg.len(), 1);
    assert_eq!(msg[0].tip, "ok");
    let data = &msg[0].data;
    assert_eq!(data, "string1\nstring2\n");
}
