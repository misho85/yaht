use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use uuid::Uuid;

use crate::dice::DiceSet;
use crate::game::GameStateSnapshot;
use crate::lobby::RoomInfo;
use crate::scoring::Category;

// -- Framing --

pub type Transport = Framed<TcpStream, LengthDelimitedCodec>;

pub fn framed_transport(stream: TcpStream) -> Transport {
    LengthDelimitedCodec::builder()
        .max_frame_length(64 * 1024)
        .new_framed(stream)
}

// -- Client -> Server Messages --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // Handshake
    Hello {
        player_name: String,
        version: String,
    },

    // Lobby
    CreateRoom {
        room_name: String,
        max_players: u8,
    },
    JoinRoom {
        room_id: Uuid,
    },
    LeaveRoom,
    ListRooms,
    StartGame,

    // Spectator
    SpectateRoom {
        room_id: Uuid,
    },

    // Gameplay
    RollDice,
    HoldDice {
        held: [bool; 5],
    },
    ScoreCategory {
        category: Category,
    },

    // Chat
    Chat {
        message: String,
    },

    // Connection
    Ping,
    Disconnect,
}

// -- Server -> Client Messages --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    // Handshake
    Welcome {
        player_id: Uuid,
        server_version: String,
    },
    HandshakeError {
        reason: String,
    },

    // Lobby
    RoomList {
        rooms: Vec<RoomInfo>,
    },
    RoomJoined {
        room_id: Uuid,
        room_state: RoomSnapshot,
    },
    RoomUpdate {
        room_state: RoomSnapshot,
    },
    RoomLeft,

    // Game state
    GameStarted {
        game_state: GameStateSnapshot,
    },
    GameState {
        game_state: GameStateSnapshot,
    },
    TurnStarted {
        player_id: Uuid,
        player_name: String,
        turn_number: u8,
    },
    DiceRolled {
        dice: DiceSet,
        rolls_remaining: u8,
    },
    DiceHeld {
        dice: DiceSet,
    },
    CategoryScored {
        player_id: Uuid,
        category: Category,
        score: u16,
    },
    TurnEnded {
        player_id: Uuid,
    },
    GameOver {
        final_scores: Vec<(Uuid, String, u16)>,
        winner_id: Uuid,
    },

    // Chat
    ChatMessage {
        sender_id: Uuid,
        sender_name: String,
        message: String,
        timestamp: i64,
    },
    SystemMessage {
        message: String,
    },

    // Errors
    Error {
        code: ErrorCode,
        message: String,
    },

    // Connection
    Pong,
    PlayerJoined {
        player_id: Uuid,
        player_name: String,
    },
    PlayerLeft {
        player_id: Uuid,
        player_name: String,
    },
    SpectatorJoined {
        player_name: String,
    },
    SpectatorLeft {
        player_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    RoomFull,
    RoomNotFound,
    NotYourTurn,
    InvalidAction,
    CategoryAlreadyScored,
    GameAlreadyStarted,
    NotEnoughPlayers,
    NameTaken,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub room_id: Uuid,
    pub room_name: String,
    pub host_id: Uuid,
    pub players: Vec<PlayerInfo>,
    pub spectators: Vec<String>,
    pub state: RoomState,
    pub max_players: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomState {
    WaitingForPlayers,
    InGame,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: Uuid,
    pub name: String,
    pub connected: bool,
}

// -- Serialization helpers --

pub fn serialize_message<T: Serialize>(msg: &T) -> Result<Bytes, serde_json::Error> {
    let json = serde_json::to_vec(msg)?;
    Ok(Bytes::from(json))
}

pub fn deserialize_message<T: for<'de> Deserialize<'de>>(
    data: &[u8],
) -> Result<T, serde_json::Error> {
    serde_json::from_slice(data)
}

// -- Transport helpers --

pub async fn send_message<T: Serialize>(
    transport: &mut Transport,
    msg: &T,
) -> anyhow::Result<()> {
    let bytes = serialize_message(msg).map_err(|e| anyhow::anyhow!("serialize error: {}", e))?;
    transport
        .send(bytes.into())
        .await
        .map_err(|e| anyhow::anyhow!("send error: {}", e))
}

pub async fn recv_message<T: for<'de> Deserialize<'de>>(
    transport: &mut Transport,
) -> anyhow::Result<Option<T>> {
    match transport.next().await {
        Some(Ok(frame)) => {
            let msg = deserialize_message(&frame)
                .map_err(|e| anyhow::anyhow!("deserialize error: {}", e))?;
            Ok(Some(msg))
        }
        Some(Err(e)) => Err(anyhow::anyhow!("recv error: {}", e)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Hello {
            player_name: "Alice".into(),
            version: "0.1.0".into(),
        };
        let bytes = serialize_message(&msg).unwrap();
        let deserialized: ClientMessage = deserialize_message(&bytes).unwrap();
        match deserialized {
            ClientMessage::Hello {
                player_name,
                version,
            } => {
                assert_eq!(player_name, "Alice");
                assert_eq!(version, "0.1.0");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_message_serialization() {
        let id = Uuid::new_v4();
        let msg = ServerMessage::Welcome {
            player_id: id,
            server_version: "0.1.0".into(),
        };
        let bytes = serialize_message(&msg).unwrap();
        let deserialized: ServerMessage = deserialize_message(&bytes).unwrap();
        match deserialized {
            ServerMessage::Welcome {
                player_id,
                server_version,
            } => {
                assert_eq!(player_id, id);
                assert_eq!(server_version, "0.1.0");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_game_over_serialization() {
        let winner = Uuid::new_v4();
        let msg = ServerMessage::GameOver {
            final_scores: vec![
                (winner, "Alice".into(), 250),
                (Uuid::new_v4(), "Bob".into(), 200),
            ],
            winner_id: winner,
        };
        let bytes = serialize_message(&msg).unwrap();
        let deserialized: ServerMessage = deserialize_message(&bytes).unwrap();
        match deserialized {
            ServerMessage::GameOver {
                final_scores,
                winner_id,
            } => {
                assert_eq!(final_scores.len(), 2);
                assert_eq!(winner_id, winner);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_all_client_messages_serialize() {
        let room_id = Uuid::new_v4();
        let messages = vec![
            ClientMessage::Hello {
                player_name: "Test".into(),
                version: "0.1.0".into(),
            },
            ClientMessage::CreateRoom {
                room_name: "Room1".into(),
                max_players: 4,
            },
            ClientMessage::JoinRoom { room_id },
            ClientMessage::LeaveRoom,
            ClientMessage::ListRooms,
            ClientMessage::StartGame,
            ClientMessage::SpectateRoom { room_id },
            ClientMessage::RollDice,
            ClientMessage::HoldDice {
                held: [true, false, true, false, true],
            },
            ClientMessage::ScoreCategory {
                category: Category::Yahtzee,
            },
            ClientMessage::Chat {
                message: "hello".into(),
            },
            ClientMessage::Ping,
            ClientMessage::Disconnect,
        ];

        for msg in &messages {
            let bytes = serialize_message(msg).unwrap();
            let _: ClientMessage = deserialize_message(&bytes).unwrap();
        }
    }
}
