use std::collections::{HashMap, HashSet};

use artplace::messages::*;



use actix::prelude::*;
use rand::random;


pub type Room = String;
pub type Uid = String;

#[derive(Message)]
#[rtype(InitClient)]
pub struct Connect {
    pub id: Uid,
    pub addr: Recipient<ClientMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uid,
    pub room: Room,
}

#[derive(Message)]
#[rtype(InitClient)]
pub struct Join {
    pub id: Uid,
    pub old: Room,
    pub new: Room,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OverlayClientMessage {
    pub room: Room,
    pub msg: ClientMessage,
}

pub struct OverlayServer {
    sessions: HashMap<Uid, Recipient<ClientMessage>>,
    rooms: HashMap<Room, HashSet<Uid>>,
    logs: HashMap<Room, Vec<ClientMessage>>,
}

impl OverlayServer {
    pub fn new() -> OverlayServer {
        OverlayServer {
            sessions: HashMap::new(),
            rooms: HashMap::new(),
            logs: HashMap::new(),
        }
    }

    fn send_message(&self, room: &str ,msg: &ClientMessage) {
        if let Some(clients) = self.rooms.get(room) {
            for id in clients {
                if let Some(addr) = self.sessions.get(id) {
                    addr.do_send(msg.clone());
                }
            }
        }
    }
}

impl Actor for OverlayServer {
    type Context = Context<Self>;
}


impl Handler<Connect> for OverlayServer {
    type Result = InitClient;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.id.clone(), msg.addr);
        let room = (random::<u8>() % 5 + 1).to_string();
        self.rooms.entry(room.clone()).or_default().insert(msg.id.clone());
        InitClient {
            id: msg.id,
            room: room.clone(),
            messages: self.logs.entry(room).or_default().clone(),
        }
    }
}

impl Handler<Disconnect> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
        self.rooms.entry(msg.room).or_default().remove(&msg.id);
    }
}

impl Handler<Join> for OverlayServer {
    type Result = InitClient;

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) -> Self::Result {
        self.rooms.entry(msg.old).or_default().remove(&msg.id);
        self.rooms.entry(msg.new.clone()).or_default().insert(msg.id.clone());
        InitClient {
            id: msg.id,
            room: msg.new.clone(),
            messages: self.logs.entry(msg.new).or_default().clone(),
        }
    }
}

impl Handler<OverlayClientMessage> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: OverlayClientMessage, _: &mut Context<Self>) {
        self.send_message(&msg.room, &msg.msg);
        self.logs.entry(msg.room).or_default().push(msg.msg);
    }
}
