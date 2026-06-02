use clap::Parser;
use swiftforge::tui::app_controller::AppController;
use swiftforge_log::{info, init_log, LogLevel};

#[derive(Parser, Debug)]
#[command(name = "swiftforge")]
#[command(about = "SwiftForge - Agent Platform TUI")]
struct Args {
    #[arg(short, long, help = "Enable debug logging (TRACE level)")]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".swiftforge")
        .join("log");

    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let log_path = log_dir.join(format!("swiftforge_{}.log", timestamp));

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
