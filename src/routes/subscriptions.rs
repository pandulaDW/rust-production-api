use actix_web::web::{Data, Form};
use actix_web::HttpResponse;
use anyhow::Context;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::startup::ApplicationBaseUrl;
use crate::{domain::NewSubscriber, email_client::EmailClient};

#[derive(Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

/// Subscribe an email to the newsletter.
///
/// Extract form data using serde.
/// This handler get called only if content type is *x-www-form-urlencoded*
/// and content of the request could be deserialized to a `FormData` struct.
#[tracing::instrument(
    name = "Adding a new subscriber", skip(form, pool, email_client, base_url),
    fields(subscriber_email = %form.email, subscriber_name= %form.name)
)]
pub async fn subscribe(
    form: Form<FormData>,
    pool: Data<PgPool>,
    email_client: Data<EmailClient>,
    base_url: Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber).await?;

    let sub_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &sub_token).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber")?;

    send_confirmation_email(&email_client, new_subscriber, &base_url.0, &sub_token).await?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Saving new subscriber details in the database", skip(s, tx))]
async fn insert_subscriber(
    tx: &mut Transaction<'_, Postgres>,
    s: &NewSubscriber,
) -> Result<Uuid, SubscribeError> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        subscriber_id,
        s.email.as_ref(),
        s.name.as_ref(),
        Utc::now(),
    )
    .execute(tx)
    .await
    .context("Failed to store the confirmation token for a new subscriber")?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(client, subscriber, base_url, token)
)]
async fn send_confirmation_email(
    client: &EmailClient,
    subscriber: NewSubscriber,
    base_url: &str,
    token: &str,
) -> Result<(), SubscribeError> {
    let confirmation_link = format!("{base_url}/subscriptions/confirm?subscription_token={token}");
    client
        .send_email(
            subscriber.email,
            "Welcome!",
            &format!(
                "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
                confirmation_link
            ),
            &format!(
                "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
                confirmation_link
            ),
        )
        .await
        .context("Failed to send a confirmation email")?;
    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for SubscribeError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            Self::ValidationError(_) => reqwest::StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(tx, sub_id, sub_token)
)]
async fn store_token(
    tx: &mut Transaction<'_, Postgres>,
    sub_id: Uuid,
    sub_token: &str,
) -> Result<(), SubscribeError> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscriber_id, subscription_token) VALUES ($1, $2)"#,
        sub_id,
        sub_token,
    )
    .execute(tx)
    .await
    .context("Failed to store the confirmation token for a new subscriber")?;
    Ok(())
}

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

fn error_chain_fmt(e: impl std::error::Error, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();

    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}
