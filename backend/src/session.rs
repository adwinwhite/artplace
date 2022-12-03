use std::time::{Duration, Instant};

use crate::server;
use artplace::wsmsg;
use artplace::wsmsg::client_message::Kind as ClientKind;
use artplace::wsmsg::server_message::Kind as ServerKind;

use actix::prelude::*;
use actix_web_actors::ws;
// use uuid::Uuid;
use prost::Message;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WsClientSession {
    /// unique session id
    // pub id: messages::Uid,
    pub id: Option<String>,
    pub room_id: Option<String>,

    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,

    pub server: Addr<server::OverlayServer>,
}

impl WsClientSession {
    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                act.server.do_send(server::Disconnect {
                    id: act.id.clone().unwrap(),
                    room_id: act.room_id.clone()
                });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WsClientSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with ChatServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        // log::info!("client {} connected", Uuid::from_u64_pair(self.id.first, self.id.second));
        let client_addr = ctx.address();
        self.server
            .send(server::Connect {
                id: self.id.clone(),
                addr: client_addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(id) => {
                        act.id = Some(id.clone());
                        let server_message = wsmsg::ServerMessage {
                            kind: Some(ServerKind::SetId(wsmsg::SetId { id })),
                        };
                        let bytes = server_message.encode_to_vec();
                        ctx.binary(bytes);
                    }
                    Err(_) => {
                        ctx.stop();
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.server.do_send(server::Disconnect {
            id: self.id.clone().unwrap(),
            room_id: self.room_id.clone()
        });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<wsmsg::ServerMessage> for WsClientSession {
    type Result = ();

    fn handle(&mut self, msg: wsmsg::ServerMessage, ctx: &mut Self::Context) {
        ctx.binary(msg.encode_to_vec())
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsClientSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        log::debug!("WEBSOCKET MESSAGE: {msg:?}");
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(_) => println!("Unexpected text"),
            ws::Message::Binary(bin) => match wsmsg::ClientMessage::decode(bin) {
                Ok(client_msg) => {
                    // log::info!("received client message: {:#?}", client_msg);
                    match client_msg.kind.unwrap() {
                        ClientKind::SetBrush(set_brush) => {
                            if let Err(err) = self.server.try_send(server::SetBrush {
                                id: self.id.clone().unwrap(),
                                room_id: self.room_id.clone().unwrap(),
                                set_brush,
                            }) {
                                ctx.binary(
                                    (wsmsg::ServerMessage {
                                        kind: Some(ServerKind::ServerError(err.to_string())),
                                    })
                                    .encode_to_vec(),
                                );
                            }
                        }
                        ClientKind::Movement(movement) => {
                            if let Err(err) = self.server.try_send(server::Movement {
                                id: self.id.clone().unwrap(),
                                room_id: self.room_id.clone().unwrap(),
                                movement,
                            }) {
                                ctx.binary(
                                    (wsmsg::ServerMessage {
                                        kind: Some(ServerKind::ServerError(err.to_string())),
                                    })
                                    .encode_to_vec(),
                                );
                            }
                        }
                        ClientKind::JoinRoom(join_room) => {
                            self.server
                                .send(server::JoinRoom {
                                    id: self.id.clone().unwrap(),
                                    room_id: self.room_id.clone(),
                                    join_room,
                                })
                                .into_actor(self)
                                .then(|res, act, ctx| {
                                    match res {
                                        Ok(room_init) => {
                                            act.room_id = Some(room_init.room_id.clone());
                                            let server_message = wsmsg::ServerMessage {
                                                kind: Some(ServerKind::RoomInit(room_init)),
                                            };
                                            let bytes = server_message.encode_to_vec();
                                            ctx.binary(bytes);
                                        }
                                        _ => ctx.stop(),
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx);
                        }
                        ClientKind::Snapshot(snapshot) => {
                            if let Err(err) = self.server.try_send(server::Snapshot {
                                id: self.id.clone().unwrap(),
                                room_id: self.room_id.clone().unwrap(),
                                snapshot,
                            }) {
                                ctx.binary(
                                    (wsmsg::ServerMessage {
                                        kind: Some(ServerKind::ServerError(err.to_string())),
                                    })
                                    .encode_to_vec(),
                                );
                            }
                        }
                        ClientKind::SnapperRequest(snapper_request) => {
                            if let Err(err) = self.server.try_send(server::SnapperRequest {
                                id: self.id.clone().unwrap(),
                                room_id: self.room_id.clone().unwrap(),
                                snapper_request,
                            }) {
                                ctx.binary(
                                    (wsmsg::ServerMessage {
                                        kind: Some(ServerKind::ServerError(err.to_string())),
                                    })
                                    .encode_to_vec(),
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("failed to decode client message: {}", err);
                }
            },
            ws::Message::Close(reason) => {
                log::info!(
                    "Client {:#?} disconnected for reason: {:#?}",
                    self.id,
                    reason
                );
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
