use actix::prelude::*;
use actix_web::{error, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use uuid::Uuid;

use crate::rooms::{Big2Server, RoomWs};
use crate::utils::get_identifier;

pub async fn create_room(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<ServerData>,
) -> Result<HttpResponse, Error> {
    let id = get_identifier(&req);
    let server: Addr<Big2Server> = data.server_addr.clone();
    let actor = RoomWs::new(server, None, id);
    let resp = ws::start(actor, &req, stream);
    resp
}

pub async fn join_room(
    req: HttpRequest,
    path: web::Path<String>,
    stream: web::Payload,
    data: web::Data<ServerData>,
) -> Result<HttpResponse, Error> {
    let key = path.to_string();
    let server: Addr<Big2Server> = data.server_addr.clone();
    let id = get_identifier(&req);
    let key = match Uuid::parse_str(&key) {
        Ok(key) => key,
        Err(_) => {
            return Err(error::ErrorBadRequest("Invalid UUID"));
        }
    };
    let actor = RoomWs::new(server, Some(key), id);
    let resp = ws::start(actor, &req, stream);
    resp
}

pub struct ServerData {
    pub server_addr: Addr<Big2Server>,
}
