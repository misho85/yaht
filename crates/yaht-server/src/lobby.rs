use std::collections::HashMap;
use uuid::Uuid;

use yaht_common::lobby::RoomInfo;

use crate::room::Room;

pub struct LobbyManager {
    pub rooms: HashMap<Uuid, Room>,
}

impl LobbyManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    pub fn create_room(&mut self, name: String, max_players: u8, host_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        self.rooms
            .insert(id, Room::new(id, name, max_players, host_id));
        id
    }

    pub fn list_rooms(&self) -> Vec<RoomInfo> {
        self.rooms.values().map(|r| r.info()).collect()
    }

    pub fn get_room(&self, id: &Uuid) -> Option<&Room> {
        self.rooms.get(id)
    }

    pub fn get_room_mut(&mut self, id: &Uuid) -> Option<&mut Room> {
        self.rooms.get_mut(id)
    }

    pub fn remove_room(&mut self, id: &Uuid) {
        self.rooms.remove(id);
    }

    pub fn prune_empty_rooms(&mut self) {
        self.rooms.retain(|_, r| !r.is_empty());
    }

    /// Find which room a player is in.
    pub fn find_player_room(&self, player_id: Uuid) -> Option<Uuid> {
        for (room_id, room) in &self.rooms {
            if room.player_ids.contains(&player_id) || room.spectator_ids.contains(&player_id) {
                return Some(*room_id);
            }
        }
        None
    }
}
