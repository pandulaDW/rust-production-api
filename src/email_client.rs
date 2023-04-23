use crate::domain::SubscriberEmail;
use reqwest::Client;

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    sender: SubscriberEmail,
    base_url: String,
}

impl EmailClient {
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        todo!()
    }

    pub fn new(base_url: String, sender: SubscriberEmail) -> Self {
        Self {
            http_client: Client::new(),
            sender,
            base_url,
        }
    }
}
