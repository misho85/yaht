use std::io;
use std::time::Duration;

use rand::{Rng, SeedableRng};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use uuid::Uuid;

use yaht_common::ai::{self, AiDifficulty};
use yaht_common::dice::MAX_ROLLS;
use yaht_common::game::{GamePhase, GameState, TurnPhase};
use yaht_common::player::Player;

use crate::input::{self, Action};
use crate::ui::game::{GameScreen, RollAnimation};
use crate::ui::help_popup;
use crate::ui::results::ResultsScreen;

#[derive(Debug)]
enum SoloScreen {
    Game(GameScreen),
    Results(ResultsScreen),
}

pub async fn run_solo(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    player_name: String,
    ai_count: u8,
) -> anyhow::Result<()> {
    let mut rng = rand::rngs::StdRng::from_entropy();

    // Create players: human + AI
    let human_id = Uuid::new_v4();
    let mut players = vec![Player::new(human_id, player_name)];

    let ai_names = ["Bot Alpha", "Bot Beta", "Bot Gamma", "Bot Delta", "Bot Epsilon"];
    let mut ai_ids = Vec::new();
    for i in 0..ai_count as usize {
        let id = Uuid::new_v4();
        ai_ids.push(id);
        players.push(Player::new(id, ai_names[i % ai_names.len()].to_string()));
    }

    let mut game = GameState::new(players);
    game.start_solo()?;

    let snapshot = game.snapshot();
    let mut game_screen = GameScreen::new(human_id, snapshot);
    game_screen.chat_messages = vec!["[System] Solo game started! You vs AI.".into()];

    let mut screen = SoloScreen::Game(game_screen);
    let mut running = true;
    let mut show_help = false;

    // Set up key event channel
    let (event_tx, mut event_rx) = mpsc::channel::<crossterm::event::KeyEvent>(64);
    tokio::spawn(async move {
        use crossterm::event::{Event, EventStream};
        use futures::StreamExt;
        let mut key_stream = EventStream::new();
        loop {
            if let Some(Ok(Event::Key(key))) = key_stream.next().await {
                if event_tx.send(key).await.is_err() {
                    break;
                }
            }
        }
    });

    // Initial turn notification
    let first_player = &game.players[game.current_player_index];
    if first_player.id == human_id {
        if let SoloScreen::Game(ref mut gs) = screen {
            gs.status_message = Some("Your turn! Press [R] to roll.".into());
        }
    }

    while running {
        // Draw
        terminal.draw(|frame| {
            match &screen {
                SoloScreen::Game(s) => s.draw(frame),
                SoloScreen::Results(s) => s.draw(frame),
            }
            if show_help {
                help_popup::draw_help_popup(frame);
            }
        })?;

        // Check if it's an AI's turn
        if game.phase == GamePhase::Playing {
            let current_id = game.current_player().id;
            if ai_ids.contains(&current_id) {
                // AI turn - process it with a small delay for visual effect
                tokio::time::sleep(Duration::from_millis(300)).await;
                process_ai_turn(&mut game, current_id, &mut rng, &mut screen, human_id, &ai_ids);
                continue;
            }
        }

        // Wait for human input with tick
        let key = tokio::select! {
            k = event_rx.recv() => {
                match k {
                    Some(key) => key,
                    None => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                // Tick for animations
                if let SoloScreen::Game(ref mut gs) = screen {
                    gs.tick();
                }
                continue;
            }
        };

        // Help dismiss
        if show_help {
            show_help = false;
            continue;
        }

        let chat_focused = matches!(&screen, SoloScreen::Game(g) if g.chat_focused);
        let app_screen = match &screen {
            SoloScreen::Game(g) => crate::app::Screen::Game(g.clone()),
            SoloScreen::Results(r) => crate::app::Screen::Results(r.clone()),
        };
        let action = input::map_key(key, &app_screen, chat_focused);

        if let Some(action) = action {
            match action {
                Action::Quit => {
                    running = false;
                }
                Action::ShowHelp => {
                    show_help = !show_help;
                }
                Action::RollDice => {
                    if game.phase == GamePhase::Playing && game.is_current_player(human_id) {
                        if let Ok(()) = game.roll_dice(human_id, &mut rng) {
                            let turn = game.turn.as_ref().unwrap();
                            let dice = turn.dice;
                            let rolls_remaining = MAX_ROLLS - turn.rolls_used;

                            if let SoloScreen::Game(ref mut gs) = screen {
                                gs.roll_animation = Some(RollAnimation::new(dice));
                                gs.rolls_remaining = rolls_remaining;
                                gs.game_state = game.snapshot();
                            }
                        }
                    }
                }
                Action::ToggleHold(idx) => {
                    if game.phase == GamePhase::Playing && game.is_current_player(human_id) {
                        if let SoloScreen::Game(ref mut gs) = screen {
                            gs.toggle_hold(idx);
                            let held = gs.get_held_array();
                            let _ = game.hold_dice(human_id, held);
                            if let Some(ref turn) = game.turn {
                                gs.dice = Some(turn.dice);
                            }
                        }
                    }
                }
                Action::ConfirmScore => {
                    if game.phase == GamePhase::Playing && game.is_current_player(human_id) {
                        if let SoloScreen::Game(ref gs) = screen {
                            if let Some(cat) = gs.selected_category() {
                                let prev_player = game.current_player().name.clone();
                                match game.score_category(human_id, cat) {
                                    Ok(score) => {
                                        if let SoloScreen::Game(ref mut gs) = screen {
                                            gs.score_flash = Some((cat, score, std::time::Instant::now()));
                                            gs.status_message = Some(format!(
                                                "{} scored {} for {}",
                                                prev_player, score, cat.display_name()
                                            ));
                                            gs.game_state = game.snapshot();

                                            if game.phase == GamePhase::Finished {
                                                let final_scores: Vec<(Uuid, String, u16)> = game
                                                    .players
                                                    .iter()
                                                    .map(|p| (p.id, p.name.clone(), p.scorecard.grand_total()))
                                                    .collect();
                                                let winner_id = game.winner().map(|w| w.id).unwrap_or(human_id);
                                                print!("\x07"); // Bell
                                                screen = SoloScreen::Results(ResultsScreen::new(final_scores, winner_id));
                                            } else {
                                                // Update for next turn
                                                update_game_screen_turn(&game, gs, human_id);
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        if let SoloScreen::Game(ref mut gs) = screen {
                                            gs.status_message = Some("Cannot score that category".into());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Action::NavigateUp => {
                    if let SoloScreen::Game(ref mut gs) = screen {
                        gs.select_prev_category();
                    }
                }
                Action::NavigateDown => {
                    if let SoloScreen::Game(ref mut gs) = screen {
                        gs.select_next_category();
                    }
                }
                Action::BackToLobby => {
                    running = false;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn process_ai_turn(
    game: &mut GameState,
    ai_id: Uuid,
    rng: &mut impl Rng,
    screen: &mut SoloScreen,
    human_id: Uuid,
    _ai_ids: &[Uuid],
) {
    let ai_name = game.current_player().name.clone();
    let difficulty = AiDifficulty::Hard;

    // Roll up to 3 times
    for roll_num in 0..3 {
        if game.roll_dice(ai_id, rng).is_err() {
            break;
        }

        let turn = game.turn.as_ref().unwrap();
        let dice = turn.dice;

        if let SoloScreen::Game(ref mut gs) = screen {
            gs.dice = Some(dice);
            gs.rolls_remaining = MAX_ROLLS - turn.rolls_used;
            gs.game_state = game.snapshot();
        }

        // Decide whether to reroll
        if roll_num < 2 {
            let scorecard = &game.current_player().scorecard;
            let held = ai::choose_holds(&dice, scorecard, difficulty, rng);

            // If AI wants to hold everything, stop rolling
            if held.iter().all(|&h| h) {
                break;
            }

            let _ = game.hold_dice(ai_id, held);
        }
    }

    // Choose category to score
    let turn = game.turn.as_ref().unwrap();
    let dice = turn.dice;
    let scorecard = &game.current_player().scorecard;
    let category = ai::choose_category(&dice, scorecard, difficulty, rng);

    match game.score_category(ai_id, category) {
        Ok(score) => {
            if let SoloScreen::Game(ref mut gs) = screen {
                gs.score_flash = Some((category, score, std::time::Instant::now()));
                gs.status_message = Some(format!(
                    "{} scored {} for {}",
                    ai_name, score, category.display_name()
                ));
                gs.game_state = game.snapshot();

                if game.phase == GamePhase::Finished {
                    let final_scores: Vec<(Uuid, String, u16)> = game
                        .players
                        .iter()
                        .map(|p| (p.id, p.name.clone(), p.scorecard.grand_total()))
                        .collect();
                    let winner_id = game.winner().map(|w| w.id).unwrap_or(ai_id);
                    print!("\x07"); // Bell
                    *screen = SoloScreen::Results(ResultsScreen::new(final_scores, winner_id));
                } else {
                    update_game_screen_turn(game, gs, human_id);
                }
            }
        }
        Err(_) => {
            // AI error - shouldn't happen, but try Chance as fallback
            if let Some(fallback) = game.current_player().scorecard.available_categories().first() {
                let _ = game.score_category(ai_id, *fallback);
            }
            if let SoloScreen::Game(ref mut gs) = screen {
                gs.game_state = game.snapshot();
                update_game_screen_turn(game, gs, human_id);
            }
        }
    }
}

fn update_game_screen_turn(game: &GameState, gs: &mut GameScreen, human_id: Uuid) {
    let current = &game.players[game.current_player_index];
    gs.current_turn_player_id = Some(current.id);
    gs.game_state.current_player_index = game.current_player_index;
    gs.round = game.round;
    gs.game_state.round = game.round;
    gs.rolls_remaining = MAX_ROLLS;
    gs.dice = None;
    gs.selected_category_index = 0;
    gs.game_state.turn_phase = Some(TurnPhase::WaitingForRoll);

    if current.id == human_id {
        print!("\x07"); // Bell for human's turn
        gs.status_message = Some(format!("Your turn! (round {})", game.round));
    }
}
