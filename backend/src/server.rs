use std::collections::{HashMap, HashSet};

use artplace::wsmsg;
use artplace::wsmsg::client_message;
use artplace::wsmsg::server_message;
use artplace::wsmsg::RoomInit;
use artplace::wsmsg::server_message::Kind as ServerKind;



use actix::prelude::*;
use rand::random;
use uuid::Uuid;


pub type RoomId = String;
pub type Uid = String;

#[derive(Message)]
#[rtype(Uid)]
pub struct Connect {
    pub id: Option<Uid>,
    pub addr: Recipient<wsmsg::ServerMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uid,
    pub room_id: Option<RoomId>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetBrush {
    pub id: Uid,
    pub room_id: RoomId,
    pub set_brush: client_message::SetBrush
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Movement {
    pub id: Uid,
    pub room_id: RoomId,
    pub movement: client_message::Movement
}

#[derive(Message)]
#[rtype(RoomInit)]
pub struct JoinRoom {
    pub id: Uid,
    pub room_id: Option<RoomId>,
    pub join_room: client_message::JoinRoom
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Snapshot {
    pub id: Uid,
    pub room_id: RoomId,
    pub snapshot: wsmsg::Snapshot
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SnapperRequest {
    pub id: Uid,
    pub room_id: RoomId,
    pub snapper_request: wsmsg::SnapperRequest
}

#[derive(Default, Clone)]
struct RoomState {
    users: HashSet<Uid>,
    snapper: Option<Uid>,
    snapshot: Option<wsmsg::Snapshot>,
    log: Vec<wsmsg::ServerMessage>
}

struct UserState {
    addr: Recipient<wsmsg::ServerMessage>,
    room: Option<RoomId>
}

pub struct OverlayServer {
    sessions: HashMap<Uid, UserState>,
    rooms: HashMap<RoomId, RoomState>,
}

impl OverlayServer {
    pub fn new() -> OverlayServer {
        OverlayServer {
            sessions: HashMap::new(),
            rooms: HashMap::new(),
        }
    }

    fn send_message(&mut self, room_id: &str, msg: &wsmsg::ServerMessage) {
        if let Some(room) = self.rooms.get(room_id) {
            for id in &room.users {
                if let Some(user) = self.sessions.get(id) {
                    user.addr.do_send(msg.clone());
                }
            }
            self.rooms.entry(room_id.to_string()).or_default().log.push(msg.clone());
        }
    }

    fn leave_room(&mut self, room_id: &str, id: &str) {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        room.users.remove(id);
        // emptry room owner if it's me.
        if let Some(snapper) = &mut room.snapper {
            if snapper == id {
                room.snapper = None;
            }
        }
        self.send_message(room_id, &wsmsg::ServerMessage {
            kind: Some(ServerKind::LeaveRoom(wsmsg::LeaveRoom {
                id: id.to_string()
            }))
        });
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
        self.sessions.insert(id.clone(), UserState {
            addr: msg.addr,
            room: None,
        });
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

impl Handler<SetBrush> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: SetBrush, _: &mut Context<Self>) {
        self.send_message(&msg.room_id, &wsmsg::ServerMessage {
            kind: Some(ServerKind::SetBrush(
                server_message::SetBrush {
                    id: msg.id,
                    brush: msg.set_brush.brush,
                }
            ))
        });
    }
}

impl Handler<Movement> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: Movement, _: &mut Context<Self>) {
        self.send_message(&msg.room_id, &wsmsg::ServerMessage {
            kind: Some(ServerKind::Movement(
                server_message::Movement {
                    id: msg.id,
                    point: msg.movement.point,
                    kind: msg.movement.kind,
                }
            ))
        });
    }
}

impl Handler<JoinRoom> for OverlayServer {
    type Result = RoomInit;

    fn handle(&mut self, msg: JoinRoom, _: &mut Context<Self>) -> Self::Result {
        if let Some(old) = msg.room_id {
            self.leave_room(&old, &msg.id);
        }
        let new_room = msg.join_room.room_id.unwrap_or((random::<u8>() % 5 + 1).to_string());
        self.rooms.entry(new_room.clone()).or_default().users.insert(msg.id.clone());
        let new_room_state = self.rooms.entry(new_room.clone()).or_default();
        new_room_state.users.insert(msg.id.clone());
        let new_room_clone = new_room_state.clone();
        self.sessions.get_mut(&msg.id).unwrap().room = Some(new_room.clone());
        self.send_message(&new_room, &wsmsg::ServerMessage {
            kind: Some(ServerKind::JoinRoom(server_message::JoinRoom {
                id: msg.id
            }))
        });
        RoomInit {
            room_id: new_room,
            snapshot: new_room_clone.snapshot,
            log: new_room_clone.log,
        }
    }
}

impl Handler<Snapshot> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: Snapshot, _: &mut Context<Self>) -> Self::Result {
        // Apply snapshot.
        // 0. save old snapshot's next index.
        // 1. replace old snapshot with new one.
        // 2. replace old log with old_log[new_next_index - old_next_index:].
        let room = self.rooms.entry(msg.room_id.clone()).or_default();
        let old_next_index = if let Some(old_snapshot) = &room.snapshot {
            old_snapshot.next_log_index
        } else {
            0
        };
        let new_next_index = msg.snapshot.next_log_index;
        room.snapshot = Some(msg.snapshot);
        room.log = room.log.split_off((new_next_index - old_next_index) as usize);
        log::info!("recevived snapshot for room: {}", msg.room_id);
    }
}

impl Handler<SnapperRequest> for OverlayServer {
    type Result = ();

    fn handle(&mut self, msg: SnapperRequest, _: &mut Context<Self>) -> Self::Result {
        // if there is no current snapper, you can do it.
        let mut room = self.rooms.get_mut(&msg.room_id).unwrap();
        if room.snapper.is_none() {
            room.snapper = Some(msg.id.clone());
        }
        self.send_message(&msg.room_id, &wsmsg::ServerMessage {
            kind: Some(ServerKind::SetSnapper(
                wsmsg::SetSnapper {
                    id: msg.id,
                }
            ))
        });
    }
}

