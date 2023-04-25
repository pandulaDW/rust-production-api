use once_cell::sync::Lazy;
use reqwest::Response;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::task::AbortHandle;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
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

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub app_abort_handler: AbortHandle,
    // pub test_user: TestUser,
    // pub api_client: reqwest::Client,
    // pub email_client: EmailClient,
}

impl TestApp {
    /// Make post subscription requests
    pub async fn post_subscriptions(&self, body: String) -> Response {
        let client = reqwest::Client::new();
        client
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request.")
    }
}

/// Spawns a new test app
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    // build a mock email server that can intercept email requests
    let email_server = MockServer::builder().start().await;

    let config = {
        let mut c = get_configuration().expect("failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create and migrate the database
    configure_database(&config.database).await;

    // build the app
    let app = Application::build(config.clone())
        .await
        .expect("failed to build the server");
    let application_port = app.port();

    // launch the server as a background task and return its handle
    let t = tokio::spawn(app.run_until_stopped());

    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&config.database),
        email_server,
        app_abort_handler: t.abort_handle(),
    };

    test_app
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

impl Drop for TestApp {
    // Aborts the background task associated with running the test app.
    // This will prevent having multiple servers running in the background for each test
    fn drop(&mut self) {
        self.app_abort_handler.abort();
    }
}
