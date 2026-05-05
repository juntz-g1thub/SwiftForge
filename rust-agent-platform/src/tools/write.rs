use async_trait::async_trait;
use std::path::Path;
use crate::core::{Tool, ToolResult, ToolCall};

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str { "write" }
    fn description(&self) -> &str { "Write content to a file" }
    
    async fn execute(&self, call: ToolCall) -> ToolResult {
        let path = call.arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = call.arguments.get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        match std::fs::write(Path::new(path), content) {
            Ok(_) => ToolResult {
                success: true,
                output: Some(format!("Written to {}", path)),
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