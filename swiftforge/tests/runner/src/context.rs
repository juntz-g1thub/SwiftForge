use std::collections::HashMap;
use swiftforge_types::{Message, ModelResponse, ToolResult};

#[derive(Debug, Clone)]
pub struct TestContext {
    pub messages: Vec<Message>,
    pub last_response: Option<ModelResponse>,
    pub tool_results: Vec<ToolResult>,
    pub snapshots: HashMap<String, serde_json::Value>,
    pub env: HashMap<String, String>,
}

impl TestContext {
    pub fn new(env: HashMap<String, String>) -> Self {
        Self {
            messages: Vec::new(),
            last_response: None,
            tool_results: Vec::new(),
            snapshots: HashMap::new(),
            env,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn set_response(&mut self, response: ModelResponse) {
        self.last_response = Some(response);
    }

    pub fn add_tool_result(&mut self, result: ToolResult) {
        self.tool_results.push(result);
    }

    pub fn get_last_response_content(&self) -> Option<String> {
        self.last_response.as_ref().map(|r| r.content.clone())
    }

    pub fn get_tool_result_count(&self) -> usize {
        self.tool_results.len()
    }

    pub fn save_snapshot(&mut self, name: &str, data: serde_json::Value) {
        self.snapshots.insert(name.to_string(), data);
    }

    pub fn reset_for_loop(&mut self) {
        // Keep messages but reset response state for next iteration
    }
}
