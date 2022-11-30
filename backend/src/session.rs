use std::time::{Duration, Instant};

use crate::server;
use artplace::messages;
use artplace::messages::client_message::MessageKind;

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
    pub id: String,
    pub room_id: String,

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
                    id: act.id.clone(),
                    room_id: act.room_id.clone(),
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
        log::info!("client {} connected", self.id);
        let client_addr = ctx.address();
        self.server
            .send(server::Connect {
                id: self.id.clone(),
                addr: client_addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(init_client) => {
                        act.room_id = init_client.room_id.clone();
                        let client_message = messages::ClientMessage {
                            message_kind: Some(MessageKind::InitClient(init_client)),
                        };
                        // println!("{:#?}", client_message);
                        let bytes = client_message.encode_to_vec();
                        // let hex : String = bytes.iter()
                        // .map(|b| format!("{:02x}", b))
                        // .collect::<Vec<String>>()
                        // .join("");
                        // println!("{}", hex);
                        ctx.binary(bytes);
                    }
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.server.do_send(server::Disconnect {
            id: self.id.clone(),
            room_id: self.room_id.clone(),
        });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<messages::ClientMessage> for WsClientSession {
    type Result = ();

    fn handle(&mut self, msg: messages::ClientMessage, ctx: &mut Self::Context) {
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
            ws::Message::Binary(bin) => match messages::ClientMessage::decode(bin) {
                Ok(client_msg) => {
                    // log::info!("received client message: {:#?}", client_msg);
                    if let Some(messages::client_message::MessageKind::JoinRoom(join_room)) =
                        &client_msg.message_kind
                    {
                        self.server
                            .send(server::Join {
                                id: self.id.clone(),
                                old: self.room_id.clone(),
                                new: join_room.room_id.clone(),
                            })
                            .into_actor(self)
                            .then(|res, act, ctx| {
                                match res {
                                    Ok(init_client) => {
                                        act.room_id = init_client.room_id.clone();
                                        let client_message = messages::ClientMessage {
                                            message_kind: Some(MessageKind::InitClient(
                                                init_client,
                                            )),
                                        };
                                        let bytes = client_message.encode_to_vec();
                                        ctx.binary(bytes);
                                    }
                                    _ => ctx.stop(),
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    }
                    self.server.do_send(server::OverlayClientMessage {
                        room: self.room_id.clone(),
                        msg: client_msg,
                    });
                }
                Err(err) => {
                    log::error!("failed to decode client message: {}", err);
                }
            },
            ws::Message::Close(reason) => {
                log::info!("Client {} disconnected for reason: {:#?}", self.id, reason);
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
