use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::net::{self, TcpListener};
use tracing::info;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{confirm, health_check, subscribe};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    /// Builds the components needed to run the app
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
        let conn_pool = get_connection_pool(&config.database);
        let email_client = config.email_client.client();

        let address = format!("{}:{}", config.application.host, config.application.port);
        info!(
            "App running on {} env with address {address}",
            config.env.unwrap().as_str()
        );

        let listener = net::TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, conn_pool, email_client).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

/// Creates and returns the server (which implements Future trait)
async fn run(
    listener: TcpListener,
    conn_pool: Pool<Postgres>,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let db_conn = web::Data::new(conn_pool);
    let email_client = web::Data::new(email_client);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(db_conn.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
