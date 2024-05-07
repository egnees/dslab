use dslab_async_mp::{
    network::message::Message,
    process::{context::Context, process::Process},
    system::System,
};

struct SendRecvProc {
    to_proc: String,
}

impl Process for SendRecvProc {
    fn on_message(&mut self, msg: Message, from: String, ctx: Context) -> Result<(), String> {
        let to = from;
        let tag = u64::from_str_radix(&msg.data, 10).unwrap();
        ctx.clone().spawn(async move {
            ctx.send_with_tag(msg, tag, &to, 10.0).await.unwrap();
        });
        Ok(())
    }

    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let ctx_clone = ctx.clone();
        let to = self.to_proc.clone();
        let tag = u64::from_str_radix(&msg.data, 10).unwrap();
        ctx.spawn(async move {
            let msg = ctx_clone.send_recv_with_tag(msg, tag, &to, 10.0).await.unwrap();
            assert_eq!(u64::from_str_radix(&msg.data, 10).unwrap(), tag);
            ctx_clone.send_local(msg);
        });
        Ok(())
    }

    fn on_timer(&mut self, _timer: String, _ctx: Context) -> Result<(), String> {
        unreachable!()
    }
}

#[test]
fn test_send_recv_with_tag() {
    let mut sys = System::new(1111);
    sys.add_node("node1");
    sys.add_node("node2");
    sys.add_process(
        "proc1",
        Box::new(SendRecvProc {
            to_proc: "proc2".to_owned(),
        }),
        "node1",
    );
    sys.add_process(
        "proc2",
        Box::new(SendRecvProc {
            to_proc: "proc1".to_owned(),
        }),
        "node2",
    );
    sys.network().connect_node("node1");
    sys.network().connect_node("node2");
    sys.network().set_corrupt_rate(0.0);
    sys.network().set_delays(0.5, 1.5);

    sys.send_local_message("proc1", Message::new("tagged_msg", "1235"));

    sys.step_until_no_events();

    let msgs = sys.read_local_messages("proc1").unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], Message::new("tagged_msg", "1235"));
}
