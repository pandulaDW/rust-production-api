use crate::helpers::{new_sub_request_body, spawn_app};
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

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

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = app.post_newsletters(newsletter_request_body).await;
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

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = app.post_newsletters(newsletter_request_body).await;
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
        let response = app.post_newsletters(invalid_body).await;

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

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    app.post_newsletters(newsletter_request_body).await;
    // Mock verifies on Drop that we have sent the newsletter email to each subscriber
}
