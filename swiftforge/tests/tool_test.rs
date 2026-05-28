use rust_agent_platform::core::{ToolRegistry, Tool, ToolCall, ToolResult};
use rust_agent_platform::tools::{BashTool, ReadTool, WriteTool, EditTool, GrepTool};
use async_trait::async_trait;
use std::collections::HashMap;

struct DummyTool;

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &str { "dummy" }
    fn description(&self) -> &str { "A dummy tool" }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    async fn execute(&self, _call: ToolCall) -> ToolResult {
        ToolResult { success: true, output: Some("done".to_string()), error: None }
    }
}

#[test]
fn test_bash_tool_creation() {
    let tool = BashTool::new();
    assert_eq!(tool.name(), "bash");
}

#[test]
fn test_read_tool_creation() {
    let tool = ReadTool::new();
    assert_eq!(tool.name(), "read");
}

#[test]
fn test_write_tool_creation() {
    let tool = WriteTool::new();
    assert_eq!(tool.name(), "write");
}

#[test]
fn test_edit_tool_creation() {
    let tool = EditTool::new();
    assert_eq!(tool.name(), "edit");
}

#[test]
fn test_grep_tool_creation() {
    let tool = GrepTool::new();
    assert_eq!(tool.name(), "grep");
}

#[test]
fn test_tool_registry_with_multiple_tools() {
    let mut registry = ToolRegistry::new();
    registry.register(BashTool::new());
    registry.register(ReadTool::new());
    registry.register(WriteTool::new());
    registry.register(EditTool::new());
    registry.register(GrepTool::new());
}