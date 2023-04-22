use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberName};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// Subscribe an email to the newsletter.
///
/// Extract form data using serde.
/// This handler get called only if content type is *x-www-form-urlencoded*
/// and content of the request could be deserialized to a `FormData` struct.
#[tracing::instrument(
    name = "Adding a new subscriber", skip(form, pool),
    fields(subscriber_email = %form.email, subscriber_name= %form.name)
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let Ok(name) = SubscriberName::parse(form.0.name) else {
        return HttpResponse::BadRequest().finish();
    };

    let s = NewSubscriber {
        email: form.0.email,
        name,
    };

    match insert_subscriber(&pool, s).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Saving new subscriber details in the database", skip(s, pool))]
async fn insert_subscriber(pool: &PgPool, s: NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        s.email,
        s.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
