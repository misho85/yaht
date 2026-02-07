use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: Uuid,
    pub room_name: String,
    pub player_count: u8,
    pub max_players: u8,
    pub spectator_count: u8,
    pub state: RoomInfoState,
    pub has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomInfoState {
    Waiting,
    InProgress,
    Finished,
}
