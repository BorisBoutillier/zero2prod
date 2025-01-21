use actix_web::{
    http::{
        header::{self, HeaderMap, HeaderValue},
        StatusCode,
    },
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use base64::Engine;
use secrecy::Secret;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    domain::SubscriberEmail,
    email_client::EmailClient,
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}
#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}
#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, anyhow::Error> {
    struct Row {
        email: String,
    }
    let rows = sqlx::query_as!(
        Row,
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?;
    let subcribers = rows
        .into_iter()
        .filter_map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Some(ConfirmedSubscriber { email }),
            Err(e) => {
                tracing::warn!("Unexpected incorred subsriber email stored in database\n{e}");
                None
            }
        })
        .collect::<Vec<_>>();
    Ok(subcribers)
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body,pool,email_client,request),
    fields(username=tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn publish_newsletters(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|error| match error {
            AuthError::InvalidCredentials(error) => PublishError::AuthError(error),
            AuthError::Unexpected(error) => PublishError::UnexpectedError(error),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id.to_string()));
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get confirmed subscribers from the database")?;
    for subscriber in subscribers {
        email_client
            .send_email(
                subscriber.email,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .with_context(|| "Failed to send newsletter issue to {recipient}")?;
    }
    Ok(HttpResponse::Ok().finish())
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing")?
        .to_str()
        .context("The 'Authorization' header is not a valid UTF8 string")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not a valid UTF8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in the 'Basic' auth"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in the 'Basic' auth"))?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}
