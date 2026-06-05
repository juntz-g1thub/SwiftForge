use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use test_runner::{
    load_script, load_scripts_from_dir, TestExecutor, JsonReporter, HtmlReporter, JunitReporter,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "sf-test")]
#[command(about = "SwiftForge Test Runner", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Enable debug output")]
    debug: bool,

    #[arg(short, long, help = "Test report format [json|html|junit]")]
    report_format: Option<String>,

    #[arg(short, long, help = "Output file for report")]
    output: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(help = "Path to test script or directory")]
        path: Option<PathBuf>,
    },
    List {
        #[arg(help = "Directory containing test scripts")]
        dir: Option<PathBuf>,
    },
}

fn setup_logging(debug: bool) {
    let filter = if debug {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.debug);

    let report_format = cli.report_format.unwrap_or_else(|| "json".to_string());
    let output_path = cli.output;

    match cli.command {
        Some(Commands::Run { path }) => {
            let target_path = path.unwrap_or_else(|| PathBuf::from("scripts"));

            if target_path.is_dir() {
                let scripts = load_scripts_from_dir(&target_path)?;
                tracing::info!("Found {} test scripts", scripts.len());

                let mut all_passed = true;
                for script in scripts {
                    let reporters: Vec<Box<dyn test_runner::Reporter>> = match report_format.as_str() {
                        "html" => {
                            let path = output_path.clone().unwrap_or_else(|| PathBuf::from("test-report.html"));
                            vec![Box::new(HtmlReporter::new(path))]
                        }
                        "junit" => {
                            let path = output_path.clone().unwrap_or_else(|| PathBuf::from("test-results.xml"));
                            vec![Box::new(JunitReporter::new(path))]
                        }
                        _ => {
                            let path = output_path.clone().unwrap_or_else(|| PathBuf::from("test-results.json"));
                            vec![Box::new(JsonReporter::new(Some(path)))]
                        }
                    };

                    let mut executor = TestExecutor::new(script, reporters);
                    let summary = executor.run().await?;

                    println!("\nTest: {}", summary.name);
                    println!("Status: {}", summary.status);
                    println!("Duration: {}ms", summary.duration_ms);
                    println!("Steps: {}/{} passed", summary.passed_steps, summary.total_steps);

                    if summary.status != test_runner::TestStatus::Passed {
                        all_passed = false;
                    }
                }

                if all_passed {
                    std::process::exit(0);
                } else {
                    std::process::exit(1);
                }
            } else {
                let script = load_script(&target_path)?;
                tracing::info!("Running test: {}", script.name);

                let reporters: Vec<Box<dyn test_runner::Reporter>> = match report_format.as_str() {
                    "html" => {
                        let path = output_path.unwrap_or_else(|| PathBuf::from("test-report.html"));
                        vec![Box::new(HtmlReporter::new(path))]
                    }
                    "junit" => {
                        let path = output_path.unwrap_or_else(|| PathBuf::from("test-results.xml"));
                        vec![Box::new(JunitReporter::new(path))]
                    }
                    _ => {
                        let path = output_path.unwrap_or_else(|| PathBuf::from("test-results.json"));
                        vec![Box::new(JsonReporter::new(Some(path)))]
                    }
                };

                let mut executor = TestExecutor::new(script, reporters);
                let summary = executor.run().await?;

                println!("\nTest: {}", summary.name);
                println!("Status: {}", summary.status);
                println!("Duration: {}ms", summary.duration_ms);
                println!("Steps: {}/{} passed", summary.passed_steps, summary.total_steps);

                executor.finalize()?;

                if summary.status == test_runner::TestStatus::Passed {
                    std::process::exit(0);
                } else {
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::List { dir }) => {
            let target_dir = dir.unwrap_or_else(|| PathBuf::from("scripts"));

            if target_dir.is_dir() {
                let scripts = load_scripts_from_dir(&target_dir)?;
                println!("Found {} test scripts:\n", scripts.len());
                for script in scripts {
                    println!("  - {} ({})", script.name, script.version);
                    println!("    Description: {}", script.description);
                    println!("    Tags: {:?}", script.tags);
                    println!("    Steps: {}", script.steps.len());
                    println!();
                }
            } else {
                println!("Directory not found: {:?}", target_dir);
                std::process::exit(1);
            }
        }
        None => {
            println!("No command specified. Use --help for usage information.");
            std::process::exit(1);
        }
    }

    Ok(())
}