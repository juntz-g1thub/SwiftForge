mod openai;
mod anthropic;
mod ollama;

pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;

pub use crate::core::{ModelResponse, Usage, Message};

use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
}