use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw_help_popup(frame: &mut Frame) {
    let area = frame.area();

    // Center popup
    let popup_area = centered_rect(70, 80, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let sections = vec![
        (
            "YAHTZEE SCORING RULES",
            Color::Rgb(255, 220, 50),
            vec![],
        ),
        (
            "Upper Section",
            Color::Rgb(100, 200, 255),
            vec![
                ("Ones - Sixes", "Sum of matching dice face values"),
                ("Upper Bonus", "+35 if upper total >= 63"),
            ],
        ),
        (
            "Lower Section",
            Color::Rgb(200, 150, 255),
            vec![
                ("3 of a Kind", "Sum of all dice if 3+ match"),
                ("4 of a Kind", "Sum of all dice if 4+ match"),
                ("Full House", "25 pts (3 of one + 2 of another)"),
                ("Sm. Straight", "30 pts (4 consecutive dice)"),
                ("Lg. Straight", "40 pts (5 consecutive dice)"),
                ("YAHTZEE", "50 pts (all 5 dice the same)"),
                ("Chance", "Sum of all dice (any combination)"),
                ("Yahtzee Bonus", "+100 per extra Yahtzee"),
            ],
        ),
        (
            "CONTROLS",
            Color::Rgb(100, 255, 150),
            vec![
                ("[R]", "Roll dice (up to 3 times per turn)"),
                ("[1]-[5]", "Toggle hold on individual dice"),
                ("[j]/[k]", "Navigate categories up/down"),
                ("[S]/[Enter]", "Score selected category"),
                ("[C]", "Open/close chat"),
                ("[?]", "Toggle this help screen"),
                ("[Q]", "Quit game"),
            ],
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (title, color, items) in &sections {
        lines.push(Line::from(Span::styled(
            format!("  {}", title),
            Style::default()
                .fg(*color)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for (key, desc) in items {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {:<16}", key),
                    Style::default().fg(Color::Rgb(200, 200, 220)),
                ),
                Span::styled(
                    *desc,
                    Style::default().fg(Color::Rgb(150, 150, 170)),
                ),
            ]));
        }
        if !items.is_empty() {
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(Span::styled(
        "  Press [?] or any key to close",
        Style::default().fg(Color::Rgb(100, 100, 120)),
    )));

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 200, 255)))
                .title(" Help - Yahtzee Rules & Controls ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(255, 220, 50))
                        .add_modifier(Modifier::BOLD),
                ),
        );

    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
