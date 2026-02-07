mod app;
mod event;
mod input;
mod network;
mod solo;
mod ui;

use std::io;

use clap::Parser;
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// YAHT Client - Multiplayer Yahtzee terminal game
#[derive(Parser, Debug)]
#[command(name = "yaht-client", version, about)]
struct Args {
    /// Server address to connect to
    #[arg(short = 's', long, default_value = "127.0.0.1:9876")]
    server: String,

    /// Player name
    #[arg(short, long)]
    name: Option<String>,

    /// Solo mode: play against AI opponents (no server needed)
    #[arg(long)]
    solo: bool,

    /// Number of AI opponents in solo mode (1-5)
    #[arg(long, default_value_t = 1)]
    ai_count: u8,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "yaht_client=debug".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = if args.solo {
        let player_name = args.name.unwrap_or_else(|| "Player".to_string());
        let ai_count = args.ai_count.clamp(1, 5);
        solo::run_solo(&mut terminal, player_name, ai_count).await
    } else {
        app::run(&mut terminal, args.server, args.name).await
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}
