use crate::message::Message;

mod actions;
mod context;
mod events;
mod filesystem;
mod message;
mod node;
mod process;
mod system;

fn main() {
    // Create system.
    let mut system = system::System::new(12345);

    // Create server and add it to the system.
    let server = process::Server::default();
    let server_id = system.add_process(server);

    // Create client1 and add it to the system.
    let client1 = process::Client::new("Client1".to_owned(), server_id);
    let client1_id = system.add_process(client1);

    // Create client2 and add it to the system.
    let client2 = process::Client::new("Client2".to_owned(), server_id);
    let client2_id = system.add_process(client2);

    // Start system.
    system.start().unwrap();

    // Step until no events in the system.
    system.step_until_no_events();

    // Read local messages from both clients.
    println!("system time: {}", system.get_time());

    // From client1.
    let messages_1 = system.read_local_messages(client1_id).unwrap();
    assert_eq!(messages_1.is_empty(), false);

    // From client2.
    let messages_2 = system.read_local_messages(client2_id).unwrap();
    assert_eq!(messages_2.is_empty(), false);

    let msg = process::ClientLocalMessage {
        info: "Message from client1".to_owned(),
    };

    // Send message in the chat from client1.
    system
        .send_local_msg(
            client1_id,
            &Message::new(process::ClientLocalMessage::TIP, &msg).unwrap(),
        )
        .unwrap();

    // Step until no events in the system.
    system.step_until_no_events();

    // Read local messages from both clients.
    let messages_1 = system.read_local_messages(client1_id).unwrap();
    assert_eq!(messages_1.is_empty(), false);

    // system.add_process(...)
    // let recv = system.read_local_messages(client1_id);
    // system.register_local_msg_reader()
    // system.register_loacal_msg_writer(async move {
    // std::cin >> info;
    // tokio::spawn {
    // recv.read().await()
    //}

    // From client2.
    let messages_2 = system.read_local_messages(client2_id).unwrap();
    assert_eq!(messages_2.is_empty(), false);
}
