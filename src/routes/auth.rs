use actix_web::http::header::HeaderMap;
use anyhow::{anyhow, Context, Result};
use secrecy::Secret;

pub struct Credentials {
    _username: String,
    _password: Secret<String>,
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
        _username: username,
        _password: Secret::new(password),
    })
}
