use actix_web::HttpResponse;

/// health check handler
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
