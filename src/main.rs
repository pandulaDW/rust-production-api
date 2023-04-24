use sqlx::postgres::PgPoolOptions;
use std::net;
use tracing::info;

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::{email_client::EmailClient, telemetry};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = get_configuration().expect("failed to read configuration");
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

    let address = format!("{}:{}", config.application.host, config.application.port);

    info!(
        "App running on env {} and address {}",
        config.env.unwrap().as_str(),
        address
    );
    let listener = net::TcpListener::bind(address)?;
    run(listener, conn_pool, email_client)?.await
}
