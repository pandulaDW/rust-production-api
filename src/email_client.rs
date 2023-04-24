use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    sender: SubscriberEmail,
    base_url: String,
    auth_token: Secret<String>,
}

/// Request body specified in the postmark docs
#[derive(Serialize)]
struct SendEmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
}

impl EmailClient {
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref().to_owned(),
            to: recipient.as_ref().to_owned(),
            subject: subject.to_owned(),
            html_body: html_content.to_owned(),
            text_body: text_content.to_owned(),
        };

        let _response = self
            .http_client
            .post(&url)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&request_body)
            .send()
            .await?;

        Ok(())
    }

    pub fn new(base_url: String, sender: SubscriberEmail, auth_token: Secret<String>) -> Self {
        Self {
            http_client: Client::new(),
            sender,
            base_url,
            auth_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use crate::domain::SubscriberEmail;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                // Check that all the mandatory fields are populated without inspecting the field values
                return body.get("from").is_some()
                    && body.get("to").is_some()
                    && body.get("subject").is_some()
                    && body.get("html_body").is_some()
                    && body.get("text_body").is_some();
            }
            false
        }
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(mock_server.uri(), sender, Secret::new(Faker.fake()));

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        // Act
        _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        // Assert (Mock expectations are checked on drop)
    }
}
