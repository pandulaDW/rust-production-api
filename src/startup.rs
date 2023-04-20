use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::routes::{health_check, subscribe};

/// Creates and returns the server (which implements Future trait)
pub fn run(listener: TcpListener, db_conn: PgPool) -> Result<Server, std::io::Error> {
    let db_conn = web::Data::new(db_conn);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_conn.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
