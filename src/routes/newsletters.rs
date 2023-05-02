use actix_web::{
    web::{Data, Json},
    HttpResponse, ResponseError,
};
use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient, utils};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[tracing::instrument(name = "Publish Newsletter", skip(body, pool, email_client))]
pub async fn publish_newsletter(
    body: Json<BodyData>,
    pool: Data<PgPool>,
    email_client: Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        email_client
            .send_email(
                &subscriber.email,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .with_context(|| {
                format!(
                    "Failed to send newsletter issue to {}",
                    subscriber.email.as_ref()
                )
            })?;
    }

    Ok(HttpResponse::Ok().finish())
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(pool: &PgPool) -> Result<Vec<ConfirmedSubscriber>> {
    struct Row {
        email: String,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#,
    )
    .fetch_all(pool)
    .await?;

    let out = rows
        .into_iter()
        .filter_map(|v| match SubscriberEmail::parse(v.email) {
            Ok(v) => Some(ConfirmedSubscriber { email: v }),
            Err(e) => {
                tracing::warn!("A confirmed subscriber is using an invalid email address.\n{e}");
                None
            }
        })
        .collect();

    Ok(out)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            Self::UnexpectedError(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
