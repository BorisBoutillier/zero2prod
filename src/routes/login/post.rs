use actix_web::{
    error::InternalError,
    http::{header::LOCATION, StatusCode},
    web, HttpResponse, ResponseError,
};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, AuthError, Credentials};

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
impl ResponseError for LoginError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(name = "Login request", skip(form, pool),fields(username=tracing::field::Empty,user_id=tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(err) => {
            let login_error = match err {
                AuthError::Unexpected(e) => LoginError::UnexpectedError(e),
                AuthError::InvalidCredentials(e) => LoginError::AuthError(e),
            };
            FlashMessage::error(login_error.to_string()).send();
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish();
            Err(InternalError::from_response(login_error, response))
        }
    }
}
