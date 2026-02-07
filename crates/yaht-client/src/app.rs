use std::io;

use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use uuid::Uuid;

use yaht_common::dice::MAX_ROLLS;
use yaht_common::protocol::{ClientMessage, ServerMessage};

use crate::event::{self, AppEvent};
use crate::input::{self, Action};
use crate::network;
use crate::ui::connect::ConnectScreen;
use crate::ui::game::GameScreen;
use crate::ui::lobby::LobbyScreen;
use crate::ui::results::ResultsScreen;

#[derive(Debug)]
pub enum Screen {
    Connect(ConnectScreen),
    Lobby(LobbyScreen),
    Game(GameScreen),
    Results(ResultsScreen),
}

pub async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut screen = Screen::Connect(ConnectScreen::new());
    let mut player_id: Option<Uuid> = None;
    let mut player_name = String::new();
    let mut network_tx: Option<mpsc::Sender<ClientMessage>> = None;
    let mut running = true;

    let (local_event_tx, mut event_rx) = mpsc::channel::<AppEvent>(64);

    let local_tx = local_event_tx.clone();
    let mut local_event_handle = Some(tokio::spawn(async move {
        use crossterm::event::{Event, EventStream};
        use futures::StreamExt;

        let mut key_stream = EventStream::new();
        loop {
            if let Some(Ok(Event::Key(key))) = key_stream.next().await {
                if local_tx.send(AppEvent::Key(key)).await.is_err() {
                    break;
                }
            }
        }
    }));

    while running {
        terminal.draw(|frame| match &screen {
            Screen::Connect(s) => s.draw(frame),
            Screen::Lobby(s) => s.draw(frame),
            Screen::Game(s) => s.draw(frame),
            Screen::Results(s) => s.draw(frame),
        })?;

        let event = match event_rx.recv().await {
            Some(e) => e,
            None => break,
        };

        let chat_focused = matches!(&screen, Screen::Game(g) if g.chat_focused);
        let action = match &event {
            AppEvent::Key(key) => input::map_key(*key, &screen, chat_focused),
            AppEvent::Network(msg) => {
                let outbound = handle_server_message(msg.clone(), &mut screen, &mut player_id, &mut player_name);
                if let Some(ref tx) = network_tx {
                    for out_msg in outbound {
                        let _ = tx.send(out_msg).await;
                    }
                }
                None
            }
            AppEvent::Tick => None,
        };

        if let Some(action) = action {
            match action {
                Action::Quit => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx.send(ClientMessage::Disconnect).await;
                    }
                    running = false;
                }

                Action::TypeChar(c) => match &mut screen {
                    Screen::Connect(s) => s.type_char(c),
                    Screen::Game(s) if s.chat_focused => s.chat_input.push(c),
                    _ => {}
                },
                Action::Backspace => match &mut screen {
                    Screen::Connect(s) => s.backspace(),
                    Screen::Game(s) if s.chat_focused => {
                        s.chat_input.pop();
                    }
                    _ => {}
                },
                Action::SwitchField => {
                    if let Screen::Connect(s) = &mut screen {
                        s.switch_field();
                    }
                }
                Action::Submit => {
                    if let Screen::Connect(s) = &mut screen {
                        if s.name.is_empty() {
                            s.error_message = Some("Please enter a name".into());
                            continue;
                        }
                        s.connecting = true;
                        s.error_message = None;
                        player_name = s.name.clone();

                        match network::connect(&s.host).await {
                            Ok((tx, rx)) => {
                                let _ = tx
                                    .send(ClientMessage::Hello {
                                        player_name: s.name.clone(),
                                        version: env!("CARGO_PKG_VERSION").to_string(),
                                    })
                                    .await;

                                network_tx = Some(tx);

                                if let Some(handle) = local_event_handle.take() {
                                    handle.abort();
                                }

                                let (full_event_tx, full_event_rx) =
                                    mpsc::channel::<AppEvent>(64);
                                event_rx = full_event_rx;

                                tokio::spawn(event::event_loop(rx, full_event_tx));
                            }
                            Err(e) => {
                                s.connecting = false;
                                s.error_message = Some(format!("Connection failed: {}", e));
                            }
                        }
                    }
                }

                Action::RefreshRooms => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx.send(ClientMessage::ListRooms).await;
                    }
                }
                Action::CreateRoom => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx
                            .send(ClientMessage::CreateRoom {
                                room_name: format!("{}'s room", player_name),
                                max_players: 6,
                            })
                            .await;
                    }
                }
                Action::JoinSelected => {
                    if let Screen::Lobby(s) = &screen {
                        if let Some(room_id) = s.selected_room_id() {
                            if let Some(ref tx) = network_tx {
                                let _ = tx.send(ClientMessage::JoinRoom { room_id }).await;
                            }
                        }
                    }
                }
                Action::SpectateSelected => {
                    if let Screen::Lobby(s) = &screen {
                        if let Some(room_id) = s.selected_room_id() {
                            if let Some(ref tx) = network_tx {
                                let _ =
                                    tx.send(ClientMessage::SpectateRoom { room_id }).await;
                            }
                        }
                    }
                }
                Action::NavigateUp => match &mut screen {
                    Screen::Lobby(s) => s.select_prev(),
                    Screen::Game(s) => s.select_prev_category(),
                    _ => {}
                },
                Action::NavigateDown => match &mut screen {
                    Screen::Lobby(s) => s.select_next(),
                    Screen::Game(s) => s.select_next_category(),
                    _ => {}
                },

                Action::RollDice => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx.send(ClientMessage::RollDice).await;
                    }
                }
                Action::ToggleHold(idx) => {
                    if let Screen::Game(s) = &mut screen {
                        if let Some(ref pid) = player_id {
                            if s.is_my_turn(pid) {
                                s.toggle_hold(idx);
                                if let Some(ref tx) = network_tx {
                                    let held = s.get_held_array();
                                    let _ = tx.send(ClientMessage::HoldDice { held }).await;
                                }
                            }
                        }
                    }
                }
                Action::ConfirmScore => {
                    if let Screen::Game(s) = &screen {
                        if let Some(cat) = s.selected_category() {
                            if let Some(ref tx) = network_tx {
                                let _ = tx
                                    .send(ClientMessage::ScoreCategory { category: cat })
                                    .await;
                            }
                        }
                    }
                }
                Action::ToggleChatFocus => {
                    if let Screen::Game(s) = &mut screen {
                        s.chat_focused = !s.chat_focused;
                    }
                }
                Action::SendChat => {
                    if let Screen::Game(s) = &mut screen {
                        if !s.chat_input.is_empty() {
                            let msg = s.chat_input.drain(..).collect::<String>();
                            if let Some(ref tx) = network_tx {
                                let _ = tx.send(ClientMessage::Chat { message: msg }).await;
                            }
                        }
                    }
                }
                Action::StartGame => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx.send(ClientMessage::StartGame).await;
                    }
                }
                Action::LeaveRoom => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx.send(ClientMessage::LeaveRoom).await;
                        let _ = tx.send(ClientMessage::ListRooms).await;
                    }
                    if let Screen::Lobby(s) = &mut screen {
                        s.joined_room = None;
                        s.status_message = None;
                    }
                }

                Action::BackToLobby => {
                    if let Some(ref tx) = network_tx {
                        let _ = tx.send(ClientMessage::LeaveRoom).await;
                        let _ = tx.send(ClientMessage::ListRooms).await;
                    }
                    let mut lobby = LobbyScreen::new(player_name.clone());
                    lobby.player_id = player_id;
                    screen = Screen::Lobby(lobby);
                }

                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_server_message(
    msg: ServerMessage,
    screen: &mut Screen,
    player_id: &mut Option<Uuid>,
    player_name: &mut String,
) -> Vec<ClientMessage> {
    let mut outbound = Vec::new();

    match msg {
        ServerMessage::Welcome {
            player_id: pid,
            server_version: _,
        } => {
            *player_id = Some(pid);
            let mut lobby = LobbyScreen::new(player_name.clone());
            lobby.player_id = Some(pid);
            *screen = Screen::Lobby(lobby);
            outbound.push(ClientMessage::ListRooms);
        }

        ServerMessage::HandshakeError { reason } => {
            if let Screen::Connect(s) = screen {
                s.connecting = false;
                s.error_message = Some(reason);
            }
        }

        ServerMessage::RoomList { rooms } => {
            if let Screen::Lobby(s) = screen {
                s.rooms = rooms;
                if s.table_state.selected().is_none() && !s.rooms.is_empty() {
                    s.table_state.select(Some(0));
                }
            }
        }

        ServerMessage::RoomJoined {
            room_id: _,
            room_state,
        } => {
            if let Screen::Lobby(s) = screen {
                s.status_message = None;
                s.joined_room = Some(room_state);
            }
        }

        ServerMessage::RoomUpdate { room_state } => {
            if let Screen::Lobby(s) = screen {
                if s.joined_room.is_some() {
                    s.joined_room = Some(room_state);
                }
            }
        }

        ServerMessage::RoomLeft => {
            if let Screen::Lobby(s) = screen {
                s.joined_room = None;
                outbound.push(ClientMessage::ListRooms);
            }
        }

        ServerMessage::GameStarted { game_state } => {
            if let Some(pid) = player_id {
                *screen = Screen::Game(GameScreen::new(*pid, game_state));
            }
        }

        ServerMessage::GameState { game_state } => {
            if let Screen::Game(s) = screen {
                s.update_from_snapshot(game_state);
            } else if let Some(pid) = player_id {
                *screen = Screen::Game(GameScreen::new(*pid, game_state));
            }
        }

        ServerMessage::DiceRolled {
            dice,
            rolls_remaining,
        } => {
            if let Screen::Game(s) = screen {
                s.dice = Some(dice);
                s.rolls_remaining = rolls_remaining;
            }
        }

        ServerMessage::DiceHeld { dice } => {
            if let Screen::Game(s) = screen {
                s.dice = Some(dice);
            }
        }

        ServerMessage::CategoryScored {
            player_id: _,
            category,
            score,
        } => {
            if let Screen::Game(s) = screen {
                s.status_message = Some(format!(
                    "Scored {} for {}",
                    score,
                    category.display_name()
                ));
            }
        }

        ServerMessage::TurnStarted {
            player_id: turn_pid,
            player_name: turn_name,
            turn_number,
        } => {
            if let Screen::Game(s) = screen {
                s.current_turn_player_id = Some(turn_pid);
                s.round = turn_number;
                s.rolls_remaining = MAX_ROLLS;
                s.dice = None;
                s.status_message =
                    Some(format!("{}'s turn (round {})", turn_name, turn_number));
            }
        }

        ServerMessage::TurnEnded { player_id: _ } => {}

        ServerMessage::GameOver {
            final_scores,
            winner_id,
        } => {
            *screen = Screen::Results(ResultsScreen::new(final_scores, winner_id));
        }

        ServerMessage::ChatMessage {
            sender_id: _,
            sender_name,
            message,
            timestamp: _,
        } => {
            if let Screen::Game(s) = screen {
                s.chat_messages
                    .push(format!("{}: {}", sender_name, message));
            }
        }

        ServerMessage::SystemMessage { message } => {
            if let Screen::Game(s) = screen {
                s.chat_messages.push(format!("[System] {}", message));
            }
        }

        ServerMessage::Error { code: _, message } => match screen {
            Screen::Lobby(s) => {
                s.status_message = Some(format!("Error: {}", message));
            }
            Screen::Game(s) => {
                s.status_message = Some(format!("Error: {}", message));
            }
            _ => {}
        },

        ServerMessage::PlayerJoined {
            player_id: joined_pid,
            player_name: name,
        } => {
            match screen {
                Screen::Lobby(s) => {
                    if let Some(ref mut room) = s.joined_room {
                        room.players.push(yaht_common::protocol::PlayerInfo {
                            id: joined_pid,
                            name: name.clone(),
                            connected: true,
                        });
                        s.status_message = Some(format!("{} joined", name));
                    }
                }
                Screen::Game(s) => {
                    s.chat_messages
                        .push(format!("[System] {} joined the game", name));
                }
                _ => {}
            }
        }

        ServerMessage::PlayerLeft {
            player_id: left_pid,
            player_name: name,
        } => {
            match screen {
                Screen::Lobby(s) => {
                    if let Some(ref mut room) = s.joined_room {
                        room.players.retain(|p| p.id != left_pid);
                        s.status_message = Some(format!("{} left", name));
                    }
                }
                Screen::Game(s) => {
                    s.chat_messages
                        .push(format!("[System] {} left the game", name));
                }
                _ => {}
            }
        }

        ServerMessage::SpectatorJoined { player_name: name } => {
            match screen {
                Screen::Lobby(s) => {
                    if let Some(ref mut room) = s.joined_room {
                        room.spectators.push(name.clone());
                    }
                }
                Screen::Game(s) => {
                    s.chat_messages
                        .push(format!("[System] {} is spectating", name));
                }
                _ => {}
            }
        }

        ServerMessage::SpectatorLeft { player_name: name } => {
            match screen {
                Screen::Lobby(s) => {
                    if let Some(ref mut room) = s.joined_room {
                        room.spectators.retain(|n| n != &name);
                    }
                }
                Screen::Game(s) => {
                    s.chat_messages
                        .push(format!("[System] {} stopped spectating", name));
                }
                _ => {}
            }
        }

        ServerMessage::Pong => {}
    }

    outbound
}
