use once_cell::sync::Lazy;
use reqwest::Response;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings, Settings},
    email_client::EmailClient,
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

#[tokio::test]
async fn health_check_works() {
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{address}/health_check"))
        .send()
        .await
        .expect("failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let (address, config) = spawn_app().await;
    let mut db_conn = PgConnection::connect_with(&config.database.with_db())
        .await
        .expect("failed to connect to Postgres");

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = make_subscription_request(address.to_string(), body.to_string()).await;

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&mut db_conn)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let (address, _) = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, error_message) in test_cases {
        let response = make_subscription_request(address.to_string(), body.to_string()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_empty() {
    let (address, _) = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, desc) in test_cases {
        let response = make_subscription_request(address.to_string(), body.to_string()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            desc
        );
    }
}

/// Spawns a test app
async fn spawn_app() -> (String, Settings) {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let listener = net::TcpListener::bind("127.0.0.1:0").expect("failed to bind address");
    let port = listener.local_addr().unwrap().port();

    let mut config = get_configuration().expect("failed to read configuration");
    config.database.database_name = Uuid::new_v4().to_string();
    let conn_pool = configure_database(&config.database).await;

    let email_client = EmailClient::new(
        config.email_client.base_url.clone(),
        config.email_client.sender().expect("invalid sender email"),
        config.email_client.auth_token.clone(),
        config.email_client.timeout(),
    );

    let server = zero2prod::startup::run(listener, conn_pool, email_client)
        .expect("failed to start the server");

    // launch the server as a background task
    let _ = tokio::spawn(server);

    (format!("127.0.0.1:{port}"), config)
}

/// creates a new test database and returns a connection to it
async fn configure_database(config: &DatabaseSettings) -> PgPool {
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

async fn make_subscription_request(address: String, body: String) -> Response {
    let client = reqwest::Client::new();

    client
        .post(&format!("http://{address}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.")
}
