use std::assert_eq;

use crate::helpers::{new_sub_request_body, spawn_app};
use once_cell::sync::Lazy;
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

static NEWSLETTER_CORRECT_BODY: Lazy<serde_json::Value> = Lazy::new(|| {
    serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    })
});

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.create_unconfirmed_subscriber(new_sub_request_body())
        .await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0) // assert no request is fired at Postmark
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletters(&*NEWSLETTER_CORRECT_BODY).await;
    assert_eq!(response.status(), 200);

    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    app.create_confirmed_subscriber(new_sub_request_body())
        .await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletters(&*NEWSLETTER_CORRECT_BODY).await;
    assert_eq!(response.status(), 200);

    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    app.create_confirmed_subscriber("name=le%20guin&email=ursula_le_guin%40gmail.com".into())
        .await;

    let test_cases = vec![
        (
            serde_json::json!({
                            "content": {
                                "text": "Newsletter body as plain text",
                                "html": "<p>Newsletter body as HTML</p>",
            } }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body, error) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;

        assert_eq!(
            response.status(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}",
            error
        );
    }
}

#[tokio::test]
async fn newsletters_are_delivered_to_multiple_subscribers() {
    let app = spawn_app().await;
    let num_new_subscribers = 45;

    for _ in 1..=num_new_subscribers {
        app.create_confirmed_subscriber(new_sub_request_body())
            .await;
    }

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(num_new_subscribers)
        .mount(&app.email_server)
        .await;

    app.post_newsletters(&*NEWSLETTER_CORRECT_BODY).await;
    // Mock verifies on Drop that we have sent the newsletter email to each subscriber
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(401, response.status());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn invalid_auth_is_rejected() {
    let app = spawn_app().await;

    // assert 401 for non-existing user
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(Uuid::new_v4().to_string(), Some(Uuid::new_v4().to_string()))
        .json(&*NEWSLETTER_CORRECT_BODY)
        .send()
        .await
        .unwrap();

    assert_eq!(401, response.status());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );

    // assert 401 for incorrect password
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(&app.test_user.username, Some(Uuid::new_v4().to_string()))
        .json(&*NEWSLETTER_CORRECT_BODY)
        .send()
        .await
        .unwrap();

    assert_eq!(401, response.status());
}
