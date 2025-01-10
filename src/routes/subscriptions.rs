use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SubscriptionsFormData {
    email: String,
    name: String,
}

pub async fn subscriptions(
    form: web::Form<SubscriptionsFormData>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    match sqlx::query!(
        r#"INSERT INTO subscriptions  (id, email, name, subscribed_at) VALUES ($1,$2,$3,$4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => {
            println!("Failed to execute query: {e}");
            HttpResponse::InternalServerError()
        }
    }
}
