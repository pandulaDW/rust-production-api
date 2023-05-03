use crate::helpers::{new_sub_request_body, spawn_app};

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    app.create_unconfirmed_subscriber(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, error_message) in test_cases {
        let response = app.post_subscriptions(body.to_string()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, desc) in test_cases {
        let response = app.post_subscriptions(body.to_string()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            desc
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    app.create_unconfirmed_subscriber(new_sub_request_body())
        .await;

    // first intercepted request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let (html_link, text_link) = app.get_confirmation_links(email_request);
    assert_eq!(html_link, text_link);
}

#[tokio::test]
async fn subscribe_saves_subscription_token() {
    let app = spawn_app().await;
    app.create_unconfirmed_subscriber(new_sub_request_body())
        .await;

    let saved_user = sqlx::query!("SELECT id FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    let sub_id = saved_user.id;

    let saved = sqlx::query!(
        "SELECT * FROM subscription_tokens WHERE subscriber_id=$1",
        sub_id
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(saved.subscription_token.len(), 25);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = new_sub_request_body();
    app.create_unconfirmed_subscriber(body.clone()).await;

    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body).await;
    assert_eq!(response.status(), 500);
}
