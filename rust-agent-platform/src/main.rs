use anyhow::Result;
use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Terminal};
use rust_agent_platform::tui::App;
use std::io;

#[derive(Parser, Debug)]
#[command(name = "ragent")]
#[command(about = "Rust Agent Platform - TUI")]
struct Args {
    #[arg(short, long, help = "Show debug window and log file")]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(io::stderr)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("Starting Rust Agent Platform...");

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_debug(args.debug);
    app.run(&mut terminal)?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}
