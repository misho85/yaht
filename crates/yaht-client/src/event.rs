use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

use yaht_common::protocol::ServerMessage;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Network(ServerMessage),
    Tick,
}

pub async fn event_loop(
    mut network_rx: mpsc::Receiver<ServerMessage>,
    event_tx: mpsc::Sender<AppEvent>,
) {
    let mut key_stream = EventStream::new();
    let mut tick_interval = tokio::time::interval(Duration::from_millis(250));

    loop {
        let event = tokio::select! {
            Some(Ok(Event::Key(key))) = key_stream.next() => {
                AppEvent::Key(key)
            }
            Some(msg) = network_rx.recv() => {
                AppEvent::Network(msg)
            }
            _ = tick_interval.tick() => {
                AppEvent::Tick
            }
        };

        if event_tx.send(event).await.is_err() {
            break;
        }
    }
}
