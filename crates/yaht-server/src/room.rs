use std::collections::HashMap;
use uuid::Uuid;

use yaht_common::game::{GameError, GameState};
use yaht_common::lobby::{RoomInfo, RoomInfoState};
use yaht_common::player::Player;
use yaht_common::protocol::{PlayerInfo, RoomSnapshot, RoomState};

use crate::connection::ConnectionHandle;

pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub max_players: u8,
    pub host_id: Uuid,
    pub player_ids: Vec<Uuid>,
    pub spectator_ids: Vec<Uuid>,
    pub game: Option<GameState>,
    pub password: Option<String>,
}

impl Room {
    pub fn new(id: Uuid, name: String, max_players: u8, host_id: Uuid, password: Option<String>) -> Self {
        Self {
            id,
            name,
            max_players: max_players.clamp(2, 6),
            host_id,
            player_ids: vec![host_id],
            spectator_ids: Vec::new(),
            game: None,
            password,
        }
    }

    pub fn check_password(&self, provided: &Option<String>) -> bool {
        match &self.password {
            None => true, // No password set, anyone can join
            Some(pass) => provided.as_ref().map(|p| p == pass).unwrap_or(false),
        }
    }

    pub fn add_player(&mut self, player_id: Uuid) -> Result<(), GameError> {
        if self.player_ids.len() as u8 >= self.max_players {
            return Err(GameError::TooManyPlayers);
        }
        if self.game.is_some() {
            return Err(GameError::GameNotInProgress);
        }
        if !self.player_ids.contains(&player_id) {
            self.player_ids.push(player_id);
        }
        Ok(())
    }

    pub fn add_spectator(&mut self, spectator_id: Uuid) {
        if !self.spectator_ids.contains(&spectator_id) {
            self.spectator_ids.push(spectator_id);
        }
    }

    pub fn remove_player(&mut self, player_id: &Uuid) {
        self.player_ids.retain(|id| id != player_id);
        self.spectator_ids.retain(|id| id != player_id);

        // If the host left, assign a new host
        if &self.host_id == player_id {
            if let Some(&new_host) = self.player_ids.first() {
                self.host_id = new_host;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.player_ids.is_empty() && self.spectator_ids.is_empty()
    }

    pub fn info(&self) -> RoomInfo {
        RoomInfo {
            room_id: self.id,
            room_name: self.name.clone(),
            player_count: self.player_ids.len() as u8,
            max_players: self.max_players,
            spectator_count: self.spectator_ids.len() as u8,
            state: if self.game.is_some() {
                RoomInfoState::InProgress
            } else {
                RoomInfoState::Waiting
            },
            has_password: self.password.is_some(),
        }
    }

    pub fn snapshot(&self, connections: &HashMap<Uuid, ConnectionHandle>) -> RoomSnapshot {
        let players = self
            .player_ids
            .iter()
            .filter_map(|id| {
                connections.get(id).map(|c| PlayerInfo {
                    id: c.player_id,
                    name: c.player_name.clone(),
                    connected: true,
                })
            })
            .collect();

        let spectators = self
            .spectator_ids
            .iter()
            .filter_map(|id| connections.get(id).map(|c| c.player_name.clone()))
            .collect();

        let state = if self.game.is_some() {
            RoomState::InGame
        } else {
            RoomState::WaitingForPlayers
        };

        RoomSnapshot {
            room_id: self.id,
            room_name: self.name.clone(),
            host_id: self.host_id,
            players,
            spectators,
            state,
            max_players: self.max_players,
        }
    }

    pub fn start_game(&mut self, players: Vec<Player>) -> Result<(), GameError> {
        let mut game = GameState::new(players);
        game.start()?;
        self.game = Some(game);
        Ok(())
    }

    /// Get all player + spectator IDs for broadcasting.
    pub fn all_member_ids(&self) -> Vec<Uuid> {
        self.player_ids
            .iter()
            .chain(self.spectator_ids.iter())
            .copied()
            .collect()
    }
}
