use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::net::{self, TcpListener};
use tracing::info;
use tracing_actix_web::TracingLogger;

use crate::configuration::{Environment, Settings};
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};

/// Creates and returns the server (which implements Future trait)
pub fn run(components: AppComponents) -> Result<Server, std::io::Error> {
    let db_conn = web::Data::new(components.conn_pool);
    let email_client = web::Data::new(components.email_client);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_conn.clone())
            .app_data(email_client.clone())
    })
    .listen(components.listener)?
    .run();
    Ok(server)
}

/// Includes all the high level components needed to run the app
pub struct AppComponents {
    pub conn_pool: Pool<Postgres>,
    pub email_client: EmailClient,
    pub listener: TcpListener,
}

/// Builds the components needed to run the app
pub async fn build(config: Settings) -> Result<AppComponents, std::io::Error> {
    let conn_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_with(config.database.with_db())
        .await
        .expect("failed to connect to postgres");

    let email_client = EmailClient::new(
        config.email_client.base_url.clone(),
        config.email_client.sender().expect("invalid sender email"),
        config.email_client.auth_token.clone(),
        config.email_client.timeout(),
    );

    let env = config.env.unwrap();
    let address = match env {
        Environment::Testing => "127.0.0.1:0".to_string(),
        _ => format!("{}:{}", config.application.host, config.application.port),
    };

    info!("App running on {} env with address {address}", env.as_str());

    let listener = net::TcpListener::bind(address)?;

    Ok(AppComponents {
        conn_pool,
        email_client,
        listener,
    })
}
