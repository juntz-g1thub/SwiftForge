# swiftforge-provider-core

Provider 核心库，定义 LLM Provider trait 和注册表。

## 文档

| 文档 | 说明 |
|------|------|
| `docs/2026-05-29-provider-library-refactor-plan.md` | Provider 库独立化重构计划 |

## 概述

- **Trait 定义**：`LLMProvider`, `ToolCallingProvider`
- **Registry**：`ProviderRegistry` 管理多个 provider
- **Error 类型**：`ProviderError` 包装各类 API 错误

## 核心 Trait

```rust
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
```
