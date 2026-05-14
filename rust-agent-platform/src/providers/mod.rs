mod openai;
mod anthropic;
mod ollama;
mod deepseek;
mod minimax;
mod custom;

pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use deepseek::DeepSeekProvider;
pub use minimax::MiniMaxProvider;
pub use custom::CustomProvider;

pub use crate::core::{ModelResponse, Usage, Message};

use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream_chat<F>(&self, messages: Vec<Message>, on_chunk: F) -> Result<()>
    where
        F: FnMut(String) + Send + Sync;
}