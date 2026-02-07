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
                    .fg(Color::Rgb(255, 220, 50))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "- Waiting Room",
                Style::default().fg(Color::Rgb(180, 180, 200)),
            ),
        ]));
        frame.render_widget(title, chunks[0]);

        // Room name + player count
        let room_info = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &room.room_name,
                Style::default()
                    .fg(Color::Rgb(100, 200, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({}/{} players)", room.players.len(), room.max_players),
                Style::default().fg(Color::Rgb(120, 120, 140)),
            ),
        ]));
        frame.render_widget(room_info, chunks[1]);

        // Player list
        let player_colors = [
            Color::Rgb(100, 200, 255),
            Color::Rgb(255, 150, 100),
            Color::Rgb(150, 255, 150),
            Color::Rgb(255, 200, 100),
            Color::Rgb(200, 150, 255),
            Color::Rgb(255, 150, 200),
        ];
        let mut player_lines: Vec<Line> = room
            .players
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let marker = if p.id == room.host_id { " * " } else { "   " };
                let color = if p.connected {
                    player_colors[idx % player_colors.len()]
                } else {
                    Color::Rgb(80, 80, 100)
                };
                Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Rgb(120, 120, 140))),
                    Span::styled(&p.name, Style::default().fg(color)),
                    if p.id == room.host_id {
                        Span::styled(
                            " (host)",
                            Style::default()
                                .fg(Color::Rgb(255, 220, 50))
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("")
                    },
                ])
            })
            .collect();

        if !room.spectators.is_empty() {
            player_lines.push(Line::from(Span::styled(
                format!("   {} spectator(s)", room.spectators.len()),
                Style::default().fg(Color::Rgb(120, 120, 140)),
            )));
        }

        let players_widget = Paragraph::new(player_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
                .title(" Players ")
                .title_style(Style::default().fg(Color::Rgb(180, 180, 200))),
        );
        frame.render_widget(players_widget, chunks[2]);

        // Status
        if let Some(ref msg) = self.status_message {
            let status = Paragraph::new(format!("  {}", msg))
                .style(Style::default().fg(Color::Rgb(100, 255, 150)));
            frame.render_widget(status, chunks[3]);
        }

        // Help
        if self.is_host() {
            let help = Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(100, 255, 150))),
                Span::styled(" Start Game  ", Style::default().fg(Color::Rgb(120, 120, 140))),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(255, 150, 100))),
                Span::styled(" Leave Room", Style::default().fg(Color::Rgb(120, 120, 140))),
            ]));
            frame.render_widget(help, chunks[4]);
        } else {
            let help = Paragraph::new(Line::from(vec![
                Span::styled(
                    "  Waiting for host to start...  ",
                    Style::default().fg(Color::Rgb(150, 150, 170)),
                ),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(255, 150, 100))),
                Span::styled(" Leave Room", Style::default().fg(Color::Rgb(120, 120, 140))),
            ]));
            frame.render_widget(help, chunks[4]);
        }
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
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                "  YAHT ",
                Style::default()
                    .fg(Color::Rgb(255, 220, 50))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Lobby - Welcome, ", Style::default().fg(Color::Rgb(180, 180, 200))),
            Span::styled(
                &self.player_name,
                Style::default()
                    .fg(Color::Rgb(100, 200, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("!", Style::default().fg(Color::Rgb(180, 180, 200))),
        ]))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80))),
        );
        frame.render_widget(title, chunks[0]);

        // Room list
        if self.rooms.is_empty() {
            let empty = Paragraph::new(Line::from(vec![
                Span::styled("  No rooms available. Press ", Style::default().fg(Color::Rgb(120, 120, 140))),
                Span::styled("[C]", Style::default().fg(Color::Rgb(100, 200, 255))),
                Span::styled(" to create one.", Style::default().fg(Color::Rgb(120, 120, 140))),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
                    .title(" Rooms ")
                    .title_style(Style::default().fg(Color::Rgb(180, 180, 200))),
            );
            frame.render_widget(empty, chunks[1]);
        } else {
            let header = Row::new(vec![
                Cell::from("Room Name").style(Style::default().fg(Color::Rgb(180, 180, 200))),
                Cell::from("Players").style(Style::default().fg(Color::Rgb(180, 180, 200))),
                Cell::from("Spectators").style(Style::default().fg(Color::Rgb(180, 180, 200))),
                Cell::from("Status").style(Style::default().fg(Color::Rgb(180, 180, 200))),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD));

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
                        RoomInfoState::Waiting => Color::Rgb(100, 255, 150),
                        RoomInfoState::InProgress => Color::Rgb(100, 200, 255),
                        RoomInfoState::Finished => Color::Rgb(100, 100, 120),
                    };
                    let lock_icon = if room.has_password { "[locked] " } else { "" };
                    Row::new(vec![
                        Cell::from(format!("{}{}", lock_icon, room.room_name))
                            .style(Style::default().fg(Color::Rgb(200, 200, 220))),
                        Cell::from(format!("{}/{}", room.player_count, room.max_players))
                            .style(Style::default().fg(Color::Rgb(150, 150, 170))),
                        Cell::from(format!("{}", room.spectator_count))
                            .style(Style::default().fg(Color::Rgb(150, 150, 170))),
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
                        .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
                        .title(" Rooms ")
                        .title_style(Style::default().fg(Color::Rgb(180, 180, 200))),
                )
                .row_highlight_style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 60))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(" > ");

            let mut table_state = self.table_state.clone();
            frame.render_stateful_widget(table, chunks[1], &mut table_state);
        }

        // Help bar
        let mut help_spans = vec![Span::raw("  ")];
        if let Some(ref msg) = self.status_message {
            help_spans.push(Span::styled(
                format!("{} | ", msg),
                Style::default().fg(Color::Rgb(100, 255, 150)),
            ));
        }
        help_spans.extend_from_slice(&[
            Span::styled("[C]", Style::default().fg(Color::Rgb(100, 200, 255))),
            Span::styled(" Create  ", Style::default().fg(Color::Rgb(120, 120, 140))),
            Span::styled("[Enter]", Style::default().fg(Color::Rgb(100, 255, 150))),
            Span::styled(" Join  ", Style::default().fg(Color::Rgb(120, 120, 140))),
            Span::styled("[S]", Style::default().fg(Color::Rgb(200, 150, 255))),
            Span::styled(" Spectate  ", Style::default().fg(Color::Rgb(120, 120, 140))),
            Span::styled("[R]", Style::default().fg(Color::Rgb(255, 200, 100))),
            Span::styled(" Refresh  ", Style::default().fg(Color::Rgb(120, 120, 140))),
            Span::styled("[Q]", Style::default().fg(Color::Rgb(255, 150, 100))),
            Span::styled(" Quit", Style::default().fg(Color::Rgb(120, 120, 140))),
        ]);
        let help = Paragraph::new(Line::from(help_spans)).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Rgb(60, 60, 80))),
        );
        frame.render_widget(help, chunks[2]);
    }
}
