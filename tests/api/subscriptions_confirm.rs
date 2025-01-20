use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_tokens_are_rejected_with_400() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .expect("Could not send the test Request");

    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn confirmations_with_bad_tokens_are_rejected_with_401_unauthorized() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let response = reqwest::Client::new()
        .get(format!("{}/subscriptions/confirm", test_app.address))
        .query(&[("subscription_token", "000000")])
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn confirmations_with_correct_tokens_are_confirmed() {
    // Arrange
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    // Act
    test_app.post_subscriptions(body).await;

    // Get the link in the email and fetch the subscription token
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    // Pase the body as JSON, starting from RAW bytes
    let confirmation_links = test_app.get_confirmation_links(email_request);

    // Assert
    //
    // Check that we don't lin kto random API on the web
    assert_eq!(confirmation_links.html.host_str().unwrap(), "localhost");

    // Check and extract subscription_token in confirmation link query
    let subscription_token = confirmation_links
        .html
        .query_pairs()
        .find(|(k, _)| k == "subscription_token")
        .map(|(_, v)| v)
        .expect("No subscription_token query parameter in email confirmation link");
    tracing::info!("Subscription token: {subscription_token}");

    // Act
    // Call the confirm API with the extracted subscription token
    let response = reqwest::Client::new()
        .get(format!("{}/subscriptions/confirm", test_app.address))
        .query(&[("subscription_token", subscription_token)])
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
