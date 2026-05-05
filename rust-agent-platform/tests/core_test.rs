use rust_agent_platform::core::{Agent, AgentConfig, AgentRole, ToolRegistry, Tool, ToolCall, ToolResult};
use async_trait::async_trait;

struct DummyTool;

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &str { "dummy" }
    fn description(&self) -> &str { "A dummy tool for testing" }
    async fn execute(&self, _call: ToolCall) -> ToolResult {
        ToolResult { success: true, output: Some("done".to_string()), error: None }
    }
}

#[test]
fn test_agent_creation() {
    let agent = Agent::new(AgentConfig {
        name: "test".to_string(),
        role: AgentRole::Orchestrator,
        model: None,
        temperature: 0.1,
    });
    assert_eq!(agent.name(), "test");
}

#[test]
fn test_tool_registry() {
    let mut registry = ToolRegistry::new();
    registry.register(DummyTool);
}