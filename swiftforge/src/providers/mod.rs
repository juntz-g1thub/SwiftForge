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

pub use crate::core::{ModelResponse, Usage, Message, ToolDefinition};

use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream_chat(&self, messages: Vec<Message>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()>;
}

#[async_trait]
pub trait ToolCallingProvider: Send + Sync {
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()>;
}

pub type DynLLMProvider = Arc<dyn LLMProvider>;
pub type DynToolCallingProvider = Arc<dyn ToolCallingProvider>;

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, DynLLMProvider>,
    tool_providers: HashMap<String, DynToolCallingProvider>,
    default_provider: Option<String>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            tool_providers: HashMap::new(),
            default_provider: None,
        }
    }

    pub fn register<P: LLMProvider + 'static>(&mut self, name: &str, provider: P) {
        self.providers.insert(name.to_string(), Arc::new(provider));
        if self.default_provider.is_none() {
            self.default_provider = Some(name.to_string());
        }
    }

    pub fn register_with_tools<P: ToolCallingProvider + 'static>(&mut self, name: &str, provider: P) {
        self.tool_providers.insert(name.to_string(), Arc::new(provider));
        if self.default_provider.is_none() {
            self.default_provider = Some(name.to_string());
        }
    }

    pub fn register_boxed(&mut self, name: &str, provider: DynLLMProvider) {
        self.providers.insert(name.to_string(), provider);
        if self.default_provider.is_none() {
            self.default_provider = Some(name.to_string());
        }
    }

    pub fn get(&self, name: &str) -> Option<&DynLLMProvider> {
        self.providers.get(name)
    }

    pub fn get_tool_provider(&self, name: &str) -> Option<&DynToolCallingProvider> {
        self.tool_providers.get(name)
    }

    pub fn default(&self) -> Option<&DynLLMProvider> {
        self.default_provider.as_ref().and_then(|n| self.providers.get(n))
    }

    pub fn default_tool_provider(&self) -> Option<&DynToolCallingProvider> {
        self.default_provider.as_ref().and_then(|n| self.tool_providers.get(n))
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn list_tool_providers(&self) -> Vec<String> {
        self.tool_providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
