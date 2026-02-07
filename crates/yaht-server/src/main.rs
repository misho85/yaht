mod connection;
mod handler;
mod lobby;
mod room;
mod server;

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "yaht_server=debug,yaht_common=debug".into()),
        )
        .init();

    let addr: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9876".to_string())
        .parse()?;

    tracing::info!("Starting yaht server on {}", addr);
    server::run(addr).await
}
