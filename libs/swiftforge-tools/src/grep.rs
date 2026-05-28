use async_trait::async_trait;
use std::path::Path;
use std::io::{self, BufRead};
use swiftforge_types::{Tool, ToolCall, ToolResult};

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }
    fn description(&self) -> &str { "Search for pattern in file" }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({ "type": "object", "properties": { "path": { "type": "string", "description": "Path to the file to search" }, "pattern": { "type": "string", "description": "Pattern to search for" } }, "required": ["path", "pattern"] })
    }
    async fn execute(&self, call: ToolCall) -> ToolResult {
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let pattern = call.arguments.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
        let file = match std::fs::File::open(Path::new(path)) {
            Ok(f) => f,
            Err(e) => return ToolResult { success: false, output: None, error: Some(e.to_string()) },
        };
        let reader = io::BufReader::new(file);
        let mut results = Vec::new();
        for (line_num, line) in reader.lines().enumerate() {
            if let Ok(content) = line {
                if content.contains(pattern) {
                    results.push(format!("{}: {}", line_num + 1, content));
                }
            }
        }
        ToolResult { success: true, output: Some(results.join("\n")), error: None }
    }
}