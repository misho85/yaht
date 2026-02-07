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
                .fg(Color::Rgb(255, 220, 50))
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
            Span::styled("  Winner: ", Style::default().fg(Color::Rgb(180, 180, 200))),
            Span::styled(
                winner_name,
                Style::default()
                    .fg(Color::Rgb(100, 255, 150))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" !", Style::default().fg(Color::Rgb(255, 220, 50))),
        ]))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(winner, vertical[2]);

        // Score table
        let header = Row::new(vec![
            Cell::from("Rank").style(Style::default().fg(Color::Rgb(180, 180, 200))),
            Cell::from("Player").style(Style::default().fg(Color::Rgb(180, 180, 200))),
            Cell::from("Score").style(Style::default().fg(Color::Rgb(180, 180, 200))),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

        let podium_colors = [
            Color::Rgb(255, 220, 50),  // Gold
            Color::Rgb(180, 200, 220), // Silver
            Color::Rgb(210, 150, 100), // Bronze
        ];

        let rows: Vec<Row> = self
            .final_scores
            .iter()
            .enumerate()
            .map(|(i, (_id, name, score))| {
                let color = if i < 3 {
                    podium_colors[i]
                } else {
                    Color::Rgb(120, 120, 140)
                };
                let style = if i == 0 {
                    Style::default()
                        .fg(color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };
                let medal = match i {
                    0 => "  #1",
                    1 => "  #2",
                    2 => "  #3",
                    _ => "   -",
                };
                Row::new(vec![
                    Cell::from(medal.to_string()).style(style),
                    Cell::from(name.clone()).style(style),
                    Cell::from(score.to_string()).style(style),
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
                .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
                .title(" Final Scores ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(255, 220, 50))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(table, horizontal[1]);

        // Help
        let help = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("[Enter]", Style::default().fg(Color::Rgb(100, 255, 150))),
            Span::styled(" Back to lobby  ", Style::default().fg(Color::Rgb(120, 120, 140))),
            Span::styled("[Q]", Style::default().fg(Color::Rgb(255, 150, 100))),
            Span::styled(" Quit", Style::default().fg(Color::Rgb(120, 120, 140))),
        ]))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(help, vertical[4]);
    }
}
