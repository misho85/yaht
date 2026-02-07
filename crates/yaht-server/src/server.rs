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
}

pub type SharedState = Arc<ServerState>;

pub async fn run(addr: SocketAddr) -> anyhow::Result<()> {
    let state: SharedState = Arc::new(ServerState {
        lobby: RwLock::new(LobbyManager::new()),
        connections: RwLock::new(HashMap::new()),
    });

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::info!("New connection from {}", peer_addr);

        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = connection::handle_connection(stream, state).await {
                tracing::warn!("Connection error from {}: {}", peer_addr, e);
            }
        });
    }
}
