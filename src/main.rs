use sqlx::postgres::PgPoolOptions;
use std::net;
use tracing::info;

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let configuration = get_configuration().expect("failed to read configuration");
    let conn_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_with(configuration.database.with_db())
        .await
        .expect("failed to connect to postgres");

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    info!(
        "App running on env {} and address {}",
        configuration.env.unwrap().as_str(),
        address
    );
    let listener = net::TcpListener::bind(address)?;
    run(listener, conn_pool)?.await
}
