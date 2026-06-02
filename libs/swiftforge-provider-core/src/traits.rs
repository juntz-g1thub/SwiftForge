use std::sync::Arc;
use async_trait::async_trait;
use swiftforge_types::{Message, ModelResponse, ToolDefinition, StreamingChunk};
use crate::error::Result;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        on_chunk: Box<dyn FnMut(StreamingChunk) + Send + Sync + 'static>
    ) -> Result<()>;
}

#[async_trait]
pub trait ToolCallingProvider: Send + Sync {
    async fn chat_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>
    ) -> Result<ModelResponse>;

    fn provider_name(&self) -> &str;

    async fn stream_chat_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        on_chunk: Box<dyn FnMut(StreamingChunk) + Send + Sync + 'static>
    ) -> Result<()>;
}

pub type DynLLMProvider = Arc<dyn LLMProvider>;
pub type DynToolCallingProvider = Arc<dyn ToolCallingProvider>;