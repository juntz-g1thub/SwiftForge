use async_trait::async_trait;
use std::path::Path;
use crate::core::{Tool, ToolResult, ToolCall};

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str { "edit" }
    fn description(&self) -> &str { "Edit file contents (replace oldString with newString)" }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "oldString": {
                    "type": "string",
                    "description": "The string to find and replace"
                },
                "newString": {
                    "type": "string",
                    "description": "The new string to replace with"
                }
            },
            "required": ["path", "oldString", "newString"]
        })
    }

    async fn execute(&self, call: ToolCall) -> ToolResult {
        let path = call.arguments.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let old_string = call.arguments.get("oldString")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new_string = call.arguments.get("newString")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let content = match std::fs::read_to_string(Path::new(path)) {
            Ok(c) => c,
            Err(e) => return ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        };
        
        if !content.contains(old_string) {
            return ToolResult {
                success: false,
                output: None,
                error: Some(format!("oldString '{}' not found in file", old_string)),
            };
        }
        
        let new_content = content.replace(old_string, new_string);
        
        match std::fs::write(Path::new(path), new_content) {
            Ok(_) => ToolResult {
                success: true,
                output: Some(format!("Edited {}", path)),
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