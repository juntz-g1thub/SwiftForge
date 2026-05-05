use std::process::Command;
use async_trait::async_trait;
use crate::core::{Tool, ToolResult, ToolCall};

pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }
    fn description(&self) -> &str { "Execute shell commands" }
    
    async fn execute(&self, call: ToolCall) -> ToolResult {
        let command = call.arguments.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output();

        match output {
            Ok(out) => ToolResult {
                success: out.status.success(),
                output: Some(String::from_utf8_lossy(&out.stdout).to_string()),
                error: if out.status.success() { None } else { Some(String::from_utf8_lossy(&out.stderr).to_string()) },
            },
            Err(e) => ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }
}