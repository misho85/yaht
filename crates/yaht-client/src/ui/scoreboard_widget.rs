use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

use yaht_common::game::PlayerSnapshot;
use yaht_common::scoring::{self, Category};

pub fn build_scoreboard_table<'a>(
    players: &[PlayerSnapshot],
    current_player_index: usize,
    dice_values: Option<&[u8; 5]>,
    my_player_id: uuid::Uuid,
    selected_category: Option<usize>,
) -> Table<'a> {
    let header_cells: Vec<Cell> = std::iter::once(Cell::from("Category"))
        .chain(players.iter().map(|p| {
            let style = if p.id == my_player_id {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Cell::from(truncate_name(&p.name, 8)).style(style)
        }))
        .collect();

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let mut rows: Vec<Row> = Vec::new();

    // Upper section
    for (cat_idx, cat) in Category::ALL.iter().enumerate() {
        let name = cat.display_name().to_string();

        let is_selected = selected_category == Some(cat_idx);
        let row_style = if is_selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        let mut cells: Vec<Cell> = vec![Cell::from(name)];

        for (player_idx, player) in players.iter().enumerate() {
            if let Some(&score) = player.scorecard.scores.get(cat) {
                cells.push(Cell::from(score.to_string()));
            } else if player_idx == current_player_index && dice_values.is_some() {
                let potential = scoring::compute_score(*cat, dice_values.unwrap());
                cells.push(
                    Cell::from(format!("({})", potential))
                        .style(Style::default().fg(Color::DarkGray)),
                );
            } else {
                cells.push(Cell::from("-").style(Style::default().fg(Color::DarkGray)));
            }
        }

        rows.push(Row::new(cells).style(row_style));

        // Add separator after upper section
        if cat_idx == 5 {
            // Add bonus row
            let mut bonus_cells: Vec<Cell> = vec![Cell::from("  Bonus")
                .style(Style::default().fg(Color::DarkGray))];
            for player in players.iter() {
                let bonus = player.scorecard.upper_bonus();
                if bonus > 0 {
                    bonus_cells.push(Cell::from(format!("+{}", bonus))
                        .style(Style::default().fg(Color::Green)));
                } else {
                    let subtotal = player.scorecard.upper_subtotal();
                    bonus_cells.push(
                        Cell::from(format!("{}/63", subtotal))
                            .style(Style::default().fg(Color::DarkGray)),
                    );
                }
            }
            rows.push(Row::new(bonus_cells));

            // Separator
            let sep_cells: Vec<Cell> = std::iter::once(Cell::from("───────────"))
                .chain((0..players.len()).map(|_| Cell::from("────")))
                .collect();
            rows.push(Row::new(sep_cells).style(Style::default().fg(Color::DarkGray)));
        }
    }

    // Total row
    let sep_cells: Vec<Cell> = std::iter::once(Cell::from("───────────"))
        .chain((0..players.len()).map(|_| Cell::from("────")))
        .collect();
    rows.push(Row::new(sep_cells).style(Style::default().fg(Color::DarkGray)));

    let mut total_cells: Vec<Cell> = vec![Cell::from("TOTAL")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )];
    for player in players.iter() {
        total_cells.push(
            Cell::from(player.scorecard.grand_total().to_string())
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
        );
    }
    rows.push(Row::new(total_cells));

    // Column widths
    let mut widths = vec![Constraint::Length(12)]; // category name
    for _ in players {
        widths.push(Constraint::Length(8));
    }

    Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Scoreboard "),
        )
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}.", &name[..max_len - 1])
    }
}
