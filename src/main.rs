use std::net::TcpListener;

use zero2prod::configuration::get_configuration;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", configuration.application.port))
        .expect("Cannot bind address");
    zero2prod::startup::run(listener)?.await
}
