use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use uuid::Uuid;

use yaht_common::protocol::{
    self, ClientMessage, ServerMessage, framed_transport, serialize_message,
};

use crate::handler;
use crate::server::SharedState;

pub struct ConnectionHandle {
    pub player_id: Uuid,
    pub player_name: String,
    pub tx: mpsc::Sender<ServerMessage>,
    pub room_id: Option<Uuid>,
    pub is_spectator: bool,
}

pub async fn handle_connection(stream: TcpStream, state: SharedState) -> anyhow::Result<()> {
    let mut transport = framed_transport(stream);

    // Step 1: Handshake -- expect Hello
    let hello: ClientMessage = match protocol::recv_message(&mut transport).await? {
        Some(msg) => msg,
        None => return Ok(()),
    };

    let (player_id, player_name) = match hello {
        ClientMessage::Hello {
            player_name,
            version,
        } => {
            tracing::info!(
                "Player '{}' connected (client version: {})",
                player_name,
                version
            );
            let id = Uuid::new_v4();
            protocol::send_message(
                &mut transport,
                &ServerMessage::Welcome {
                    player_id: id,
                    server_version: env!("CARGO_PKG_VERSION").to_string(),
                },
            )
            .await?;
            (id, player_name)
        }
        _ => {
            protocol::send_message(
                &mut transport,
                &ServerMessage::HandshakeError {
                    reason: "Expected Hello message".into(),
                },
            )
            .await?;
            return Ok(());
        }
    };

    // Step 2: Create mpsc channel for outbound messages
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(64);

    // Register connection
    {
        let handle = ConnectionHandle {
            player_id,
            player_name: player_name.clone(),
            tx: tx.clone(),
            room_id: None,
            is_spectator: false,
        };
        state.connections.write().await.insert(player_id, handle);
    }

    // Step 3: Split transport for independent read/write
    let (mut sink, mut stream) = transport.split();

    // Writer task: drains rx and writes to sink
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serialize_message(&msg) {
                Ok(bytes) => {
                    if sink.send(bytes.into()).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Step 4: Reader loop
    loop {
        match stream.next().await {
            Some(Ok(frame)) => {
                match protocol::deserialize_message::<ClientMessage>(&frame) {
                    Ok(msg) => {
                        if let Err(e) = handler::handle_message(player_id, msg, &state).await {
                            tracing::error!("Handler error for {}: {}", player_name, e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse message from {}: {}", player_name, e);
                    }
                }
            }
            Some(Err(e)) => {
                tracing::warn!("Read error from {}: {}", player_name, e);
                break;
            }
            None => {
                tracing::info!("Player '{}' disconnected", player_name);
                break;
            }
        }
    }

    // Cleanup
    handler::handle_disconnect(player_id, &state).await;
    write_task.abort();
    Ok(())
}
