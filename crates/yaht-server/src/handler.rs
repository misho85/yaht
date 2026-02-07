use rand::SeedableRng;
use uuid::Uuid;

use yaht_common::game::GamePhase;
use yaht_common::player::Player;
use yaht_common::protocol::{ClientMessage, ErrorCode, ServerMessage};
use crate::server::SharedState;

pub async fn handle_message(
    player_id: Uuid,
    msg: ClientMessage,
    state: &SharedState,
) -> anyhow::Result<()> {
    match msg {
        ClientMessage::ListRooms => {
            let lobby = state.lobby.read().await;
            let rooms = lobby.list_rooms();
            send_to_player(player_id, ServerMessage::RoomList { rooms }, state).await;
        }

        ClientMessage::CreateRoom {
            room_name,
            max_players,
            password,
        } => {
            let mut lobby = state.lobby.write().await;
            let room_id = lobby.create_room(room_name, max_players, player_id, password);

            // Update connection's room_id
            {
                let mut conns = state.connections.write().await;
                if let Some(conn) = conns.get_mut(&player_id) {
                    conn.room_id = Some(room_id);
                    conn.is_spectator = false;
                }
            }

            let conns = state.connections.read().await;
            if let Some(room) = lobby.get_room(&room_id) {
                let snapshot = room.snapshot(&conns);
                send_to_player(
                    player_id,
                    ServerMessage::RoomJoined {
                        room_id,
                        room_state: snapshot,
                    },
                    state,
                )
                .await;
            }
        }

        ClientMessage::JoinRoom { room_id, password } => {
            let mut lobby = state.lobby.write().await;
            let room = match lobby.get_room_mut(&room_id) {
                Some(r) => r,
                None => {
                    send_to_player(
                        player_id,
                        ServerMessage::Error {
                            code: ErrorCode::RoomNotFound,
                            message: "Room not found".into(),
                        },
                        state,
                    )
                    .await;
                    return Ok(());
                }
            };

            // Check password
            if !room.check_password(&password) {
                send_to_player(
                    player_id,
                    ServerMessage::Error {
                        code: ErrorCode::WrongPassword,
                        message: "Wrong room password".into(),
                    },
                    state,
                )
                .await;
                return Ok(());
            }

            if let Err(_) = room.add_player(player_id) {
                send_to_player(
                    player_id,
                    ServerMessage::Error {
                        code: ErrorCode::RoomFull,
                        message: "Room is full or game already started".into(),
                    },
                    state,
                )
                .await;
                return Ok(());
            }

            // Update connection
            {
                let mut conns = state.connections.write().await;
                if let Some(conn) = conns.get_mut(&player_id) {
                    conn.room_id = Some(room_id);
                    conn.is_spectator = false;
                }
            }

            let conns = state.connections.read().await;
            let player_name = conns
                .get(&player_id)
                .map(|c| c.player_name.clone())
                .unwrap_or_default();
            let snapshot = room.snapshot(&conns);
            let members = room.all_member_ids();
            drop(conns);
            drop(lobby);

            send_to_player(
                player_id,
                ServerMessage::RoomJoined {
                    room_id,
                    room_state: snapshot,
                },
                state,
            )
            .await;

            broadcast_to_list(
                &members,
                &ServerMessage::PlayerJoined {
                    player_id,
                    player_name,
                },
                state,
                Some(player_id),
            )
            .await;
        }

        ClientMessage::SpectateRoom { room_id } => {
            let mut lobby = state.lobby.write().await;
            let room = match lobby.get_room_mut(&room_id) {
                Some(r) => r,
                None => {
                    send_to_player(
                        player_id,
                        ServerMessage::Error {
                            code: ErrorCode::RoomNotFound,
                            message: "Room not found".into(),
                        },
                        state,
                    )
                    .await;
                    return Ok(());
                }
            };

            room.add_spectator(player_id);

            // Update connection
            {
                let mut conns = state.connections.write().await;
                if let Some(conn) = conns.get_mut(&player_id) {
                    conn.room_id = Some(room_id);
                    conn.is_spectator = true;
                }
            }

            let conns = state.connections.read().await;
            let player_name = conns
                .get(&player_id)
                .map(|c| c.player_name.clone())
                .unwrap_or_default();
            let snapshot = room.snapshot(&conns);
            let members = room.all_member_ids();

            // Also send current game state if in progress
            let game_snapshot = room.game.as_ref().map(|g| g.snapshot());
            drop(conns);
            drop(lobby);

            send_to_player(
                player_id,
                ServerMessage::RoomJoined {
                    room_id,
                    room_state: snapshot,
                },
                state,
            )
            .await;

            if let Some(gs) = game_snapshot {
                send_to_player(
                    player_id,
                    ServerMessage::GameState { game_state: gs },
                    state,
                )
                .await;
            }

            broadcast_to_list(
                &members,
                &ServerMessage::SpectatorJoined { player_name },
                state,
                Some(player_id),
            )
            .await;
        }

        ClientMessage::LeaveRoom => {
            handle_leave_room(player_id, state).await;
        }

        ClientMessage::StartGame => {
            let mut lobby = state.lobby.write().await;
            let conns = state.connections.read().await;

            let room_id = match conns.get(&player_id).and_then(|c| c.room_id) {
                Some(id) => id,
                None => return Ok(()),
            };

            let room = match lobby.get_room_mut(&room_id) {
                Some(r) => r,
                None => return Ok(()),
            };

            // Only host can start
            if room.host_id != player_id {
                send_to_player(
                    player_id,
                    ServerMessage::Error {
                        code: ErrorCode::InvalidAction,
                        message: "Only the host can start the game".into(),
                    },
                    state,
                )
                .await;
                return Ok(());
            }

            if room.player_ids.len() < 2 {
                send_to_player(
                    player_id,
                    ServerMessage::Error {
                        code: ErrorCode::NotEnoughPlayers,
                        message: "Need at least 2 players".into(),
                    },
                    state,
                )
                .await;
                return Ok(());
            }

            // Build Player objects from connections
            let players: Vec<Player> = room
                .player_ids
                .iter()
                .filter_map(|id| {
                    conns
                        .get(id)
                        .map(|c| Player::new(c.player_id, c.player_name.clone()))
                })
                .collect();

            let members = room.all_member_ids();

            if let Err(e) = room.start_game(players) {
                send_to_player(
                    player_id,
                    ServerMessage::Error {
                        code: ErrorCode::InternalError,
                        message: format!("Failed to start game: {}", e),
                    },
                    state,
                )
                .await;
                return Ok(());
            }

            let game_state = room.game.as_ref().unwrap().snapshot();
            drop(conns);
            drop(lobby);

            broadcast_to_list(
                &members,
                &ServerMessage::GameStarted { game_state },
                state,
                None,
            )
            .await;
        }

        ClientMessage::RollDice => {
            let mut lobby = state.lobby.write().await;
            let conns = state.connections.read().await;

            let room_id = match conns.get(&player_id).and_then(|c| c.room_id) {
                Some(id) => id,
                None => return Ok(()),
            };

            // Check spectator
            if conns.get(&player_id).map(|c| c.is_spectator).unwrap_or(false) {
                send_to_player(
                    player_id,
                    ServerMessage::Error {
                        code: ErrorCode::InvalidAction,
                        message: "Spectators cannot play".into(),
                    },
                    state,
                )
                .await;
                return Ok(());
            }

            let room = match lobby.get_room_mut(&room_id) {
                Some(r) => r,
                None => return Ok(()),
            };

            let game = match room.game.as_mut() {
                Some(g) => g,
                None => return Ok(()),
            };

            let mut rng = rand::rngs::StdRng::from_entropy();
            if let Err(e) = game.roll_dice(player_id, &mut rng) {
                let (code, message) = game_error_to_protocol(&e);
                drop(conns);
                drop(lobby);
                send_to_player(player_id, ServerMessage::Error { code, message }, state).await;
                return Ok(());
            }

            let turn = game.turn.as_ref().unwrap();
            let dice = turn.dice;
            let rolls_remaining = yaht_common::dice::MAX_ROLLS - turn.rolls_used;
            let members = room.all_member_ids();
            drop(conns);
            drop(lobby);

            broadcast_to_list(
                &members,
                &ServerMessage::DiceRolled {
                    dice,
                    rolls_remaining,
                },
                state,
                None,
            )
            .await;
        }

        ClientMessage::HoldDice { held } => {
            let mut lobby = state.lobby.write().await;
            let conns = state.connections.read().await;

            let room_id = match conns.get(&player_id).and_then(|c| c.room_id) {
                Some(id) => id,
                None => return Ok(()),
            };

            let room = match lobby.get_room_mut(&room_id) {
                Some(r) => r,
                None => return Ok(()),
            };

            let game = match room.game.as_mut() {
                Some(g) => g,
                None => return Ok(()),
            };

            if let Err(e) = game.hold_dice(player_id, held) {
                let (code, message) = game_error_to_protocol(&e);
                drop(conns);
                drop(lobby);
                send_to_player(player_id, ServerMessage::Error { code, message }, state).await;
                return Ok(());
            }

            let dice = game.turn.as_ref().unwrap().dice;
            let members = room.all_member_ids();
            drop(conns);
            drop(lobby);

            broadcast_to_list(
                &members,
                &ServerMessage::DiceHeld { dice },
                state,
                None,
            )
            .await;
        }

        ClientMessage::ScoreCategory { category } => {
            let mut lobby = state.lobby.write().await;
            let conns = state.connections.read().await;

            let room_id = match conns.get(&player_id).and_then(|c| c.room_id) {
                Some(id) => id,
                None => return Ok(()),
            };

            let room = match lobby.get_room_mut(&room_id) {
                Some(r) => r,
                None => return Ok(()),
            };

            {
                let game = match room.game.as_ref() {
                    Some(g) => g,
                    None => return Ok(()),
                };
                // Validate before mutation
                if !game.is_current_player(player_id) {
                    let (code, message) = game_error_to_protocol(&yaht_common::game::GameError::NotYourTurn);
                    drop(conns);
                    drop(lobby);
                    send_to_player(player_id, ServerMessage::Error { code, message }, state).await;
                    return Ok(());
                }
            }

            let game = room.game.as_mut().unwrap();
            let prev_player_id = game.current_player().id;

            let score = match game.score_category(player_id, category) {
                Ok(s) => s,
                Err(e) => {
                    let (code, message) = game_error_to_protocol(&e);
                    drop(conns);
                    drop(lobby);
                    send_to_player(player_id, ServerMessage::Error { code, message }, state).await;
                    return Ok(());
                }
            };

            let is_finished = game.phase == GamePhase::Finished;

            let mut messages = vec![
                ServerMessage::CategoryScored {
                    player_id: prev_player_id,
                    category,
                    score,
                },
                ServerMessage::TurnEnded {
                    player_id: prev_player_id,
                },
            ];

            if is_finished {
                let final_scores: Vec<(Uuid, String, u16)> = game
                    .players
                    .iter()
                    .map(|p| (p.id, p.name.clone(), p.scorecard.grand_total()))
                    .collect();
                let winner_id = game.winner().map(|w| w.id).unwrap_or(prev_player_id);
                messages.push(ServerMessage::GameOver {
                    final_scores,
                    winner_id,
                });
            } else {
                let next = game.current_player();
                messages.push(ServerMessage::TurnStarted {
                    player_id: next.id,
                    player_name: next.name.clone(),
                    turn_number: game.round,
                });
            }

            let members = room.all_member_ids();

            drop(conns);
            drop(lobby);

            for msg in &messages {
                broadcast_to_list(&members, msg, state, None).await;
            }
        }

        ClientMessage::Chat { message } => {
            let lobby = state.lobby.read().await;
            let conns = state.connections.read().await;

            let (room_id, player_name) = match conns.get(&player_id) {
                Some(c) => (c.room_id, c.player_name.clone()),
                None => return Ok(()),
            };

            let room_id = match room_id {
                Some(id) => id,
                None => return Ok(()),
            };

            let room = match lobby.get_room(&room_id) {
                Some(r) => r,
                None => return Ok(()),
            };

            let members = room.all_member_ids();
            let timestamp = chrono::Utc::now().timestamp();
            drop(conns);
            drop(lobby);

            broadcast_to_list(
                &members,
                &ServerMessage::ChatMessage {
                    sender_id: player_id,
                    sender_name: player_name,
                    message,
                    timestamp,
                },
                state,
                None,
            )
            .await;
        }

        ClientMessage::Ping => {
            send_to_player(player_id, ServerMessage::Pong, state).await;
        }

        ClientMessage::Disconnect => {
            handle_disconnect(player_id, state).await;
        }

        _ => {}
    }

    Ok(())
}

async fn handle_leave_room(player_id: Uuid, state: &SharedState) {
    let mut lobby = state.lobby.write().await;
    let conns = state.connections.read().await;

    let room_id = match conns.get(&player_id).and_then(|c| c.room_id) {
        Some(id) => id,
        None => return,
    };

    let player_name = conns
        .get(&player_id)
        .map(|c| c.player_name.clone())
        .unwrap_or_default();
    let is_spectator = conns
        .get(&player_id)
        .map(|c| c.is_spectator)
        .unwrap_or(false);

    if let Some(room) = lobby.get_room_mut(&room_id) {
        room.remove_player(&player_id);
        let members = room.all_member_ids();
        let is_empty = room.is_empty();
        drop(conns);

        if is_spectator {
            broadcast_to_list(
                &members,
                &ServerMessage::SpectatorLeft { player_name },
                state,
                None,
            )
            .await;
        } else {
            broadcast_to_list(
                &members,
                &ServerMessage::PlayerLeft {
                    player_id,
                    player_name,
                },
                state,
                None,
            )
            .await;
        }

        if is_empty {
            lobby.remove_room(&room_id);
        }
    } else {
        drop(conns);
    }

    // Clear room_id on connection
    let mut conns = state.connections.write().await;
    if let Some(conn) = conns.get_mut(&player_id) {
        conn.room_id = None;
        conn.is_spectator = false;
    }

    send_to_player(player_id, ServerMessage::RoomLeft, state).await;
}

pub async fn handle_disconnect(player_id: Uuid, state: &SharedState) {
    // Leave room first
    handle_leave_room(player_id, state).await;

    // Remove connection
    state.connections.write().await.remove(&player_id);

    // Prune empty rooms
    state.lobby.write().await.prune_empty_rooms();
}

async fn send_to_player(player_id: Uuid, msg: ServerMessage, state: &SharedState) {
    let conns = state.connections.read().await;
    if let Some(conn) = conns.get(&player_id) {
        let _ = conn.tx.send(msg).await;
    }
}

/// Broadcast a message to a list of player IDs. Optionally exclude one player.
async fn broadcast_to_list(
    member_ids: &[Uuid],
    msg: &ServerMessage,
    state: &SharedState,
    exclude: Option<Uuid>,
) {
    let conns = state.connections.read().await;
    for &id in member_ids {
        if Some(id) == exclude {
            continue;
        }
        if let Some(conn) = conns.get(&id) {
            let _ = conn.tx.send(msg.clone()).await;
        }
    }
}

fn game_error_to_protocol(e: &yaht_common::game::GameError) -> (ErrorCode, String) {
    use yaht_common::game::GameError;
    match e {
        GameError::NotYourTurn => (ErrorCode::NotYourTurn, e.to_string()),
        GameError::CategoryAlreadyScored => (ErrorCode::CategoryAlreadyScored, e.to_string()),
        GameError::GameNotInProgress => (ErrorCode::InvalidAction, e.to_string()),
        _ => (ErrorCode::InvalidAction, e.to_string()),
    }
}
