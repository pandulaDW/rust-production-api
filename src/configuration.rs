use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub env: Option<Environment>,
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

/// Read the application settings from a configuration file
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    // Initialize the configuration reader
    let mut settings = config::Config::default();

    let base_path = std::env::current_dir().expect("failed to determine the current directory");
    let config_dir = base_path.join("configuration");

    // Read the "default" configuration file
    settings.merge(config::File::from(config_dir.join("base")).required(true))?;

    // Detect the running environment and default to `local` if unspecified.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    // Layer on the environment-specific values.
    settings.merge(config::File::from(config_dir.join(environment.as_str())).required(true))?;

    // Try to convert the configuration values it read into our Settings type
    let out = settings.try_into();

    if out.is_ok() {
        let mut out: Settings = out.unwrap();
        out.env = Some(environment);
        return Ok(out);
    }

    out
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

/// The possible runtime environment for our application.
#[derive(Deserialize)]
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}
