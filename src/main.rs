use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = get_configuration().expect("failed to read configuration");
    let server = Application::build(config).await?;

    server.run_until_stopped().await
}
