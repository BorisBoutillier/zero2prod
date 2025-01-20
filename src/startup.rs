use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::{configuration::Settings, email_client::EmailClient, routes::*};

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/newsletters", web::post().to(publish_newsletters))
            .route("/subscriptions", web::post().to(subscriptions))
            .route(
                "/subscriptions/confirm",
                web::get().to(subscriptions_confirm),
            )
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
pub struct Application {
    server: Server,
    port: u16,
    db_pool: PgPool,
}
impl Application {
    pub async fn build(configuration: Settings) -> Result<Application, std::io::Error> {
        let db_pool = PgPool::connect_lazy_with(configuration.database.connect_options());
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let listener = TcpListener::bind(format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        ))?;
        let port = listener.local_addr().unwrap().port();
        Ok(Application {
            server: run(
                listener,
                db_pool.clone(),
                email_client,
                configuration.application.base_url,
            )?,
            port,
            db_pool,
        })
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn db_pool(&self) -> PgPool {
        self.db_pool.clone()
    }
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub struct ApplicationBaseUrl(pub String);
