use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::Message;

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub usage: Usage,
}

impl ModelResponse {
    pub fn new(content: String, usage: Usage) -> Self {
        Self {
            content,
            reasoning_content: None,
            tool_calls: None,
            usage,
        }
    }
    pub fn with_tool_calls(mut self, tool_calls: Vec<serde_json::Value>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }
    pub fn with_reasoning_content(mut self, reasoning: String) -> Self {
        self.reasoning_content = Some(reasoning);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone)]
pub enum StreamingChunk {
    Reasoning(String),
    Content(String),
    ToolCall { name: String, arguments: String },
}

impl StreamingChunk {
    pub fn is_reasoning(&self) -> bool {
        matches!(self, StreamingChunk::Reasoning(_))
    }
    pub fn is_content(&self) -> bool {
        matches!(self, StreamingChunk::Content(_))
    }
    pub fn is_tool_call(&self) -> bool {
        matches!(self, StreamingChunk::ToolCall { .. })
    }
    pub fn into_content(self) -> Option<String> {
        match self {
            StreamingChunk::Content(s) => Some(s),
            _ => None,
        }
    }
    pub fn into_reasoning(self) -> Option<String> {
        match self {
            StreamingChunk::Reasoning(s) => Some(s),
            _ => None,
        }
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<ModelResponse>;
    fn name(&self) -> &str;
}
