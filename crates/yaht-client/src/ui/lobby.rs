use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use uuid::Uuid;

use yaht_common::lobby::{RoomInfo, RoomInfoState};
use yaht_common::protocol::RoomSnapshot;

#[derive(Debug, Clone)]
pub struct LobbyScreen {
    pub rooms: Vec<RoomInfo>,
    pub table_state: TableState,
    pub player_name: String,
    pub player_id: Option<Uuid>,
    pub status_message: Option<String>,
    pub joined_room: Option<RoomSnapshot>,
}

impl LobbyScreen {
    pub fn new(player_name: String) -> Self {
        Self {
            rooms: Vec::new(),
            table_state: TableState::default(),
            player_name,
            player_id: None,
            status_message: None,
            joined_room: None,
        }
    }

    pub fn is_in_room(&self) -> bool {
        self.joined_room.is_some()
    }

    pub fn is_host(&self) -> bool {
        match (&self.joined_room, self.player_id) {
            (Some(room), Some(pid)) => room.host_id == pid,
            _ => false,
        }
    }

    pub fn select_next(&mut self) {
        if self.rooms.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + 1) % self.rooms.len(),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn select_prev(&mut self) {
        if self.rooms.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(0) => self.rooms.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn selected_room_id(&self) -> Option<uuid::Uuid> {
        self.table_state
            .selected()
            .and_then(|i| self.rooms.get(i))
            .map(|r| r.room_id)
    }

    pub fn draw(&self, frame: &mut Frame) {
        if let Some(ref room) = self.joined_room {
            self.draw_waiting_room(frame, room);
        } else {
            self.draw_room_list(frame);
        }
    }

    fn draw_waiting_room(&self, frame: &mut Frame, room: &RoomSnapshot) {
        let area = frame.area();

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Length(14),
                Constraint::Percentage(20),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[1]);

        let form_area = horizontal[1];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),  // Title
                Constraint::Length(2),  // Room name
                Constraint::Min(4),    // Player list
                Constraint::Length(2),  // Status
                Constraint::Length(2),  // Help
            ])
            .split(form_area);

        // Title
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                "  YAHT ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Waiting Room"),
        ]));
        frame.render_widget(title, chunks[0]);

        // Room name + player count
        let room_info = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &room.room_name,
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({}/{} players)", room.players.len(), room.max_players),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        frame.render_widget(room_info, chunks[1]);

        // Player list
        let mut player_lines: Vec<Line> = room
            .players
            .iter()
            .map(|p| {
                let marker = if p.id == room.host_id { " * " } else { "   " };
                let color = if p.connected {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                Line::from(vec![
                    Span::raw(marker),
                    Span::styled(&p.name, Style::default().fg(color)),
                    if p.id == room.host_id {
                        Span::styled(" (host)", Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw("")
                    },
                ])
            })
            .collect();

        if !room.spectators.is_empty() {
            player_lines.push(Line::from(Span::styled(
                format!("   {} spectator(s)", room.spectators.len()),
                Style::default().fg(Color::DarkGray),
            )));
        }

        let players_widget = Paragraph::new(player_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Players "),
        );
        frame.render_widget(players_widget, chunks[2]);

        // Status
        if let Some(ref msg) = self.status_message {
            let status = Paragraph::new(format!("  {}", msg))
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(status, chunks[3]);
        }

        // Help
        let help_text = if self.is_host() {
            "  [Enter] Start Game  [Esc] Leave Room"
        } else {
            "  Waiting for host to start...  [Esc] Leave Room"
        };
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[4]);
    }

    fn draw_room_list(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title bar
                Constraint::Min(5),   // Room list
                Constraint::Length(3), // Help bar
            ])
            .split(area);

        // Title
        let title = Paragraph::new(Line::from(format!(
            "  YAHT Lobby - Welcome, {}!",
            self.player_name
        )))
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(title, chunks[0]);

        // Room list
        if self.rooms.is_empty() {
            let empty = Paragraph::new("  No rooms available. Press [C] to create one.")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Rooms "),
                );
            frame.render_widget(empty, chunks[1]);
        } else {
            let header = Row::new(vec![
                Cell::from("Room Name"),
                Cell::from("Players"),
                Cell::from("Spectators"),
                Cell::from("Status"),
            ])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

            let rows: Vec<Row> = self
                .rooms
                .iter()
                .map(|room| {
                    let status = match room.state {
                        RoomInfoState::Waiting => "Waiting",
                        RoomInfoState::InProgress => "In Game",
                        RoomInfoState::Finished => "Finished",
                    };
                    let status_color = match room.state {
                        RoomInfoState::Waiting => Color::Green,
                        RoomInfoState::InProgress => Color::Cyan,
                        RoomInfoState::Finished => Color::DarkGray,
                    };
                    Row::new(vec![
                        Cell::from(room.room_name.clone()),
                        Cell::from(format!("{}/{}", room.player_count, room.max_players)),
                        Cell::from(format!("{}", room.spectator_count)),
                        Cell::from(status).style(Style::default().fg(status_color)),
                    ])
                })
                .collect();

            let widths = [
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Rooms "),
                )
                .row_highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(" > ");

            let mut table_state = self.table_state.clone();
            frame.render_stateful_widget(table, chunks[1], &mut table_state);
        }

        // Help bar
        let help_text = if let Some(ref msg) = self.status_message {
            format!("  {} | [C] Create  [Enter] Join  [S] Spectate  [R] Refresh  [Q] Quit", msg)
        } else {
            "  [C] Create room  [Enter] Join  [S] Spectate  [R] Refresh  [Q] Quit".to_string()
        };
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(help, chunks[2]);
    }
}
