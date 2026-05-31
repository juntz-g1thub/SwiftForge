use clap::Parser;
use rust_agent_platform::tui::app_controller::AppController;
use swiftforge_log::{info, init_log, LogLevel};

#[derive(Parser, Debug)]
#[command(name = "ragent")]
#[command(about = "Rust Agent Platform - TUI")]
struct Args {
    #[arg(short, long, help = "Enable debug logging (TRACE level)")]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".fastcode");
    let log_path = log_dir.join("ragent.log");

    let level = if args.debug {
        LogLevel::TRACE
    } else {
        LogLevel::INFO
    };

    init_log(log_path, level)?;

    info!("[main]", "Application started (debug={})", args.debug);

    let mut app = AppController::new()?;
    app.run()?;

    Ok(())
}
