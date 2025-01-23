use actix_web::{
    http::{
        header::{self, HeaderValue},
        StatusCode,
    },
    web, HttpResponse, ResponseError,
};
use anyhow::Context;
use sqlx::PgPool;

use crate::{authentication::UserId, domain::SubscriberEmail, email_client::EmailClient};

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

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(form,pool,email_client,user_id),
    fields(user_id=tracing::field::Empty)
)]
pub async fn publish_newsletters(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PublishError> {
    let user_id = *user_id.into_inner();

    tracing::Span::current().record("user_id", tracing::field::display(&user_id.to_string()));
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get confirmed subscribers from the database")?;
    for subscriber in subscribers {
        email_client
            .send_email(
                subscriber.email,
                &form.title,
                &form.html_content,
                &form.text_content,
            )
            .await
            .with_context(|| "Failed to send newsletter issue to {recipient}")?;
    }
    Ok(HttpResponse::Ok().finish())
}
