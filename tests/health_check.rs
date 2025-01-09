use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    let socket_addr = spawn_app();

    let client = reqwest::Client::new();

    let response = client
        .get(format!("{socket_addr}/health_check"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Can't find a tcp port");
    let port = listener.local_addr().expect("No local Addr").port();
    let server = zero2prod::run(listener).expect("Failed to create server");
    tokio::spawn(server);
    format!("http://127.0.0.1:{port}")
}
