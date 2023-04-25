use once_cell::sync::Lazy;
use reqwest::Response;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{self, get_configuration, DatabaseSettings, Settings},
    startup::{build, run},
    telemetry,
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info";
    let subscriber_name = "test";

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        telemetry::init_subscriber(subscriber);
    }
});

/// Spawns a test app
pub async fn spawn_app() -> (String, Settings) {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let mut config = get_configuration().expect("failed to read configuration");
    config.env = Some(configuration::Environment::Testing);
    config.database.database_name = Uuid::new_v4().to_string();
    configure_database(&config.database).await;

    let components = build(config.clone())
        .await
        .expect("failed to build the app components");
    let port = components.listener.local_addr().unwrap().port();

    let server = run(components).expect("failed to bind the listener to the server");
    // launch the server as a background task
    let _ = tokio::spawn(server);

    (format!("127.0.0.1:{}", port), config)
}

/// creates a new test database and returns a connection to it
pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut conn = PgConnection::connect_with(&config.without_db())
        .await
        .expect("failed to connect to Postgres");

    conn.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create database.");

    let pool = PgPool::connect_with(config.with_db())
        .await
        .expect("failed to connect to postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to migrate the database");

    pool
}

/// Make post subscription requests
pub async fn post_subscriptions(address: String, body: String) -> Response {
    let client = reqwest::Client::new();
    client
        .post(&format!("http://{address}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.")
}
