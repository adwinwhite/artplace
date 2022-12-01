use std::collections::{HashMap, HashSet};

use artplace::messages::*;
use artplace::messages::client_message::MessageKind;



use actix::prelude::*;
use rand::random;
use uuid::Uuid;


pub type RoomId = String;
pub type Uid = String;

#[derive(Message)]
#[rtype(Uid)]
pub struct Connect {
    pub id: Option<Uid>,
    pub addr: Recipient<ClientMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uid,
    pub room_id: Option<RoomId>,
}

#[derive(Message)]
#[rtype(InitClient)]
pub struct Join {
    pub id: Uid,
    pub old: Option<RoomId>,
    pub new: Option<RoomId>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OverlayClientMessage {
    pub room: RoomId,
    pub msg: ClientMessage,
}

#[derive(Default)]
struct RoomState {
    users: HashSet<Uid>,
    owner: Option<Uid>,
    snapshot: Option<Snapshot>,
    log: Vec<ClientMessage>
}

pub struct OverlayServer {
    sessions: HashMap<Uid, Recipient<ClientMessage>>,
    rooms: HashMap<RoomId, RoomState>
}

impl OverlayServer {
    pub fn new() -> OverlayServer {
        OverlayServer {
            sessions: HashMap::new(),
            rooms: HashMap::new(),
        }
    }

    fn send_message(&self, room_id: &str, msg: &ClientMessage) {
        if let Some(room) = self.rooms.get(room_id) {
            for id in &room.users {
                if let Some(addr) = self.sessions.get(id) {
                    addr.do_send(msg.clone());
                }
            }
        }
    }

    fn leave_room(&mut self, room_id: &str, id: &str) {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        room.users.remove(id);
        // emptry room owner if it's me.
        if let Some(owner) = &mut room.owner {
            if owner == id {
                room.owner = None;
            }
        }
    }
}

impl Actor for OverlayServer {
    type Context = Context<Self>;
}


impl Handler<Connect> for OverlayServer {
    type Result = Uid;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = msg.id.unwrap_or(Uuid::new_v4().as_hyphenated().to_string());
        log::info!("Client {} disconnected", id);
        self.sessions.insert(id.clone(), msg.addr);
        id
    }
}

impl Handler<Disconnect> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
        if let Some(room) = msg.room_id {
            self.leave_room(&room, &msg.id);
        }
        log::info!("Client {} disconnected", msg.id);
    }
}

impl Handler<Join> for OverlayServer {
    type Result = InitClient;

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) -> Self::Result {
        // let room_id = (random::<u8>() % 5 + 1).to_string();
        // let room_state = self.rooms.entry(room_id.clone()).or_default();
        // room_state.users.insert(msg.id.clone());
        // InitClient {
            // id: msg.id,
            // room_id,
            // snapshot: room_state.snapshot.clone(),
            // log: room_state.log.clone(),
        // }
        if let Some(old_room) = msg.old {
            self.leave_room(&old_room, &msg.id);
        }
        let new_room = msg.new.unwrap_or((random::<u8>() % 5 + 1).to_string());
        self.rooms.entry(new_room.clone()).or_default().users.insert(msg.id.clone());
        let new_room_state = self.rooms.entry(new_room.clone()).or_default();
        new_room_state.users.insert(msg.id.clone());
        InitClient {
            id: msg.id,
            room_id: new_room,
            snapshot: new_room_state.snapshot.clone(),
            log: new_room_state.log.clone(),
        }
    }
}

impl Handler<OverlayClientMessage> for OverlayServer {
    type Result = ();

    fn handle(&mut self, mut msg: OverlayClientMessage, _: &mut Context<Self>) {
        let message_kind = msg.msg.message_kind.as_mut().unwrap();
        match message_kind {
            MessageKind::OwnerCandidate(owner_candidate) => {
                // if there is no current owner, you can take it.
                if self.rooms.entry(owner_candidate.room_id.clone()).or_default().owner.is_none() {
                    self.rooms.entry(owner_candidate.room_id.clone()).or_default().owner = Some(owner_candidate.id.clone());
                    owner_candidate.chosen = true;
                }
                self.send_message(&msg.room, &msg.msg);
                self.rooms.entry(msg.room).or_default().log.push(msg.msg);
            }
            MessageKind::Snapshot(snapshot) => {
                // Apply snapshot.
                // 0. save old snapshot's next index.
                // 1. replace old snapshot with new one.
                // 2. replace old log with old_log[new_next_index - old_next_index:].
                let room = self.rooms.entry(msg.room.clone()).or_default();
                let old_next_index = if let Some(old_snapshot) = &room.snapshot {
                    old_snapshot.next_log_index
                } else {
                    0
                };
                let new_next_index = snapshot.next_log_index;
                room.snapshot = Some(snapshot.clone());
                room.log = room.log.split_off((new_next_index - old_next_index) as usize);
                log::info!("recevived snapshot for room: {}", msg.room);
            },
            _ => {
                self.send_message(&msg.room, &msg.msg);
                self.rooms.entry(msg.room).or_default().log.push(msg.msg);
            },
        };
    }
}
