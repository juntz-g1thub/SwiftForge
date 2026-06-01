# Provider 库独立化重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 providers 从 swiftforge 主应用抽离为 `swiftforge-provider-core` + `swiftforge-providers` 两个独立库，Agent 通过 `dyn LLMProvider` 与 provider 交互。

**Architecture:** 自底向上：先建 core 库 → 建 providers 库 → 重构 tools → 最后改主应用。每步验证编译。

**Tech Stack:** Rust (async-trait, thiserror, reqwest, tokio-stream)

---

## 文件结构

```
libs/
├── swiftforge-provider-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── traits.rs
│       └── registry.rs
├── swiftforge-providers/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── openai.rs
│       ├── anthropic.rs
│       ├── deepseek.rs
│       ├── ollama.rs
│       ├── minimax.rs
│       ├── custom.rs
│       └── utils.rs
└── swiftforge-tools/src/
    └── parser.rs  (新建)
```

---

## Phase 1: 创建 swiftforge-provider-core

### Task 1: 创建 swiftforge-provider-core 库结构

**Files:**
- Create: `libs/swiftforge-provider-core/Cargo.toml`
- Create: `libs/swiftforge-provider-core/src/lib.rs`
- Create: `libs/swiftforge-provider-core/src/error.rs`
- Create: `libs/swiftforge-provider-core/src/traits.rs`
- Create: `libs/swiftforge-provider-core/src/registry.rs`
- Modify: `Cargo.toml` (workspace members 添加新成员)

- [ ] **Step 1: 创建目录结构**

```bash
mkdir -p libs/swiftforge-provider-core/src
```

- [ ] **Step 2: 创建 Cargo.toml**

```toml
[package]
name = "swiftforge-provider-core"
version.workspace = true
edition.workspace = true

[dependencies]
swiftforge-types = { path = "../swiftforge-types" }

async-trait = "0.1"
thiserror = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

- [ ] **Step 3: 创建 src/error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("No provider configured")]
    NoProvider,

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ProviderError>;

impl From<anyhow::Error> for ProviderError {
    fn from(e: anyhow::Error) -> Self {
        ProviderError::Other(e.to_string())
    }
}

impl From<ProviderError> for anyhow::Error {
    fn from(e: ProviderError) -> Self {
        anyhow::anyhow!("{:?}", e)
    }
}
```

- [ ] **Step 4: 创建 src/traits.rs**

```rust
use std::sync::Arc;
use async_trait::async_trait;
use swiftforge_types::{Message, ModelResponse, ToolDefinition};
use crate::error::Result;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>
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
        on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>
    ) -> Result<()>;
}

pub type DynLLMProvider = Arc<dyn LLMProvider>;
pub type DynToolCallingProvider = Arc<dyn ToolCallingProvider>;
```

- [ ] **Step 5: 创建 src/registry.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use crate::error::{ProviderError, Result};
use crate::traits::{DynLLMProvider, DynToolCallingProvider, LLMProvider, ToolCallingProvider};

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
```

- [ ] **Step 6: 创建 src/lib.rs**

```rust
pub mod error;
pub mod traits;
pub mod registry;

pub use error::{ProviderError, Result};
pub use traits::{DynLLMProvider, DynToolCallingProvider, LLMProvider, ToolCallingProvider};
pub use registry::ProviderRegistry;
```

- [ ] **Step 7: 更新 workspace Cargo.toml**

在 `[workspace.members]` 中添加 `"libs/swiftforge-provider-core"`

- [ ] **Step 8: 验证编译**

```bash
cd .worktrees/feat-tui-refactor && cargo build -p swiftforge-provider-core
```

Expected: 编译成功

---

## Phase 2: 创建 swiftforge-providers

### Task 2: 创建 swiftforge-providers 库结构

**Files:**
- Create: `libs/swiftforge-providers/Cargo.toml`
- Create: `libs/swiftforge-providers/src/lib.rs`
- Create: `libs/swiftforge-providers/src/utils.rs`
- Modify: workspace `Cargo.toml` (members 添加)

- [ ] **Step 1: 创建目录结构**

```bash
mkdir -p libs/swiftforge-providers/src
```

- [ ] **Step 2: 创建 Cargo.toml**

```toml
[package]
name = "swiftforge-providers"
version.workspace = true
edition.workspace = true

[dependencies]
swiftforge-provider-core = { path = "../swiftforge-provider-core" }
swiftforge-types = { path = "../swiftforge-types" }

async-trait = "0.1"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
tokio-stream = "0.1"
serde_json = "1"
chrono = "0.4"
```

- [ ] **Step 3: 创建 src/lib.rs**

```rust
pub mod utils;

pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use deepseek::DeepSeekProvider;
pub use minimax::MiniMaxProvider;
pub use custom::CustomProvider;
```

- [ ] **Step 4: 创建 src/utils.rs**

SSE 解析等共享工具（从原 providers 迁移）

- [ ] **Step 5: 更新 workspace Cargo.toml**

在 `[workspace.members]` 中添加 `"libs/swiftforge-providers"`

- [ ] **Step 6: 验证编译**

```bash
cargo build -p swiftforge-providers
```

Expected: 编译成功（虽然有 missing imports 错误，后续 Task 会修复）

---

### Task 3: 迁移 openai.rs

**Files:**
- Create: `libs/swiftforge-providers/src/openai.rs`

- [ ] **Step 1: 迁移 src/openai.rs**

从 `swiftforge/src/providers/openai.rs` 复制并修改 imports：
- `use crate::providers::{LLMProvider, ToolCallingProvider}` → `use swiftforge_provider_core::{LLMProvider, ToolCallingProvider}`
- `use crate::core::{ModelResponse, Usage, Message, ToolDefinition}` → `use swiftforge_types::{ModelResponse, Usage, Message, ToolDefinition}`
- `use anyhow::{Result, Context}` → `use swiftforge_provider_core::error::Result` + `use anyhow::Context` (保留 Context 需要 anyhow)

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge-providers
```

Expected: openai.rs 编译成功

---

### Task 4: 迁移 anthropic.rs

**Files:**
- Create: `libs/swiftforge-providers/src/anthropic.rs`

- [ ] **Step 1: 迁移 src/anthropic.rs**

同 Task 3 的迁移模式

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge-providers
```

---

### Task 5: 迁移 deepseek.rs

**Files:**
- Create: `libs/swiftforge-providers/src/deepseek.rs`

- [ ] **Step 1: 迁移 src/deepseek.rs**

同 Task 3 的迁移模式（保留 chrono 调试日志）

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge-providers
```

---

### Task 6: 迁移 ollama.rs

**Files:**
- Create: `libs/swiftforge-providers/src/ollama.rs`

- [ ] **Step 1: 迁移 src/ollama.rs**

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge-providers
```

---

### Task 7: 迁移 minimax.rs

**Files:**
- Create: `libs/swiftforge-providers/src/minimax.rs`

- [ ] **Step 1: 迁移 src/minimax.rs**

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge-providers
```

---

### Task 8: 迁移 custom.rs

**Files:**
- Create: `libs/swiftforge-providers/src/custom.rs`

- [ ] **Step 1: 迁移 src/custom.rs**

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge-providers
```

---

## Phase 3: 重构 swiftforge-tools (工具解析)

### Task 9: 创建 ToolCallParser

**Files:**
- Create: `libs/swiftforge-tools/src/parser.rs`
- Modify: `libs/swiftforge-tools/src/lib.rs` (添加导出)

- [ ] **Step 1: 创建 src/parser.rs**

```rust
use std::collections::HashMap;
use swiftforge_types::ToolCall;
use serde_json::Value as JsonValue;

pub struct ToolCallParser { re: regex::Regex }

impl ToolCallParser {
    pub fn new() -> Self {
        let re = regex::Regex::new(
            r#"<tool_call>\s*\{[^}]*?"name"\s*:\s*"([^"]+)"[^}]*?"arguments"\s*:\s*(\{[^}]+\})[^}]*\}</tool_call>"#
        ).expect("Invalid regex");
        Self { re }
    }

    /// 从 content 中解析 tool_calls (支持 DeepSeek 的 <tool_call> 标签)
    pub fn parse(&self, content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        
        // 先尝试 JSON 解析
        if let Ok(json) = serde_json::from_str::<JsonValue>(content) {
            if let Some(tool_calls) = json.get("tool_calls").and_then(|t| t.as_array()) {
                for call in tool_calls {
                    if let (Some(name), Some(args)) = (
                        call.get("name").and_then(|n| n.as_str()),
                        call.get("arguments")
                    ) {
                        let arguments = Self::parse_arguments(args);
                        calls.push(ToolCall { name: name.to_string(), arguments });
                    }
                }
            }
        }
        
        // 尝试 DeepSeek <tool_call> 标签
        for cap in self.re.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("{}");
            let arguments: HashMap<String, JsonValue> = serde_json::from_str(args_str).unwrap_or_default();
            if !name.is_empty() {
                calls.push(ToolCall { name, arguments });
            }
        }
        
        calls
    }

    /// 从 JSON 数组解析 (OpenAI format)
    pub fn parse_from_json(&self, tool_calls: &[JsonValue]) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        for call in tool_calls {
            let name = call.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .or_else(|| call.get("name").and_then(|n| n.as_str()));

            let args = call.get("function")
                .and_then(|f| f.get("arguments"))
                .or_else(|| call.get("arguments"));

            if let (Some(name), Some(args)) = (name, args) {
                let arguments = Self::parse_arguments(args);
                calls.push(ToolCall { name: name.to_string(), arguments });
            }
        }
        calls
    }

    /// 判断 content 是否包含工具调用
    pub fn has_tool_calls(&self, content: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<JsonValue>(content) {
            if json.get("tool_calls").is_some() {
                return true;
            }
        }
        content.contains("<tool_call>")
    }

    fn parse_arguments(args: &JsonValue) -> HashMap<String, JsonValue> {
        if let JsonValue::Object(map) = args {
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else if let JsonValue::String(s) = args {
            serde_json::from_str(s).unwrap_or_default()
        } else {
            HashMap::new()
        }
    }
}

impl Default for ToolCallParser {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 更新 src/lib.rs 添加导出**

在 `lib.rs` 中添加 `pub mod parser;` 和 `pub use parser::ToolCallParser;`

- [ ] **Step 3: 验证编译**

```bash
cargo build -p swiftforge-tools
```

---

## Phase 4: 重构 swiftforge 主应用

### Task 10: 删除旧 providers 目录

**Files:**
- Delete: `swiftforge/src/providers/` 整个目录

- [ ] **Step 1: 删除 providers 目录**

```bash
rm -rf swiftforge/src/providers/
```

- [ ] **Step 2: 验证 git 状态**

确认删除成功

---

### Task 11: 重构 core/agent.rs

**Files:**
- Modify: `swiftforge/src/core/agent.rs`

- [ ] **Step 1: 修改 imports**

```rust
// Before:
use crate::providers::{LLMProvider, ProviderRegistry, ToolCallingProvider};

// After:
use swiftforge_provider_core::{
    DynLLMProvider, DynToolCallingProvider, LLMProvider, ProviderRegistry, ToolCallingProvider
};
use swiftforge_tools::ToolCallParser;
```

- [ ] **Step 2: 修改 Agent struct**

```rust
// Before:
pub struct Agent {
    config: AgentConfig,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    providers: ProviderRegistry,
    tool_registry: Option<Arc<ToolRegistry>>,
}

// After:
pub struct Agent {
    config: AgentConfig,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    llm_provider: DynLLMProvider,
    tool_provider: Option<DynToolCallingProvider>,
    tool_registry: Option<Arc<ToolRegistry>>,
    tool_parser: ToolCallParser,
}
```

- [ ] **Step 3: 修改构造方法**

```rust
// with_provider -> with_llm_provider
pub fn with_llm_provider(mut self, provider: DynLLMProvider) -> Self {
    self.llm_provider = provider;
    self
}

// with_tool_provider
pub fn with_tool_provider(mut self, provider: Option<DynToolCallingProvider>) -> Self {
    self.tool_provider = provider;
    self
}
```

- [ ] **Step 4: 移除 parse_tool_calls 方法，改为使用 tool_parser**

```rust
pub fn parse_tool_calls(&self, content: &str) -> Vec<ToolCall> {
    self.tool_parser.parse(content)
}

pub fn parse_tool_calls_from_json(&self, tool_calls: &[serde_json::Value]) -> Vec<ToolCall> {
    self.tool_parser.parse_from_json(tool_calls)
}
```

- [ ] **Step 5: 修改 chat 方法使用 self.llm_provider**

- [ ] **Step 6: 修改 chat_with_tools 方法使用 self.tool_provider**

- [ ] **Step 7: 验证编译**

```bash
cargo build -p swiftforge
```

Expected: 编译失败（缺少 ProviderRegistry 和相关字段），继续修改

---

### Task 12: 重构 core/mod.rs

**Files:**
- Modify: `swiftforge/src/core/mod.rs`

- [ ] **Step 1: 清理 re-exports**

移除 providers 相关的 re-exports

- [ ] **Step 2: 验证编译**

```bash
cargo build -p swiftforge
```

---

### Task 13: 重构 tui/app_controller.rs

**Files:**
- Modify: `swiftforge/src/tui/app_controller.rs`

- [ ] **Step 1: 修改 imports**

```rust
// Before:
use crate::providers::{OpenAIProvider, AnthropicProvider, OllamaProvider, DeepSeekProvider, MiniMaxProvider, CustomProvider};

// After:
use swiftforge_providers::{OpenAIProvider, AnthropicProvider, OllamaProvider, DeepSeekProvider, MiniMaxProvider, CustomProvider};
use swiftforge_provider_core::ProviderRegistry;
```

- [ ] **Step 2: 修改 provider 初始化逻辑**

在 `new()` 方法中：
1. 从 ConfigManager 获取 provider 配置
2. 创建 ProviderRegistry 并注册 provider
3. 从 registry 获取 default provider
4. 调用 `agent.with_llm_provider()` 和 `agent.with_tool_provider()`

- [ ] **Step 3: 验证编译**

```bash
cargo build -p swiftforge
```

---

### Task 14: 更新 swiftforge/Cargo.toml

**Files:**
- Modify: `swiftforge/Cargo.toml`

- [ ] **Step 1: 添加新依赖**

```toml
swiftforge-provider-core = { path = "../libs/swiftforge-provider-core" }
swiftforge-providers = { path = "../libs/swiftforge-providers" }
swiftforge-tools = { path = "../libs/swiftforge-tools" }
```

- [ ] **Step 2: 验证编译**

```bash
cargo build
```

---

## Phase 5: 验证与测试

### Task 15: 最终验证

- [ ] **Step 1: 运行测试**

```bash
cargo test
```

Expected: 所有测试通过

- [ ] **Step 2: 验证 TUI 启动**

手动运行 `cargo run` 确认 TUI 正常启动

- [ ] **Step 3: 验证流式输出**

发送一条消息，确认流式输出正常显示

---

## 验收标准

- [ ] `cargo build -p swiftforge-provider-core` 成功
- [ ] `cargo build -p swiftforge-providers` 成功
- [ ] `cargo build -p swiftforge-tools` 成功
- [ ] `cargo build` (整个 workspace) 成功
- [ ] `cargo test` 通过
- [ ] TUI 可以正常启动和对话
- [ ] 流式输出正常工作