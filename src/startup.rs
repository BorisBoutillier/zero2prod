use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::{configuration::Settings, email_client::EmailClient, routes::*};

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .route("/", web::get().to(home))
            .route("/health_check", web::get().to(health_check))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
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
    pub async fn build(configuration: Settings) -> Result<Application, anyhow::Error> {
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
                configuration.application.hmac_secret,
                configuration.redis_uri,
            )
            .await?,
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
