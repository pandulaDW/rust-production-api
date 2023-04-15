use sqlx::PgPool;
use std::net;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let configuration = get_configuration().expect("failed to read configuration");

    let conn_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("failed to connect to postgres");

    let listener = net::TcpListener::bind(format!("127.0.0.1:{}", configuration.application_port))?;
    run(listener, conn_pool)?.await
}
