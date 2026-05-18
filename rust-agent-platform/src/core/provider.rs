use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub usage: Usage,
}

impl ModelResponse {
    pub fn new(content: String, usage: Usage) -> Self {
        Self {
            content,
            tool_calls: None,
            usage,
        }
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<serde_json::Value>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, messages: Vec<crate::core::session::Message>) -> anyhow::Result<ModelResponse>;
    fn name(&self) -> &str;
}