use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use uuid::Uuid;

use yaht_common::dice::{Die, DiceSet, MAX_ROLLS};
use yaht_common::game::{GameStateSnapshot, TurnPhase};
use yaht_common::scoring::Category;

use super::dice_widget;
use super::scoreboard_widget;

const ROLL_ANIM_DURATION: Duration = Duration::from_millis(600);
const ROLL_ANIM_FRAME_INTERVAL: Duration = Duration::from_millis(60);
const SCORE_FLASH_DURATION: Duration = Duration::from_millis(1500);

/// Dice rolling animation state
#[derive(Debug, Clone)]
pub struct RollAnimation {
    pub final_dice: DiceSet,
    pub started_at: Instant,
    pub last_frame: Instant,
    pub current_display: [u8; 5],
}

impl RollAnimation {
    pub fn new(final_dice: DiceSet) -> Self {
        let now = Instant::now();
        Self {
            final_dice,
            started_at: now,
            last_frame: now,
            current_display: [1, 1, 1, 1, 1],
        }
    }

    pub fn is_done(&self) -> bool {
        self.started_at.elapsed() >= ROLL_ANIM_DURATION
    }

    /// Advance animation frame, returns true if display changed
    pub fn tick(&mut self) -> bool {
        if self.is_done() {
            return false;
        }
        if self.last_frame.elapsed() < ROLL_ANIM_FRAME_INTERVAL {
            return false;
        }
        self.last_frame = Instant::now();
        let mut rng = rand::rngs::StdRng::from_entropy();
        for i in 0..5 {
            if !self.final_dice.dice[i].held {
                self.current_display[i] = rng.gen_range(1..=6);
            } else {
                self.current_display[i] = self.final_dice.dice[i].value;
            }
        }
        true
    }

    /// Get dice to display during animation
    pub fn display_dice(&self) -> [Die; 5] {
        let mut dice = [Die { value: 1, held: false }; 5];
        for i in 0..5 {
            dice[i] = Die {
                value: self.current_display[i],
                held: self.final_dice.dice[i].held,
            };
        }
        dice
    }
}

#[derive(Debug, Clone)]
pub struct GameScreen {
    pub game_state: GameStateSnapshot,
    pub my_player_id: Uuid,
    pub dice: Option<DiceSet>,
    pub rolls_remaining: u8,
    pub round: u8,
    pub current_turn_player_id: Option<Uuid>,
    pub chat_messages: Vec<String>,
    pub chat_input: String,
    pub chat_focused: bool,
    pub selected_category_index: usize,
    pub status_message: Option<String>,
    // Animation state
    pub roll_animation: Option<RollAnimation>,
    pub score_flash: Option<(Category, u16, Instant)>,
}

impl GameScreen {
    pub fn new(my_player_id: Uuid, game_state: GameStateSnapshot) -> Self {
        let current_pid = game_state
            .players
            .get(game_state.current_player_index)
            .map(|p| p.id);
        let round = game_state.round;
        let dice = game_state.dice;
        let rolls_remaining = MAX_ROLLS - game_state.rolls_used;

        Self {
            game_state,
            my_player_id,
            dice,
            rolls_remaining,
            round,
            current_turn_player_id: current_pid,
            chat_messages: vec!["[System] Game started!".into()],
            chat_input: String::new(),
            chat_focused: false,
            selected_category_index: 0,
            status_message: None,
            roll_animation: None,
            score_flash: None,
        }
    }

    pub fn update_from_snapshot(&mut self, snapshot: GameStateSnapshot) {
        self.dice = snapshot.dice;
        self.rolls_remaining = MAX_ROLLS - snapshot.rolls_used;
        self.round = snapshot.round;
        self.current_turn_player_id = snapshot
            .players
            .get(snapshot.current_player_index)
            .map(|p| p.id);
        self.game_state = snapshot;
    }

    /// Called on each tick to advance animations
    pub fn tick(&mut self) {
        // Advance dice rolling animation
        if let Some(ref mut anim) = self.roll_animation {
            if anim.is_done() {
                // Animation finished, set final dice
                self.dice = Some(anim.final_dice);
                self.roll_animation = None;
            } else {
                anim.tick();
            }
        }

        // Clear expired score flash
        if let Some((_, _, started)) = self.score_flash {
            if started.elapsed() >= SCORE_FLASH_DURATION {
                self.score_flash = None;
            }
        }
    }

    pub fn is_my_turn(&self, my_id: &Uuid) -> bool {
        self.current_turn_player_id.as_ref() == Some(my_id)
    }

    pub fn selected_category(&self) -> Option<Category> {
        let me = self
            .game_state
            .players
            .iter()
            .find(|p| p.id == self.my_player_id)?;
        let available = me.scorecard.available_categories();
        available.get(self.selected_category_index).copied()
    }

    pub fn select_next_category(&mut self) {
        let count = self
            .game_state
            .players
            .iter()
            .find(|p| p.id == self.my_player_id)
            .map(|p| p.scorecard.available_categories().len())
            .unwrap_or(0);
        if count > 0 {
            self.selected_category_index = (self.selected_category_index + 1) % count;
        }
    }

    pub fn select_prev_category(&mut self) {
        let count = self
            .game_state
            .players
            .iter()
            .find(|p| p.id == self.my_player_id)
            .map(|p| p.scorecard.available_categories().len())
            .unwrap_or(0);
        if count > 0 {
            self.selected_category_index = if self.selected_category_index == 0 {
                count - 1
            } else {
                self.selected_category_index - 1
            };
        }
    }

    pub fn toggle_hold(&mut self, idx: usize) {
        if idx >= 5 {
            return;
        }
        if let Some(ref mut dice) = self.dice {
            dice.dice[idx].held = !dice.dice[idx].held;
        }
    }

    pub fn get_held_array(&self) -> [bool; 5] {
        self.dice
            .as_ref()
            .map(|d| {
                [
                    d.dice[0].held,
                    d.dice[1].held,
                    d.dice[2].held,
                    d.dice[3].held,
                    d.dice[4].held,
                ]
            })
            .unwrap_or([false; 5])
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(9), // Dice
                Constraint::Length(4), // Actions
                Constraint::Min(5),   // Chat
            ])
            .split(main_chunks[0]);

        self.draw_title_bar(frame, left_chunks[0]);
        self.draw_dice_area(frame, left_chunks[1]);
        self.draw_action_bar(frame, left_chunks[2]);
        self.draw_chat_panel(frame, left_chunks[3]);
        self.draw_scoreboard(frame, main_chunks[1]);
    }

    fn draw_title_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let current_name = self
            .game_state
            .players
            .get(self.game_state.current_player_index)
            .map(|p| p.name.as_str())
            .unwrap_or("?");

        let is_my_turn = self.is_my_turn(&self.my_player_id);
        let turn_color = if is_my_turn {
            Color::Rgb(100, 255, 150)
        } else {
            Color::Rgb(180, 180, 200)
        };

        let title = Line::from(vec![
            Span::styled(
                " YAHT ",
                Style::default()
                    .fg(Color::Rgb(255, 220, 50))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" Round {}/{}", self.round, self.game_state.total_rounds),
                Style::default().fg(Color::Rgb(150, 150, 170)),
            ),
            Span::styled("  |  ", Style::default().fg(Color::Rgb(80, 80, 100))),
            Span::styled("Turn: ", Style::default().fg(Color::Rgb(150, 150, 170))),
            Span::styled(
                current_name,
                Style::default()
                    .fg(turn_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), area);
    }

    fn draw_dice_area(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // Check if we're in a rolling animation
        if let Some(ref anim) = self.roll_animation {
            let anim_dice = anim.display_dice();
            let lines = dice_widget::render_dice_row_animated(&anim_dice, true);
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(100, 200, 255)))
                    .title(" Dice - Rolling... ")
                    .title_style(
                        Style::default()
                            .fg(Color::Rgb(100, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    ),
            );
            frame.render_widget(paragraph, area);
        } else if let Some(ref dice) = self.dice {
            let lines = dice_widget::render_dice_row(&dice.dice);
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
                    .title(" Dice ")
                    .title_style(Style::default().fg(Color::Rgb(180, 180, 200))),
            );
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new("  Waiting for roll...")
                .style(Style::default().fg(Color::Rgb(100, 100, 120)))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Rgb(60, 60, 80)))
                        .title(" Dice ")
                        .title_style(Style::default().fg(Color::Rgb(120, 120, 140))),
                );
            frame.render_widget(paragraph, area);
        }
    }

    fn draw_action_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let is_my_turn = self.is_my_turn(&self.my_player_id);
        let is_rolling = self.roll_animation.is_some();
        let can_roll = is_my_turn
            && !is_rolling
            && self.rolls_remaining > 0
            && matches!(
                self.game_state.turn_phase.as_ref(),
                Some(TurnPhase::WaitingForRoll) | Some(TurnPhase::Rolling { .. })
            );
        let can_score = is_my_turn
            && !is_rolling
            && matches!(
                self.game_state.turn_phase.as_ref(),
                Some(TurnPhase::Rolling { .. }) | Some(TurnPhase::MustScore)
            );

        let mut lines = Vec::new();

        if is_rolling {
            lines.push(Line::from(Span::styled(
                "  Rolling dice...",
                Style::default()
                    .fg(Color::Rgb(100, 200, 255))
                    .add_modifier(Modifier::BOLD),
            )));
        } else if is_my_turn {
            let mut spans = vec![Span::raw("  ")];
            if can_roll {
                spans.push(Span::styled(
                    "[R]",
                    Style::default()
                        .fg(Color::Rgb(100, 255, 150))
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!(" Roll ({} left)  ", self.rolls_remaining),
                    Style::default().fg(Color::Rgb(150, 150, 170)),
                ));
            }
            spans.push(Span::styled(
                "[1-5]",
                Style::default().fg(Color::Rgb(100, 200, 255)),
            ));
            spans.push(Span::styled(
                " Hold  ",
                Style::default().fg(Color::Rgb(150, 150, 170)),
            ));
            if can_score {
                spans.push(Span::styled(
                    "[S]",
                    Style::default()
                        .fg(Color::Rgb(200, 150, 255))
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    " Score  ",
                    Style::default().fg(Color::Rgb(150, 150, 170)),
                ));
            }
            spans.push(Span::styled(
                "[C]",
                Style::default().fg(Color::Rgb(100, 180, 255)),
            ));
            spans.push(Span::styled(
                " Chat",
                Style::default().fg(Color::Rgb(150, 150, 170)),
            ));
            lines.push(Line::from(spans));
        } else {
            lines.push(Line::from(Span::styled(
                "  Waiting for other player's turn...",
                Style::default().fg(Color::Rgb(100, 100, 120)),
            )));
        }

        if let Some(ref msg) = self.status_message {
            let style = if let Some((_, _, started)) = self.score_flash {
                let elapsed = started.elapsed().as_millis();
                let blink = (elapsed / 200) % 2 == 0;
                if blink {
                    Style::default()
                        .fg(Color::Rgb(255, 220, 50))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(100, 255, 150))
                }
            } else {
                Style::default().fg(Color::Rgb(100, 200, 255))
            };
            lines.push(Line::from(Span::styled(format!("  {}", msg), style)));
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_chat_panel(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let inner_height = area.height.saturating_sub(2) as usize;
        let skip = if self.chat_messages.len() > inner_height.saturating_sub(1) {
            self.chat_messages.len() - (inner_height.saturating_sub(1))
        } else {
            0
        };

        let mut lines: Vec<Line> = self.chat_messages[skip..]
            .iter()
            .map(|msg| {
                if msg.starts_with("[System]") {
                    Line::from(Span::styled(
                        format!("  {}", msg),
                        Style::default().fg(Color::Rgb(100, 100, 120)),
                    ))
                } else if let Some(colon_pos) = msg.find(':') {
                    let (name, rest) = msg.split_at(colon_pos);
                    Line::from(vec![
                        Span::styled(
                            format!("  {}", name),
                            Style::default()
                                .fg(Color::Rgb(100, 200, 255))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            rest.to_string(),
                            Style::default().fg(Color::Rgb(200, 200, 220)),
                        ),
                    ])
                } else {
                    Line::from(Span::styled(
                        format!("  {}", msg),
                        Style::default().fg(Color::Rgb(200, 200, 220)),
                    ))
                }
            })
            .collect();

        let prefix = if self.chat_focused { "  > " } else { "  " };
        let style = if self.chat_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Rgb(80, 80, 100))
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, self.chat_input),
            style,
        )));

        let (border_style, title_style) = if self.chat_focused {
            (
                Style::default().fg(Color::Rgb(100, 180, 255)),
                Style::default()
                    .fg(Color::Rgb(100, 180, 255))
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Color::Rgb(60, 60, 80)),
                Style::default().fg(Color::Rgb(120, 120, 140)),
            )
        };

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Chat ")
                .title_style(title_style),
        );
        frame.render_widget(paragraph, area);

        if self.chat_focused {
            let cursor_x = area.x + 4 + self.chat_input.len() as u16;
            let cursor_y = area.y + area.height - 2;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn draw_scoreboard(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let is_my_turn = self.is_my_turn(&self.my_player_id);

        let selected_all_idx = if is_my_turn {
            if let Some(me) = self
                .game_state
                .players
                .iter()
                .find(|p| p.id == self.my_player_id)
            {
                let available = me.scorecard.available_categories();
                available
                    .get(self.selected_category_index)
                    .and_then(|cat| Category::ALL.iter().position(|c| c == cat))
            } else {
                None
            }
        } else {
            None
        };

        // Determine which dice to show potential scores for
        let active_dice = if self.roll_animation.is_none() {
            self.dice.as_ref().map(|d| d.values())
        } else {
            None
        };

        // Get the flash category for highlighting
        let flash_cat = self.score_flash.as_ref().and_then(|(cat, score, started)| {
            if started.elapsed() < SCORE_FLASH_DURATION {
                Some((*cat, *score))
            } else {
                None
            }
        });

        let table = scoreboard_widget::build_scoreboard_table(
            &self.game_state.players,
            self.game_state.current_player_index,
            active_dice.as_ref(),
            self.my_player_id,
            selected_all_idx,
            flash_cat,
        );
        frame.render_widget(table, area);
    }
}
