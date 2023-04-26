use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(parameters: Query<Parameters>, pool: Data<PgPool>) -> HttpResponse {
    let Ok(id) = get_subscriber_id_from_token(&pool, &parameters.subscription_token).await else {
        return HttpResponse::InternalServerError().finish();
    };

    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(sub_id) => {
            if confirm_subscriber(&pool, sub_id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            };
            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, sub_token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    sub_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        sub_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, sub_id))]
async fn confirm_subscriber(pool: &PgPool, sub_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status='confirmed' WHERE id=$1"#,
        sub_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
