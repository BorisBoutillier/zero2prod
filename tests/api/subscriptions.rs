use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
pub async fn subscribe_returns_a_200_for_valid_form_data() {
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
    let response = test_app.post_subscriptions(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
pub async fn subscribe_persists_the_new_subscriber() {
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

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch subscriptions");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
pub async fn subscribe_returns_a_400_when_data_is_missing() {
    let test_app = spawn_app().await;

    let test_cases = [
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return  400 (BadRequest) when: {error_message}"
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let test_app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=ursula,&email=", "empty email"),
        ("name=&email=", "both name and email empty"),
    ];
    for (body, description) in test_cases {
        let response = test_app.post_subscriptions(body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return 400 (BadRequest) when: {description}"
        );
    }
}

#[tokio::test]
async fn subscription_sends_a_confirmation_email_for_valid_data() {
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

    // Assert
    // Mock asserts on drop
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);
    assert_eq!(confirmation_links.html, confirmation_links.text);
}

#[tokio::test]
async fn subcribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le&email=a.b@c.d";
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_subscriptions(body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
