use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::NewSubscriber;

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
    name = "Adding a new subscriber", skip(form, pool),
    fields(subscriber_email = %form.email, subscriber_name= %form.name)
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(v) => v,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match insert_subscriber(&pool, new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Saving new subscriber details in the database", skip(s, pool))]
async fn insert_subscriber(pool: &PgPool, s: NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'confirmed')"#,
        Uuid::new_v4(),
        s.email.as_ref(),
        s.name.as_ref(),
        Utc::now(),        
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
