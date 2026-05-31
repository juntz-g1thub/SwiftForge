use clap::Parser;
use rust_agent_platform::tui::app_controller::AppController;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "ragent")]
#[command(about = "Rust Agent Platform - TUI")]
struct Args {
    #[arg(short, long, help = "Show debug window and log file")]
    debug: bool,
}

fn setup_tracing(debug_mode: bool) -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".fastcode");
    std::fs::create_dir_all(&log_dir).ok();

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_path = log_dir.join(format!("ragent_{}.log", timestamp));
    std::fs::write(&log_path, "").ok();

    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_path)
        .expect("Failed to open log file");
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let _level = if debug_mode {
        tracing::Level::TRACE
    } else {
        tracing::Level::INFO
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("ragent"));

    let subscriber = tracing_subscriber::registry().with(filter).with(
        fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true),
    );

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    tracing::info!("Log file: {:?}", log_path);
    guard
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let _guard = setup_tracing(args.debug);

    tracing::info!("Starting Rust Agent Platform (debug={})", args.debug);

    let mut app = AppController::new(args.debug)?;
    app.run()?;

    Ok(())
}
