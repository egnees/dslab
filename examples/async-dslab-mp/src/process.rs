use std::{cell::RefCell, rc::Rc};

///! Implementation of process trait and client server chat infrastructure.
use dslab_core::Id;
use serde::{Deserialize, Serialize};

use crate::{context::VirtualContext, message::Message};

pub type ProcessContext = Rc<RefCell<VirtualContext>>;

/// Represents process trait, which must be implemented by user.
pub trait Process {
    fn on_start(&mut self, ctx: ProcessContext) -> Result<(), String>;

    fn on_message(&mut self, msg: &Message, from: Id, ctx: ProcessContext) -> Result<(), String>;

    fn on_local_message(&mut self, msg: &Message, ctx: ProcessContext) -> Result<(), String>;
}

/// Represents connect request.
#[derive(Serialize, Deserialize, Debug)]
struct ConnectRequest {
    pub process: String,
    pub time: f64,
}

impl ConnectRequest {
    pub const TIP: &'static str = "connect_request";
}

/// Represents client message.
#[derive(Serialize, Deserialize, Debug)]
pub struct ClientMessage {
    pub message: String,
    pub process: String,
    pub time: f64,
}

impl ClientMessage {
    pub const TIP: &'static str = "client_message";
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerMessage {
    pub message: String,
    pub time: f64,
}

impl ServerMessage {
    pub const TIP: &'static str = "server_message";
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientLocalMessage {
    pub info: String,
}

impl ClientLocalMessage {
    pub const TIP: &'static str = "client_local_message";
}

/// Implementation of client side.
#[derive(Clone)]
pub struct Client {
    client_name: String,
    server: Id,
}

impl Process for Client {
    fn on_start(&mut self, ctx: ProcessContext) -> Result<(), String> {
        let connect_request = ConnectRequest {
            process: self.client_name.clone(),
            time: ctx.borrow().get_time(),
        };

        let msg = Message::new(ConnectRequest::TIP, &connect_request).unwrap();

        let ctx_clone = ctx.clone();

        ctx.borrow_mut().spawn(async move {
            ctx_clone.borrow().send_msg_reliable(msg, self.server).await.unwrap();
        });

        Ok(())
    }

    fn on_message(&mut self, msg: &Message, from: Id, ctx: ProcessContext) -> Result<(), String> {
        assert_eq!(from, self.server);

        let content = if msg.get_tip() == ClientMessage::TIP {
            let msg_data = msg.get_data::<ClientMessage>().unwrap();
            format!("[{}] {}: {}", msg_data.time, msg_data.process, msg_data.message)
        } else {
            assert_eq!(msg.get_tip(), ServerMessage::TIP);

            let msg_data = msg.get_data::<ServerMessage>().unwrap();
            format!("[{}] SERVER: {}", msg_data.time, msg_data.message)
        };

        let local_msg = ClientLocalMessage { info: content };

        let msg = Message::new(ClientLocalMessage::TIP, &local_msg).unwrap();

        ctx.borrow_mut().send_local_msg(msg)?;

        Ok(())
    }

    fn on_local_message(&mut self, msg: &Message, ctx: ProcessContext) -> Result<(), String> {
        assert_eq!(msg.get_tip(), ClientLocalMessage::TIP);

        let local_msg_content = msg.get_data::<ClientLocalMessage>().unwrap().info;

        let client_net_msg = ClientMessage {
            message: local_msg_content,
            process: self.client_name.clone(),
            time: ctx.borrow().get_time(),
        };

        let msg = Message::new(ClientMessage::TIP, &client_net_msg).unwrap();

        let ctx_clone = ctx.clone();

        ctx.borrow().spawn(async move {
            ctx_clone
                .borrow_mut()
                .send_msg_reliable(msg, self.server)
                .await
                .unwrap();
        });

        Ok(())
    }
}

impl Client {
    pub fn new(name: String, server: Id) -> Self {
        Self {
            client_name: name,
            server,
        }
    }
}

#[derive(Default, Clone)]
pub struct Server {
    clients: Vec<Id>,
}

impl Process for Server {
    fn on_start(&mut self, _: ProcessContext) -> Result<(), String> {
        Ok(())
    }

    fn on_message(&mut self, msg: &Message, from: Id, ctx: ProcessContext) -> Result<(), String> {
        if msg.get_tip() == ConnectRequest::TIP {
            self.clients.push(from);

            let process_name = msg.get_data::<ConnectRequest>().unwrap().process;
            let time = msg.get_data::<ConnectRequest>().unwrap().time;

            let msg = Message::new(
                ServerMessage::TIP,
                &ServerMessage {
                    message: format!("{} joined to chat.", process_name),
                    time,
                },
            )
            .unwrap();

            for client in self.clients.iter() {
                let ctx_clone = ctx.clone();
                let msg_clone = msg.clone();
                let client_id = *client;
                ctx.borrow().spawn(async move {
                    ctx_clone
                        .borrow()
                        .send_msg_reliable(msg_clone, client_id)
                        .await
                        .unwrap();
                });
            }
        } else {
            assert_eq!(msg.get_tip(), ClientMessage::TIP);

            for client in self.clients.iter() {
                let ctx_clone = ctx.clone();
                let msg_clone = msg.clone();
                let client_id = *client;
                ctx.borrow().spawn(async move {
                    ctx_clone
                        .borrow()
                        .send_msg_reliable(msg_clone, client_id)
                        .await
                        .unwrap();
                });
            }
        }

        Ok(())
    }

    fn on_local_message(&mut self, _: &Message, _: ProcessContext) -> Result<(), String> {
        Err("No local messages in server.".to_owned())
    }
}
