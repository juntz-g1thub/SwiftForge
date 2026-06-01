# swiftforge-providers

LLM Provider 实现库，包含 OpenAI、Anthropic、DeepSeek、Ollama、MiniMax、Custom 等 Provider。

## 文档

| 文档 | 说明 |
|------|------|
| `docs/2026-05-29-provider-library-refactor-plan-impl.md` | Provider 库独立化重构实现计划 |
| `docs/2026-05-29-deepseek-api-reference.md` | DeepSeek API 参考文档 |

## 支持的 Provider

| Provider | 说明 |
|----------|------|
| `OpenAIProvider` | OpenAI GPT 系列 |
| `AnthropicProvider` | Anthropic Claude 系列 |
| `DeepSeekProvider` | DeepSeek V4/R1 系列 |
| `OllamaProvider` | Ollama 本地模型 |
| `MiniMaxProvider` | MiniMax 系列 |
| `CustomProvider` | 自定义 provider |

## 使用

```rust
use swiftforge_providers::{DeepSeekProvider, OpenAIProvider};
use swiftforge_provider_core::ProviderRegistry;

let mut registry = ProviderRegistry::new();
registry.register("deepseek", DeepSeekProvider::new(api_key, base_url, model));
```
