use std::net;

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{address}/health_check"))
        .send()
        .await
        .expect("failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

/// Launch the server as a background task
fn spawn_app() -> String {
    let listener = net::TcpListener::bind("127.0.0.1:0").expect("failed to bind address");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("failed to start the server");

    // launch the server as a background task
    let _ = tokio::spawn(server);

    format!("127.0.0.1:{port}")
}
