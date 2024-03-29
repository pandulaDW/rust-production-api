use crate::helpers::{new_sub_request_body, spawn_app};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    app.create_unconfirmed_subscriber(new_sub_request_body())
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let (confirmation_link, _) = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_link).await.unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    app.create_unconfirmed_subscriber(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let (confirmation_link, _) = app.get_confirmation_links(email_request);

    reqwest::get(confirmation_link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
