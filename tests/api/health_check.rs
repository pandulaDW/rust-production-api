use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{address}/health_check"))
        .send()
        .await
        .expect("failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
