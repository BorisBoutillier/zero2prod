use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::telemetry::spawn_blocking_with_telemetry;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        from users
        WHERE username = $1
        "#,
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")
    .map_err(AuthError::Unexpected)?;
    let (expected_password_hash, user_id) = match row {
        Some(row) => (row.password_hash, row.user_id),
        None => {
            return Err(AuthError::InvalidCredentials(anyhow::anyhow!(
                "Unknown username."
            )))
        }
    };
    spawn_blocking_with_telemetry(|| {
        verify_password_hash(Secret::new(expected_password_hash), credentials.password)
    })
    .await
    .context("Failed to spawn task")
    .map_err(AuthError::Unexpected)??;
    Ok(user_id)
}

#[tracing::instrument(name = "verify password hash")]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse stored hash in PHC string format")
        .map_err(AuthError::Unexpected)?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)?;
    Ok(())
}
