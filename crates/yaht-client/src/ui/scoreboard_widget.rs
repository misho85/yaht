use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

use yaht_common::game::PlayerSnapshot;
use yaht_common::scoring::{self, Category};

/// Player colors - each player gets a distinct color
const PLAYER_COLORS: [Color; 6] = [
    Color::Rgb(100, 200, 255), // Sky blue
    Color::Rgb(255, 150, 100), // Coral
    Color::Rgb(150, 255, 150), // Lime
    Color::Rgb(255, 200, 100), // Gold
    Color::Rgb(200, 150, 255), // Lavender
    Color::Rgb(255, 150, 200), // Pink
];

fn player_color(idx: usize) -> Color {
    PLAYER_COLORS[idx % PLAYER_COLORS.len()]
}

pub fn build_scoreboard_table<'a>(
    players: &[PlayerSnapshot],
    current_player_index: usize,
    dice_values: Option<&[u8; 5]>,
    my_player_id: uuid::Uuid,
    selected_category: Option<usize>,
    flash_cat: Option<(Category, u16)>,
) -> Table<'a> {
    let header_cells: Vec<Cell> = std::iter::once(
        Cell::from("Category").style(Style::default().fg(Color::Rgb(180, 180, 200))),
    )
    .chain(players.iter().enumerate().map(|(idx, p)| {
        let mut style = Style::default()
            .fg(player_color(idx))
            .add_modifier(Modifier::BOLD);
        if p.id == my_player_id {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if idx == current_player_index {
            Cell::from(format!(">{}", truncate_name(&p.name, 7))).style(style)
        } else {
            Cell::from(truncate_name(&p.name, 8)).style(style)
        }
    }))
    .collect();

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let mut rows: Vec<Row> = Vec::new();

    // Categories
    for (cat_idx, cat) in Category::ALL.iter().enumerate() {
        let is_flashing = flash_cat.map(|(fc, _)| fc == *cat).unwrap_or(false);
        let is_selected = selected_category == Some(cat_idx);
        let is_upper = cat.is_upper();

        let row_style = if is_flashing {
            Style::default()
                .bg(Color::Rgb(60, 60, 30))
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().bg(Color::Rgb(40, 40, 60))
        } else {
            Style::default()
        };

        let name_style = if is_flashing {
            Style::default()
                .fg(Color::Rgb(255, 220, 50))
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_upper {
            Style::default().fg(Color::Rgb(180, 200, 220))
        } else {
            Style::default().fg(Color::Rgb(200, 180, 220))
        };

        let mut cells: Vec<Cell> = vec![Cell::from(cat.display_name().to_string()).style(name_style)];

        for (player_idx, player) in players.iter().enumerate() {
            if let Some(&score) = player.scorecard.scores.get(cat) {
                let cell_style = if is_flashing {
                    Style::default()
                        .fg(Color::Rgb(100, 255, 100))
                        .add_modifier(Modifier::BOLD)
                } else if score == 0 {
                    Style::default().fg(Color::Rgb(100, 100, 100))
                } else {
                    Style::default().fg(player_color(player_idx))
                };
                cells.push(Cell::from(score.to_string()).style(cell_style));
            } else if player_idx == current_player_index && dice_values.is_some() {
                let potential = scoring::compute_score(*cat, dice_values.unwrap());
                let pot_style = if potential == 0 {
                    Style::default().fg(Color::Rgb(80, 80, 80))
                } else if is_selected {
                    Style::default()
                        .fg(Color::Rgb(100, 255, 200))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(100, 160, 140))
                };
                cells.push(Cell::from(format!("({})", potential)).style(pot_style));
            } else {
                cells.push(
                    Cell::from("·").style(Style::default().fg(Color::Rgb(60, 60, 70))),
                );
            }
        }

        rows.push(Row::new(cells).style(row_style));

        // Add separator after upper section (index 5 = Sixes)
        if cat_idx == 5 {
            // Bonus row
            let mut bonus_cells: Vec<Cell> = vec![Cell::from("  Bonus")
                .style(Style::default().fg(Color::Rgb(120, 120, 140)))];
            for (_player_idx, player) in players.iter().enumerate() {
                let bonus = player.scorecard.upper_bonus();
                if bonus > 0 {
                    bonus_cells.push(
                        Cell::from(format!("+{}", bonus)).style(
                            Style::default()
                                .fg(Color::Rgb(100, 255, 100))
                                .add_modifier(Modifier::BOLD),
                        ),
                    );
                } else {
                    let subtotal = player.scorecard.upper_subtotal();
                    let progress_color = if subtotal >= 50 {
                        Color::Rgb(200, 200, 50)
                    } else if subtotal >= 30 {
                        Color::Rgb(150, 150, 80)
                    } else {
                        Color::Rgb(100, 100, 120)
                    };
                    bonus_cells.push(
                        Cell::from(format!("{}/63", subtotal))
                            .style(Style::default().fg(progress_color)),
                    );
                }
            }
            rows.push(Row::new(bonus_cells));

            // Separator
            let sep_cells: Vec<Cell> = std::iter::once(Cell::from("───────────"))
                .chain((0..players.len()).map(|_| Cell::from("────")))
                .collect();
            rows.push(Row::new(sep_cells).style(Style::default().fg(Color::Rgb(60, 60, 80))));
        }
    }

    // Yahtzee bonus row
    let has_any_bonus = players.iter().any(|p| p.scorecard.yahtzee_bonus_count > 0);
    if has_any_bonus {
        let mut yb_cells: Vec<Cell> = vec![Cell::from("  YZ Bonus")
            .style(Style::default().fg(Color::Rgb(120, 120, 140)))];
        for player in players.iter() {
            if player.scorecard.yahtzee_bonus_count > 0 {
                yb_cells.push(
                    Cell::from(format!(
                        "+{}",
                        player.scorecard.yahtzee_bonus_count as u16 * 100
                    ))
                    .style(
                        Style::default()
                            .fg(Color::Rgb(255, 200, 50))
                            .add_modifier(Modifier::BOLD),
                    ),
                );
            } else {
                yb_cells
                    .push(Cell::from("·").style(Style::default().fg(Color::Rgb(60, 60, 70))));
            }
        }
        rows.push(Row::new(yb_cells));
    }

    // Total separator
    let sep_cells: Vec<Cell> = std::iter::once(Cell::from("───────────"))
        .chain((0..players.len()).map(|_| Cell::from("════")))
        .collect();
    rows.push(Row::new(sep_cells).style(Style::default().fg(Color::Rgb(80, 80, 100))));

    // Total row
    let mut total_cells: Vec<Cell> = vec![Cell::from("TOTAL").style(
        Style::default()
            .fg(Color::Rgb(255, 220, 50))
            .add_modifier(Modifier::BOLD),
    )];
    for (player_idx, player) in players.iter().enumerate() {
        total_cells.push(
            Cell::from(player.scorecard.grand_total().to_string()).style(
                Style::default()
                    .fg(player_color(player_idx))
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

    Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
            .title(" Scoreboard ")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(255, 220, 50))
                    .add_modifier(Modifier::BOLD),
            ),
    )
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}.", &name[..max_len - 1])
    }
}
