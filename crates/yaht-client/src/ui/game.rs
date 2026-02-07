use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use uuid::Uuid;

use yaht_common::dice::{DiceSet, MAX_ROLLS};
use yaht_common::game::{GameStateSnapshot, TurnPhase};
use yaht_common::scoring::Category;

use super::dice_widget;
use super::scoreboard_widget;

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

        let title = Line::from(vec![
            Span::styled(
                " YAHT ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " Round {}/{}  |  Turn: {}",
                self.round, self.game_state.total_rounds, current_name
            )),
        ]);
        frame.render_widget(Paragraph::new(title), area);
    }

    fn draw_dice_area(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(ref dice) = self.dice {
            let lines = dice_widget::render_dice_row(&dice.dice);
            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Dice "),
            );
            frame.render_widget(paragraph, area);
        } else {
            let paragraph = Paragraph::new("  Waiting for roll...")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Dice "),
                );
            frame.render_widget(paragraph, area);
        }
    }

    fn draw_action_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let is_my_turn = self.is_my_turn(&self.my_player_id);
        let can_roll = is_my_turn
            && self.rolls_remaining > 0
            && matches!(
                self.game_state.turn_phase.as_ref(),
                Some(TurnPhase::WaitingForRoll) | Some(TurnPhase::Rolling { .. })
            );
        let can_score = is_my_turn
            && matches!(
                self.game_state.turn_phase.as_ref(),
                Some(TurnPhase::Rolling { .. }) | Some(TurnPhase::MustScore)
            );

        let mut lines = Vec::new();

        if is_my_turn {
            let mut spans = vec![Span::raw("  ")];
            if can_roll {
                spans.push(Span::styled("[R]", Style::default().fg(Color::Green)));
                spans.push(Span::raw(format!(" Roll ({} left)  ", self.rolls_remaining)));
            }
            spans.push(Span::styled("[1-5]", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(" Hold  "));
            if can_score {
                spans.push(Span::styled("[S]", Style::default().fg(Color::Magenta)));
                spans.push(Span::raw(" Score  "));
            }
            spans.push(Span::styled("[C]", Style::default().fg(Color::Blue)));
            spans.push(Span::raw(" Chat"));
            lines.push(Line::from(spans));
        } else {
            lines.push(Line::from(Span::styled(
                "  Waiting for other player's turn...",
                Style::default().fg(Color::DarkGray),
            )));
        }

        if let Some(ref msg) = self.status_message {
            lines.push(Line::from(Span::styled(
                format!("  {}", msg),
                Style::default().fg(Color::Cyan),
            )));
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
                        Style::default().fg(Color::DarkGray),
                    ))
                } else {
                    Line::from(Span::raw(format!("  {}", msg)))
                }
            })
            .collect();

        let prefix = if self.chat_focused { "  > " } else { "  " };
        let style = if self.chat_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, self.chat_input),
            style,
        )));

        let border_style = if self.chat_focused {
            Style::default().fg(Color::Blue)
        } else {
            Style::default()
        };

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Chat "),
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

        let dice_values = self.dice.as_ref().map(|d| d.values());
        let table = scoreboard_widget::build_scoreboard_table(
            &self.game_state.players,
            self.game_state.current_player_index,
            dice_values.as_ref(),
            self.my_player_id,
            selected_all_idx,
        );
        frame.render_widget(table, area);
    }
}
