use crate::{domain::SubscriberEmail, email_client::EmailClient, utils};
use actix_web::{
    http::header,
    web::{Data, Json},
    HttpRequest, HttpResponse, ResponseError,
};
use anyhow::anyhow;
use futures::{stream::FuturesUnordered, StreamExt};
use sqlx::PgPool;

use super::auth::basic_authentication;

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
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    basic_authentication(request.headers()).map_err(PublishError::AuthError)?;

    let subscribers = get_confirmed_subscribers(&pool).await?;

    process_all_subscribers(subscribers, Data::new(body.0), email_client).await;

    Ok(HttpResponse::Ok().finish())
}

/// Takes the subscribers, create and process the subscribers in chunks.
async fn process_all_subscribers(
    subscribers: Vec<anyhow::Result<ConfirmedSubscriber>>,
    body: Data<BodyData>,
    email_client: Data<EmailClient>,
) {
    let mut iter = subscribers.into_iter();
    let mut num_processed = 0;
    let num_subscribers = iter.len();
    let chunk_size = 20;

    while num_processed <= num_subscribers {
        let chunk = iter.by_ref().take(chunk_size).collect::<Vec<_>>();

        // process the chunk in parallel
        if let Err(e) = tokio::spawn(process_subscriber_chunk(
            chunk,
            body.clone(),
            email_client.clone(),
        ))
        .await
        {
            tracing::warn!("Failed to execute newsletter sending task: {e}");
        };

        // timeout to not overwhelm the email server
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        num_processed += chunk_size;
    }
}

/// Sends newsletters to each subscriber in parallel
async fn process_subscriber_chunk(
    chunk: Vec<anyhow::Result<ConfirmedSubscriber>>,
    body: Data<BodyData>,
    email_client: Data<EmailClient>,
) {
    let mut futures = FuturesUnordered::new();
    for subscriber in chunk {
        futures.push(async {
            match subscriber {
                Ok(subscriber) => {
                    email_client
                        .send_email(
                            &subscriber.email,
                            &body.title,
                            &body.content.html,
                            &body.content.text,
                        )
                        .await
                }
                Err(error) => {
                    tracing::warn!(
                        error.cause_chain = ?error,
                        "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                    );
                    Ok(())
                }
            }
        });
    }

    while let Some(result) = futures.next().await {
        if result.is_err() {
            tracing::warn!("Failed to send newsletter issue");
        }
    }
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> anyhow::Result<Vec<anyhow::Result<ConfirmedSubscriber>>> {
    let out = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#,)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow!(error)),
        })
        .collect();

    Ok(out)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        utils::error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        use reqwest::{header::HeaderValue, StatusCode};

        match self {
            Self::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}
