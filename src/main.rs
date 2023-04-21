use sqlx::postgres::PgConnectOptions;
use sqlx::{ConnectOptions, PgPool};
use std::net;
use std::str::FromStr;

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let configuration = get_configuration().expect("failed to read configuration");
    let options = PgConnectOptions::from_str(&configuration.database.connection_string())
        .expect("incorrect db uri")
        .disable_statement_logging()
        .clone();
    let conn_pool = PgPool::connect_lazy_with(options);

    let listener = net::TcpListener::bind(format!("127.0.0.1:{}", configuration.application_port))?;
    run(listener, conn_pool)?.await
}
