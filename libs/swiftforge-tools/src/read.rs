use async_trait::async_trait;
use std::path::Path;
use swiftforge_types::{Tool, ToolCall, ToolResult};

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }
    fn description(&self) -> &str {
        "Read file contents"
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": { "path": { "type": "string", "description": "Path to the file to read" } }, "required": ["path"] })
    }
    async fn execute(&self, call: ToolCall) -> ToolResult {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match std::fs::read_to_string(Path::new(path)) {
            Ok(content) => ToolResult {
                success: true,
                output: Some(content),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }
}
