use env_logger::Env;
use sqlx::postgres::PgConnectOptions;
use sqlx::{ConnectOptions, PgPool};
use std::net;
use std::str::FromStr;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("failed to read configuration");

    let options = PgConnectOptions::from_str(&configuration.database.connection_string())
        .expect("incorrect db uri")
        .disable_statement_logging()
        .clone();

    let conn_pool = PgPool::connect_with(options)
        .await
        .expect("failed to connect to postgres");

    // `init` calls `set_logger`. We are falling back to printing all logs at info-level
    // or above if the RUST_LOG environment variable has not been set
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let listener = net::TcpListener::bind(format!("127.0.0.1:{}", configuration.application_port))?;
    run(listener, conn_pool)?.await
}
