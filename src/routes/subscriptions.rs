use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubscriptionsFormData {
    email: String,
    name: String,
}

pub async fn subscriptions(_form: web::Form<SubscriptionsFormData>) -> impl Responder {
    HttpResponse::Ok()
}
