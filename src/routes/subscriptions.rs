use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

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
pub async fn subscribe(form: web::Form<FormData>, db_conn: web::Data<PgPool>) -> HttpResponse {
    match sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_conn.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            println!("failed to execute query: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
