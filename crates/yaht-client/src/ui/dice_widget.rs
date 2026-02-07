use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use yaht_common::dice::Die;

fn render_die_styled(die: &Die, index: usize, animating: bool) -> Vec<Line<'static>> {
    let (border_style, dot_style) = if animating && !die.held {
        (
            Style::default().fg(Color::Rgb(100, 200, 255)),
            Style::default()
                .fg(Color::Rgb(100, 255, 200))
                .add_modifier(Modifier::BOLD),
        )
    } else if die.held {
        (
            Style::default().fg(Color::Rgb(255, 180, 50)),
            Style::default()
                .fg(Color::Rgb(255, 220, 100))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            Style::default().fg(Color::Rgb(180, 180, 200)),
            Style::default().fg(Color::White),
        )
    };

    let (top, mid, bot) = die_face(die.value);

    let label = if die.held {
        format!(" [{}]* ", index + 1)
    } else {
        format!("  {}   ", index + 1)
    };

    let label_style = if die.held {
        Style::default()
            .fg(Color::Rgb(255, 180, 50))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(120, 120, 140))
    };

    vec![
        Line::from(Span::styled("┌─────┐", border_style)),
        Line::from(vec![
            Span::styled("│", border_style),
            Span::styled(top, dot_style),
            Span::styled("│", border_style),
        ]),
        Line::from(vec![
            Span::styled("│", border_style),
            Span::styled(mid, dot_style),
            Span::styled("│", border_style),
        ]),
        Line::from(vec![
            Span::styled("│", border_style),
            Span::styled(bot, dot_style),
            Span::styled("│", border_style),
        ]),
        Line::from(Span::styled("└─────┘", border_style)),
        Line::from(Span::styled(label, label_style)),
    ]
}

fn die_face(value: u8) -> (&'static str, &'static str, &'static str) {
    match value {
        1 => ("     ", "  *  ", "     "),
        2 => ("    *", "     ", "*    "),
        3 => ("    *", "  *  ", "*    "),
        4 => ("*   *", "     ", "*   *"),
        5 => ("*   *", "  *  ", "*   *"),
        6 => ("*   *", "*   *", "*   *"),
        _ => ("     ", "  ?  ", "     "),
    }
}

/// Render all 5 dice side by side as a block of lines.
pub fn render_dice_row(dice: &[Die; 5]) -> Vec<Line<'static>> {
    render_dice_row_animated(dice, false)
}

/// Render all 5 dice side by side, with optional animation styling.
pub fn render_dice_row_animated(dice: &[Die; 5], animating: bool) -> Vec<Line<'static>> {
    let rendered: Vec<Vec<Line>> = dice
        .iter()
        .enumerate()
        .map(|(i, d)| render_die_styled(d, i, animating))
        .collect();

    let num_lines = rendered[0].len();
    let mut result = Vec::new();

    for line_idx in 0..num_lines {
        let mut spans = Vec::new();
        for (die_idx, die_lines) in rendered.iter().enumerate() {
            if die_idx > 0 {
                spans.push(Span::raw("  ")); // spacing between dice
            }
            for span in &die_lines[line_idx].spans {
                spans.push(span.clone());
            }
        }
        result.push(Line::from(spans));
    }

    result
}
