use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::configuration::get_configuration;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", configuration.application.port))
        .expect("Cannot bind address");
    zero2prod::startup::run(listener, connection_pool)?.await
}
