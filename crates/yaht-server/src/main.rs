mod connection;
mod handler;
mod lobby;
mod room;
mod server;

use std::net::SocketAddr;

use clap::Parser;

/// YAHT Server - Multiplayer Yahtzee game server
#[derive(Parser, Debug)]
#[command(name = "yaht-server", version, about)]
struct Args {
    /// Address to bind the server to
    #[arg(short, long, default_value = "0.0.0.0:9876")]
    bind: String,

    /// Maximum simultaneous connections allowed
    #[arg(short, long, default_value_t = 100)]
    max_connections: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "yaht_server=debug,yaht_common=debug".into()),
        )
        .init();

    let args = Args::parse();

    let addr: SocketAddr = args.bind.parse()?;

    tracing::info!("Starting yaht server on {} (max {} connections)", addr, args.max_connections);
    server::run(addr, args.max_connections).await
}
