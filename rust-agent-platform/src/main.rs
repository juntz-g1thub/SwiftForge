use anyhow::Result;
use clap::Parser;
use rust_agent_platform::tui::app_controller::AppController;
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

    let mut app = AppController::new(args.debug)?;
    app.run()?;

    Ok(())
}
