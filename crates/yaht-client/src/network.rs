use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use yaht_common::protocol::{
    ClientMessage, ServerMessage, framed_transport, serialize_message, deserialize_message,
};

/// Connect to the server and return channels for bidirectional communication.
pub async fn connect(
    addr: &str,
) -> anyhow::Result<(mpsc::Sender<ClientMessage>, mpsc::Receiver<ServerMessage>)> {
    let stream = TcpStream::connect(addr).await?;
    let transport = framed_transport(stream);
    let (mut sink, mut stream) = transport.split();

    let (client_tx, mut client_rx) = mpsc::channel::<ClientMessage>(64);
    let (server_tx, server_rx) = mpsc::channel::<ServerMessage>(64);

    // Writer task: client_rx -> TCP sink
    tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            match serialize_message(&msg) {
                Ok(bytes) => {
                    if sink.send(bytes.into()).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize client message: {}", e);
                }
            }
        }
    });

    // Reader task: TCP stream -> server_tx
    tokio::spawn(async move {
        while let Some(Ok(frame)) = stream.next().await {
            match deserialize_message::<ServerMessage>(&frame) {
                Ok(msg) => {
                    if server_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse server message: {}", e);
                }
            }
        }
    });

    Ok((client_tx, server_rx))
}
