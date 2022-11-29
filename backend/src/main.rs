//! Simple echo websocket server.
//!
//! Open `http://localhost:8080/` in browser to test.
//!
use std::time::Instant;

use actix_web::{middleware, get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use actix::*;
use serde::Deserialize;

use uuid::Uuid;

mod server;
mod session;
// use artplace::messages;
// use self::server::MyWebSocket;


/// Entry point for our websocket route
async fn accept_client(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::OverlayServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        session::WsClientSession {
            id: {
                // let uuid_u64_pair = Uuid::new_v4().as_u64_pair();
                // messages::Uid {
                    // first: uuid_u64_pair.0,
                    // second: uuid_u64_pair.1,
                // }
                // messages::Uid {
                    // first: 13956278896965799043,
                    // second: 9128467882299892980,
                // }
                Uuid::new_v4().as_hyphenated().to_string()
            },
            room: "".to_owned(),
            hb: Instant::now(),
            server: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[derive(Deserialize)]
pub struct EchoQuery {
    input: String,
}

#[get("/echo")]
async fn echo(info: web::Query<EchoQuery>) -> String {
    info.input.clone()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    println!(env!("OUT_DIR"));
    log::info!("starting HTTP server at http://localhost:8080");

    let server = server::OverlayServer::new().start();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            // WebSocket UI HTML file
            // websocket route
            .service(web::resource("/ws").route(web::get().to(accept_client)))
            .service(actix_files::Files::new("/artplace", "../frontend/dist").index_file("../frontend/dist/index.html").redirect_to_slash_directory())
            .service(echo)
            // enable logger
            .wrap(middleware::Logger::default())
    })
    .workers(2)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
