use actix_web::http::header::HeaderMap;
use anyhow::{anyhow, Context, Result};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

pub fn basic_authentication(headers: &HeaderMap) -> Result<Credentials> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    let decoded_bytes = base64::decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    let credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    let mut credentials = credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

pub async fn validate_credentials(credentials: &Credentials, pool: &PgPool) -> Result<Uuid> {
    let user_id: Option<_> = sqlx::query!(
        r#"SELECT user_id FROM users WHERE username = $1 AND password = $2"#,
        credentials.username,
        credentials.password.expose_secret()
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?;

    user_id
        .map(|row| row.user_id)
        .ok_or_else(|| anyhow!("Invalid username or password."))
}
