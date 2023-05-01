use crate::helpers::spawn_app;
use wiremock::{matchers::any, Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.create_unconfirmed_subscriber("name=le%20guin&email=ursula_le_guin%40gmail.com".into())
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

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), 200);
    // Mock verifies on Drop that we haven't sent the newsletter email
}
