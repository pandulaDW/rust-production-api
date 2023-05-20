use once_cell::sync::Lazy;
use reqwest::{Response, Url};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::task::AbortHandle;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, ResponseTemplate,
};
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings, Environment},
    routes::auth,
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

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: FirstName().fake::<String>(),
            password: Password(8..16).fake::<String>(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            &self.username,
            auth::hash_password(&self.password).unwrap(),
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");
    }
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub app_abort_handler: AbortHandle,
    pub test_user: TestUser,
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

    /// Extract the confirmation links (html and plain text) from the mail body
    pub fn get_confirmation_links(&self, email_request: &Request) -> (Url, Url) {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();
            let confirmation_link = Url::parse(&raw_link).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        (html, plain_text)
    }

    /// Use the public API of the application under test to create
    /// an unconfirmed subscriber and return the confirmation link received
    pub async fn create_unconfirmed_subscriber(&self, body: String) -> Url {
        let _guard = Mock::given(path("/email"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount_as_scoped(&self.email_server)
            .await;

        self.post_subscriptions(body)
            .await
            .error_for_status()
            .unwrap();

        let email_request = self
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();

        self.get_confirmation_links(&email_request).0
    }

    /// creates a confirmed subscriber
    pub async fn create_confirmed_subscriber(&self, body: String) {
        let confirmation_link = self.create_unconfirmed_subscriber(body).await;
        reqwest::get(confirmation_link)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    /// post a newsletter
    pub async fn post_newsletters(&self, body: &serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(body)
            .send()
            .await
            .expect("Failed to execute request.")
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
        c.env = Environment::Testing;
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

    // create a test user
    let test_user = TestUser::generate();
    test_user
        .store(&get_connection_pool(&config.database))
        .await;

    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&config.database),
        email_server,
        app_abort_handler: t.abort_handle(),
        test_user,
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

use fake::{
    faker::{
        internet::en::{Password, SafeEmail},
        name::en::{FirstName, LastName},
    },
    Fake,
};

/// returns a request body for creating a new subscriber
pub fn new_sub_request_body() -> String {
    let f_name = FirstName().fake::<String>();
    let l_name = LastName().fake::<String>();
    let email = SafeEmail().fake::<String>().replace("@", "%40");
    format!("name={f_name}%20{l_name}&email={email}")
}
