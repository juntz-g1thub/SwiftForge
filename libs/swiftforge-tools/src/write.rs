use async_trait::async_trait;
use std::path::Path;
use swiftforge_types::{Tool, ToolCall, ToolResult};

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str { "write" }
    fn description(&self) -> &str { "Write content to a file" }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": { "path": { "type": "string", "description": "Path to the file to write" }, "content": { "type": "string", "description": "Content to write to the file" } }, "required": ["path", "content"] })
    }
    async fn execute(&self, call: ToolCall) -> ToolResult {
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let content = call.arguments.get("content").and_then(|v| v.as_str()).unwrap_or("");
        match std::fs::write(Path::new(path), content) {
            Ok(_) => ToolResult { success: true, output: Some(format!("Written to {}", path)), error: None },
            Err(e) => ToolResult { success: false, output: None, error: Some(e.to_string()) },
        }
    }
}