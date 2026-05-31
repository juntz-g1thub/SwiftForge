# Provider 库独立化重构计划

> 版本: 2.0
> 日期: 2026-05-30
> 状态: L3 设计 (已定稿)
> 分支: feat/tui-refactor

---

## 一、重构目标

将 providers 从 `swiftforge` 主应用抽离为独立静态库，实现：

1. **库分离**: `swiftforge-provider-core` (trait + registry + error) + `swiftforge-providers` (实现)
2. **完全解耦**: Agent 通过 `dyn LLMProvider` / `dyn ToolCallingProvider` 与 provider 交互
3. **职责清理**: Agent 移除工具解析逻辑，移至 `swiftforge-tools`

---

## 二、关键设计决策

| # | 问题 | 选择 | 理由 |
|---|------|------|------|
| 1 | 分发机制 | **trait object** (`dyn LLMProvider`) | Registry 运行时多态、测试友好、二进制小 |
| 2 | Registry 关系 | **Registry 只负责构建** | Agent 独立持有 `Arc<dyn LLMProvider>`，生命周期不依赖 Registry |
| 3 | 错误类型 | **ProviderError 包装 anyhow** | 库内清晰枚举，应用层 `anyhow::Result` 改动最少 |
| 4 | Breaking change | **直接 breaking change** | SwiftForge 单一消费者，手动更新调用代码 |
| 5 | 实现顺序 | **自底向上** | core → providers → tools → main app，每步验证 |

---

## 三、目标架构

```
Workspace (SwiftForge)
├── libs/
│   ├── swiftforge-types/           # 类型定义 (已有)
│   ├── swiftforge-provider-core/   # NEW: Provider trait + Registry + Error
│   ├── swiftforge-providers/       # NEW: 具体 provider 实现
│   ├── swiftforge-tools/           # 工具解析 (已有，需新增 parser.rs)
│   └── ...
└── swiftforge/                      # 主应用 (移除 providers/)
```

---

## 四、新库详细设计

### 4.1 swiftforge-provider-core

**路径**: `libs/swiftforge-provider-core/`

**Cargo.toml**:
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

**模块结构**:
```
src/
├── lib.rs       # 公共导出
├── error.rs     # ProviderError
├── traits.rs    # LLMProvider, ToolCallingProvider
└── registry.rs  # ProviderRegistry
```

**error.rs - ProviderError**:
```rust
#[derive(Debug, thiserror::Error)]
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

// ProviderError <-> anyhow 双向转换
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

**traits.rs**:
```rust
use async_trait::async_trait;
use swiftforge_types::{Message, ModelResponse, ToolDefinition};

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> crate::error::Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> crate::error::Result<Vec<String>>;
    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>
    ) -> crate::error::Result<()>;
}

#[async_trait]
pub trait ToolCallingProvider: Send + Sync {
    async fn chat_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>
    ) -> crate::error::Result<ModelResponse>;

    fn provider_name(&self) -> &str;

    async fn stream_chat_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>
    ) -> crate::error::Result<()>;
}

pub type DynLLMProvider = Arc<dyn LLMProvider>;
pub type DynToolCallingProvider = Arc<dyn ToolCallingProvider>;
```

**registry.rs**:
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
    pub fn new() -> Self { ... }

    pub fn register<P: LLMProvider + 'static>(&mut self, name: &str, provider: P) { ... }
    pub fn register_with_tools<P: ToolCallingProvider + 'static>(&mut self, name: &str, provider: P) { ... }
    pub fn register_boxed(&mut self, name: &str, provider: DynLLMProvider) { ... }

    pub fn get(&self, name: &str) -> Option<&DynLLMProvider> { ... }
    pub fn get_tool_provider(&self, name: &str) -> Option<&DynToolCallingProvider> { ... }
    pub fn default(&self) -> Option<&DynLLMProvider> { ... }
    pub fn default_tool_provider(&self) -> Option<&DynToolCallingProvider> { ... }
    pub fn list_providers(&self) -> Vec<String> { ... }
    pub fn list_tool_providers(&self) -> Vec<String> { ... }
}

impl Default for ProviderRegistry { ... }
```

---

### 4.2 swiftforge-providers

**路径**: `libs/swiftforge-providers/`

**Cargo.toml**:
```toml
[package]
name = "swiftforge-providers"
version.workspace = true
edition.workspace = true

[dependencies]
swiftforge-provider-core = { path = "../swiftforge-provider-core" }
swiftforge-types = { path = "../swiftforge-types" }

async-trait = "0.1"
reqwest = "0.12"
tokio-stream = "0.1"
serde_json = "1"
chrono = "0.4"  # 仅 deepseek 用于调试日志
```

**模块结构**:
```
src/
├── lib.rs        # 公共导出 Provider structs
├── openai.rs
├── anthropic.rs
├── deepseek.rs
├── ollama.rs
├── minimax.rs
├── custom.rs
└── utils.rs     # SSE 解析等共享工具
```

**迁移要求**:
- 所有 `use crate::providers::{LLMProvider, ToolCallingProvider}` → `use swiftforge_provider_core::{LLMProvider, ToolCallingProvider}`
- 所有 `use crate::core::{...}` → `use swiftforge_types::{...}`
- `anyhow::Result` → `swiftforge_provider_core::error::Result`

---

### 4.3 swiftforge-tools 工具解析

**路径**: `libs/swiftforge-tools/src/parser.rs` (新建)

**解析格式支持**:
1. **OpenAI format**: `{"tool_calls": [{"function": {"name": "...", "arguments": "..."}}]}`
2. **DeepSeek format**: `<tool_call>{"name": "...", "arguments": {...}}</tool_call>`
3. **Anthropic format**: `{"name": "...", "input": {...}}`

**接口**:
```rust
pub struct ToolCallParser { ... }

impl ToolCallParser {
    pub fn new() -> Self;
    
    /// 从 content 中解析 tool_calls (支持 DeepSeek 的 <tool_call> 标签)
    pub fn parse(&self, content: &str) -> Vec<ToolCall>;

    /// 从 JSON 数组解析 (OpenAI format)
    pub fn parse_from_json(&self, tool_calls: &[serde_json::Value]) -> Vec<ToolCall>;

    /// 判断 content 是否包含工具调用
    pub fn has_tool_calls(&self, content: &str) -> bool;
}
```

---

## 五、swiftforge 主应用修改

### 5.1 删除的文件/目录

```
swiftforge/src/providers/     # 整个目录删除
```

### 5.2 core/agent.rs 修改

**Before**:
```rust
pub struct Agent {
    config: AgentConfig,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    providers: ProviderRegistry,  // 直接持有
    tool_registry: Option<Arc<ToolRegistry>>,
}
```

**After**:
```rust
pub struct Agent {
    config: AgentConfig,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    llm_provider: DynLLMProvider,                    // trait object
    tool_provider: Option<DynToolCallingProvider>, // optional
    tool_registry: Option<Arc<ToolRegistry>>,
    tool_parser: ToolCallParser,                    // 工具解析器
}
```

**方法变化**:

| Before | After |
|--------|-------|
| `with_provider()` | `with_llm_provider()` |
| `with_tool_provider()` | `with_tool_provider()` |
| `self.providers.default()` | 直接使用 `self.llm_provider` |
| `parse_tool_calls()` | 调用 `self.tool_parser.parse()` |
| `parse_tool_calls_from_json()` | 调用 `self.tool_parser.parse_from_json()` |

### 5.3 tui/app_controller.rs 修改

**Before**:
```rust
let provider = DeepSeekProvider::new(api_key, base_url, model);
agent = agent.with_provider("deepseek", provider);
agent = agent.with_tool_provider("deepseek", provider);
```

**After**:
```rust
use swiftforge_providers::DeepSeekProvider;
use swiftforge_provider_core::ProviderRegistry;

let mut registry = ProviderRegistry::new();
let deepseek = DeepSeekProvider::new(api_key, base_url, model);
registry.register("deepseek", deepseek.clone());
registry.register_with_tools("deepseek", deepseek);

let llm_provider = registry.default()
    .ok_or_else(|| anyhow::anyhow!("No provider"))?;
let tool_provider = registry.default_tool_provider();

agent = agent
    .with_llm_provider(llm_provider)
    .with_tool_provider(tool_provider);
```

---

## 六、文件变更清单

### 新建文件

| 文件 | 说明 |
|------|------|
| `libs/swiftforge-provider-core/Cargo.toml` | 包配置 |
| `libs/swiftforge-provider-core/src/lib.rs` | 公共导出 |
| `libs/swiftforge-provider-core/src/error.rs` | 错误类型 + anyhow 转换 |
| `libs/swiftforge-provider-core/src/traits.rs` | Trait 定义 |
| `libs/swiftforge-provider-core/src/registry.rs` | Registry 实现 |
| `libs/swiftforge-providers/Cargo.toml` | 包配置 |
| `libs/swiftforge-providers/src/lib.rs` | 公共导出 |
| `libs/swiftforge-providers/src/openai.rs` | OpenAI provider |
| `libs/swiftforge-providers/src/anthropic.rs` | Anthropic provider |
| `libs/swiftforge-providers/src/deepseek.rs` | DeepSeek provider |
| `libs/swiftforge-providers/src/ollama.rs` | Ollama provider |
| `libs/swiftforge-providers/src/minimax.rs` | MiniMax provider |
| `libs/swiftforge-providers/src/custom.rs` | Custom provider |
| `libs/swiftforge-providers/src/utils.rs` | 共享工具 |
| `libs/swiftforge-tools/src/parser.rs` | 工具解析器 |

### 删除文件

| 文件 | 说明 |
|------|------|
| `swiftforge/src/providers/mod.rs` | 旧 providers 模块 |
| `swiftforge/src/providers/openai.rs` | 旧 OpenAI provider |
| `swiftforge/src/providers/anthropic.rs` | 旧 Anthropic provider |
| `swiftforge/src/providers/deepseek.rs` | 旧 DeepSeek provider |
| `swiftforge/src/providers/ollama.rs` | 旧 Ollama provider |
| `swiftforge/src/providers/minimax.rs` | 旧 MiniMax provider |
| `swiftforge/src/providers/custom.rs` | 旧 Custom provider |

### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `swiftforge/src/core/agent.rs` | 使用 DynLLMProvider, 移除 parse_tool_calls |
| `swiftforge/src/core/mod.rs` | 清理 re-exports |
| `swiftforge/src/tui/app_controller.rs` | 使用 ProviderRegistry |
| `swiftforge/Cargo.toml` | 添加新依赖 |
| `libs/swiftforge-tools/src/lib.rs` | 导出 ToolCallParser |
| `libs/swiftforge-tools/src/parser.rs` | 新建 |
| workspace `Cargo.toml` | 添加新成员 |

---

## 七、验收标准

- [ ] `cargo build -p swiftforge-provider-core` 成功
- [ ] `cargo build -p swiftforge-providers` 成功
- [ ] `cargo build -p swiftforge-tools` 成功
- [ ] `cargo build` (整个 workspace) 成功
- [ ] `cargo test` 通过
- [ ] TUI 可以正常启动和对话
- [ ] 流式输出正常工作