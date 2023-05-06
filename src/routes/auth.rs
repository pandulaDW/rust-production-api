use actix_web::http::header::HeaderMap;
use anyhow::{anyhow, Context, Result};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
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
    let row: Option<_> = sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE username = $1"#,
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?;

    let (expected_hash, user_id) = match row {
        Some(row) => (row.password_hash, row.user_id),
        None => return Err(anyhow!("Unknown username")),
    };

    verify_password(
        credentials.password.expose_secret().to_string(),
        expected_hash,
    )
    .await?;

    Ok(user_id)
}

pub async fn verify_password(received_password: String, expected_hash: String) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let expected_password_hash = PasswordHash::new(&expected_hash)
            .context("Failed to parse hash in PHC string format")?;

        Argon2::default()
            .verify_password(received_password.as_bytes(), &expected_password_hash)
            .context("Invalid password")
    })
    .await
    .context("Failed to spawn blocking task")?
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .context("Failed to hash the password")?;
    Ok(password_hash.to_string())
}
