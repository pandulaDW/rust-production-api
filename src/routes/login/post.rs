use actix_web::HttpResponse;
use reqwest::header::LOCATION;

pub async fn post() -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}