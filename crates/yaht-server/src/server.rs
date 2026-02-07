use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::connection::{self, ConnectionHandle};
use crate::lobby::LobbyManager;

pub struct ServerState {
    pub lobby: RwLock<LobbyManager>,
    pub connections: RwLock<HashMap<Uuid, ConnectionHandle>>,
    pub max_connections: usize,
}

pub type SharedState = Arc<ServerState>;

pub async fn run(addr: SocketAddr, max_connections: usize) -> anyhow::Result<()> {
    let state: SharedState = Arc::new(ServerState {
        lobby: RwLock::new(LobbyManager::new()),
        connections: RwLock::new(HashMap::new()),
        max_connections,
    });

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;

        // Enforce max connections
        let conn_count = state.connections.read().await.len();
        if conn_count >= state.max_connections {
            tracing::warn!(
                "Rejecting connection from {} (max {} reached)",
                peer_addr,
                state.max_connections
            );
            drop(stream);
            continue;
        }

        tracing::info!("New connection from {} ({}/{})", peer_addr, conn_count + 1, state.max_connections);

        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = connection::handle_connection(stream, state).await {
                tracing::warn!("Connection error from {}: {}", peer_addr, e);
            }
        });
    }
}
