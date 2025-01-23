use actix_web::{web, HttpResponse};
use actix_web_flash_messages::{FlashMessage, Level};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[derive(Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = *user_id.into_inner();
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::new(
            "You entered two different new passwords - the field values must match.".to_string(),
            Level::Error,
        )
        .send();
        return Ok(see_other("/admin/password"));
    }
    let password_length = form.new_password.expose_secret().len();
    if !(12..=128).contains(&password_length) {
        FlashMessage::new(
            "New password length must be at least 12 and at most 128 characters.".to_string(),
            Level::Error,
        )
        .send();
        return Ok(see_other("/admin/password"));
    }
    let row = sqlx::query!(
        r#"
        SELECT password_hash
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool.as_ref())
    .await
    .map_err(e500)?;
    let expected_password_hash = PasswordHash::new(&row.password_hash).map_err(e500)?;
    if Argon2::default()
        .verify_password(
            form.current_password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .is_err()
    {
        FlashMessage::new(
            "The current password is incorrect.".to_string(),
            Level::Error,
        )
        .send();
        return Ok(see_other("/admin/password"));
    }
    let salt = SaltString::generate(&mut rand::thread_rng());
    let new_password_hash = Argon2::default()
        .hash_password(form.new_password.expose_secret().as_bytes(), &salt)
        .unwrap()
        .to_string();
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        new_password_hash.to_string(),
        user_id
    )
    .execute(pool.as_ref())
    .await
    .map_err(e500)?;
    FlashMessage::new("Your password has been changed.".to_string(), Level::Error).send();
    session.purge();
    Ok(see_other("/login"))
}
