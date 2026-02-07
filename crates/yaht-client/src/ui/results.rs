use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ResultsScreen {
    pub final_scores: Vec<(Uuid, String, u16)>,
    pub winner_id: Uuid,
}

impl ResultsScreen {
    pub fn new(final_scores: Vec<(Uuid, String, u16)>, winner_id: Uuid) -> Self {
        let mut scores = final_scores;
        scores.sort_by(|a, b| b.2.cmp(&a.2)); // sort descending by score
        Self {
            final_scores: scores,
            winner_id,
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Winner
                Constraint::Min(5),    // Score table
                Constraint::Length(2), // Help
                Constraint::Percentage(15),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[3]);

        // Title
        let title = Paragraph::new(Line::from(vec![Span::styled(
            "  GAME OVER",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(title, vertical[1]);

        // Winner announcement
        let winner_name = self
            .final_scores
            .iter()
            .find(|(id, _, _)| *id == self.winner_id)
            .map(|(_, name, _)| name.as_str())
            .unwrap_or("Unknown");

        let winner = Paragraph::new(Line::from(vec![
            Span::raw("  Winner: "),
            Span::styled(
                winner_name,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("!"),
        ]))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(winner, vertical[2]);

        // Score table
        let header = Row::new(vec![
            Cell::from("Rank"),
            Cell::from("Player"),
            Cell::from("Score"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

        let rows: Vec<Row> = self
            .final_scores
            .iter()
            .enumerate()
            .map(|(i, (id, name, score))| {
                let rank_style = match i {
                    0 => Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    1 => Style::default().fg(Color::LightBlue),
                    2 => Style::default().fg(Color::Red),
                    _ => Style::default(),
                };
                Row::new(vec![
                    Cell::from(format!("  #{}", i + 1)).style(rank_style),
                    Cell::from(name.clone()).style(rank_style),
                    Cell::from(score.to_string()).style(rank_style),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(8),
            Constraint::Percentage(50),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Final Scores "),
        );
        frame.render_widget(table, horizontal[1]);

        // Help
        let help = Paragraph::new("  [Enter] Back to lobby  [Q] Quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(help, vertical[4]);
    }
}
