use zero2prod::configuration::get_configuration;
use zero2prod::startup::{build, run};
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod", "info", std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = get_configuration().expect("failed to read configuration");
    let components = build(config).await?;

    run(components)?.await
}
