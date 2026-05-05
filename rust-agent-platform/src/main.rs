use rust_agent_platform::core::{Agent, AgentConfig, AgentRole};
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Rust Agent Platform...");

    let agent = Agent::new(AgentConfig {
        name: "test".to_string(),
        role: AgentRole::Orchestrator,
        model: Some("claude-3-sonnet".to_string()),
        temperature: 0.1,
    });

    println!("Agent created: {:?}", agent.name());
}
