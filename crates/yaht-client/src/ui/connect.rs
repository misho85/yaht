use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct ConnectScreen {
    pub host: String,
    pub name: String,
    pub active_field: ConnectField,
    pub error_message: Option<String>,
    pub connecting: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectField {
    Host,
    Name,
}

impl ConnectScreen {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1:9876".to_string(),
            name: String::new(),
            active_field: ConnectField::Name,
            error_message: None,
            connecting: false,
        }
    }

    pub fn switch_field(&mut self) {
        self.active_field = match self.active_field {
            ConnectField::Host => ConnectField::Name,
            ConnectField::Name => ConnectField::Host,
        };
    }

    pub fn type_char(&mut self, c: char) {
        match self.active_field {
            ConnectField::Host => self.host.push(c),
            ConnectField::Name => self.name.push(c),
        }
    }

    pub fn backspace(&mut self) {
        match self.active_field {
            ConnectField::Host => {
                self.host.pop();
            }
            ConnectField::Name => {
                self.name.pop();
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Center the form
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(15),
                Constraint::Percentage(25),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(vertical[1]);

        let form_area = horizontal[1];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(3), // Name field
                Constraint::Length(3), // Host field
                Constraint::Length(2), // Status/Error
                Constraint::Length(2), // Help
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
            Span::raw(" - Multiplayer Yahtzee"),
        ]));
        frame.render_widget(title, chunks[0]);

        // Name field
        let name_style = if self.active_field == ConnectField::Name {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let name_input = Paragraph::new(self.name.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(name_style)
                    .title(" Player Name "),
            );
        frame.render_widget(name_input, chunks[1]);

        // Host field
        let host_style = if self.active_field == ConnectField::Host {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let host_input = Paragraph::new(self.host.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(host_style)
                    .title(" Server Address "),
            );
        frame.render_widget(host_input, chunks[2]);

        // Status/Error
        if self.connecting {
            let status = Paragraph::new("  Connecting...")
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(status, chunks[3]);
        } else if let Some(ref err) = self.error_message {
            let error = Paragraph::new(format!("  {}", err))
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error, chunks[3]);
        }

        // Help
        let help = Paragraph::new("  [Tab] Switch field  [Enter] Connect  [Esc] Quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[4]);

        // Set cursor position
        if !self.connecting {
            let (cursor_x, cursor_y) = match self.active_field {
                ConnectField::Name => (chunks[1].x + self.name.len() as u16 + 1, chunks[1].y + 1),
                ConnectField::Host => (chunks[2].x + self.host.len() as u16 + 1, chunks[2].y + 1),
            };
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
