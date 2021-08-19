#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::convert::TryFrom;
use std::path;

use actix::prelude::*;
use actix_files::Files;
use actix_web::dev::Service;
use actix_web::http::header::{CacheControl, CacheDirective, CACHE_CONTROL};
use actix_web::http::HeaderValue;
use actix_web::{web, App, HttpServer};

mod messages;
mod rooms;
mod routes;
mod utils;

use crate::rooms::Big2Server;
use crate::routes::{create_room, join_room, ServerData};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let server_addr = Big2Server::new().start();
    let data = ServerData { server_addr };
    let data = web::Data::new(data);

    let mut built_path = path::Path::new("./public");
    if !built_path.exists() {
        built_path = path::Path::new("../public");
    }

    if !built_path.exists() {
        println!("Can't find built_path: {}", built_path.to_string_lossy());
        return Ok(());
    }

    HttpServer::new(move || {
        let factory = web::scope("/ws")
            .route("/{key}", web::get().to(join_room))
            .route("/", web::get().to(create_room));
        let app = App::new()
            // Cache all requests to paths in /static otherwise don't cache
            .wrap_fn(|req, srv| {
                let is_static = req.path().starts_with("/static") || req.path().ends_with(".wasm");
                let cache_static = match is_static {
                    true => CacheControl(vec![CacheDirective::MaxAge(86400)]).to_string(),
                    false => CacheControl(vec![CacheDirective::Extension(
                        "s-maxage".to_owned(),
                        Some("300".to_owned()),
                    )])
                    .to_string(),
                };
                let fut = srv.call(req);
                async {
                    let mut res = fut.await?;
                    let cache_control: HeaderValue =
                        HeaderValue::try_from(cache_static).expect("Oops");
                    res.headers_mut().insert(CACHE_CONTROL, cache_control);
                    Ok(res)
                }
            })
            .app_data(data.clone())
            .service(factory);
        app.service(Files::new("/", built_path).index_file("index.html"))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}
