# Rust Agent Platform - 接口规范文档

> 文档版本: 1.0
> 生成日期: 2026-05-19
> 分支: feature/tui-refactor
> Worktree: `.worktrees/tui-refactor/`
> 状态: **初稿 - 待完善**
>
> **重要说明**: 本文档始终描述**目标架构 (Target Architecture)**，而非当前实现状态。
> 目标架构是模块应该达到的设计目标，实现可能落后于文档，需要持续更新文档以反映目标而非现状。

---

## 一、文档目的

本文档定义 Rust Agent Platform 的标准化接口规范，作为所有模块开发的参考依据。

**目标**:
1. 统一接口设计，避免各模块各行其是
2. 提供明确的类型定义和 trait 签名
3. 记录模块间的依赖关系和数据流
4. 后续优化和扩展的基准文档

---

## 二、模块总览

```
rust-agent-platform/src/
├── lib.rs              # 库入口，公共导出
├── core/               # 核心类型 (Agent, Tool, Session, Provider)
├── providers/          # LLM Provider 接口和实现
├── tools/              # 内置工具 (bash, read, write, edit, grep)
├── tui/                # 终端用户界面
├── platform/           # 平台功能 (boulder, hooks, skill, intent_gate)
├── orchestration/      # 多智能体编排
└── integration/        # 外部集成 (MCP)
```

---

## 三、Core 模块接口

### 3.1 Agent (core/agent.rs)

**类型定义**:

```rust
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub model: Option<String>,
    pub temperature: f32,
}

pub enum AgentRole {
    Orchestrator,
    Executor,
    Planner,
    Advisor,
    Explorer,
    Librarian,
}
```

**公开方法**:

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(config: AgentConfig) -> Self` | 创建 Agent |
| `with_provider` | `fn with_provider<P: LLMProvider + 'static>(self, name: &str, provider: P) -> Self` | 添加 LLM Provider |
| `with_tool_provider` | `fn with_tool_provider<P: ToolCallingProvider + 'static>(self, name: &str, provider: P) -> Self` | 添加 Tool-Calling Provider |
| `with_tool_registry` | `fn with_tool_registry(self, registry: Arc<ToolRegistry>) -> Self` | 设置工具注册表 |
| `with_scheduler` | `fn with_scheduler(self, scheduler: Arc<TaskScheduler>) -> Self` | 设置任务调度器 |
| `with_message_bus` | `fn with_message_bus(self, message_bus: Arc<MessageBus>) -> Self` | 设置消息总线 |
| `name` | `fn name(&self) -> &str` | 获取 Agent 名称 |
| `role` | `fn role(&self) -> &AgentRole` | 获取 Agent 角色 |
| `config` | `fn config(&self) -> &AgentConfig` | 获取配置 |
| `list_providers` | `fn list_providers(&self) -> Vec<String>` | 列出所有 Provider |
| `list_tool_providers` | `fn list_tool_providers(&self) -> Vec<String>` | 列出所有 Tool Provider |
| `list_tools` | `fn list_tools(&self) -> Vec<String>` | 列出所有工具 |
| `get_tool_definitions` | `fn get_tool_definitions(&self) -> Vec<ToolDefinition>` | 获取工具定义列表 |
| `call_tool` | `async fn call_tool(&self, name: &str, arguments: JsonValue) -> Result<ToolResult>` | 调用工具 |
| `parse_tool_calls` | `fn parse_tool_calls(&self, content: &str) -> Vec<ToolCall>` | 从内容解析工具调用 |
| `execute_tool_calls` | `async fn execute_tool_calls(&self, calls: Vec<ToolCall>) -> Result<Vec<ToolResult>>` | 执行工具调用列表 |
| `chat` | `async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>` | 普通聊天 |
| `chat_with` | `async fn chat_with(&self, provider_name: &str, messages: Vec<Message>) -> Result<ModelResponse>` | 使用指定 Provider 聊天 |
| `chat_with_tools` | `async fn chat_with_tools(&self, messages, debug_log, debug_ui) -> Result<ModelResponse>` | 带工具的聊天 |
| `chat_with_tools_streaming` | `async fn chat_with_tools_streaming<F>(&self, messages, debug_log, debug_ui, on_chunk: F) -> Result<ModelResponse>` | 流式工具调用 |
| `run_agent_loop` | `async fn run_agent_loop(&self, initial_message: &str, max_iterations: usize, debug_log: Option<String>, debug_ui: Option<Sender<String>>, stream_ui: Option<Sender<Result<String>>>) -> Result<String>` | **核心循环** |
| `list_models` | `async fn list_models(&self) -> Result<Vec<String>>` | 列出可用模型 |

---

### 3.2 Tool (core/tool.rs)

**类型定义**:

```rust
pub struct ToolCall {
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

pub struct ToolResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

**Trait 定义**:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn execute(&self, call: ToolCall) -> ToolResult;
}
```

**ToolRegistry**:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self;
    pub fn register<T: Tool + 'static>(&mut self, tool: T);
    pub async fn execute(&self, call: ToolCall) -> ToolResult;
    pub fn list_tools(&self) -> Vec<String>;
    pub fn get_definitions(&self) -> Vec<ToolDefinition>;
}
```

---

### 3.3 Session (core/session.rs)

**类型定义**:

```rust
pub struct Message {
    pub role: String,      // "user" | "assistant" | "system"
    pub content: String,
}

pub struct Session {
    messages: VecDeque<Message>,
    context_window: usize,
}

pub struct SessionConfig {
    pub max_tokens: Option<usize>,
    pub context_window: usize,
}
```

---

### 3.4 Provider (core/provider.rs)

**类型定义**:

```rust
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

pub struct ModelResponse {
    pub content: String,
    pub tool_calls: Option<Vec<serde_json::Value>>,
    pub usage: Usage,
}

pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

---

## 四、Providers 模块接口

### 4.1 LLMProvider Trait

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream_chat(&self, messages: Vec<Message>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()>;
}
```

### 4.2 ToolCallingProvider Trait

```rust
#[async_trait]
pub trait ToolCallingProvider: Send + Sync {
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()>;
}
```

### 4.3 ProviderRegistry

```rust
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    tool_providers: HashMap<String, Arc<dyn ToolCallingProvider>>,
    default_provider: Option<String>,
}

impl ProviderRegistry {
    pub fn new() -> Self;
    pub fn register<P: LLMProvider + 'static>(&mut self, name: &str, provider: P);
    pub fn register_with_tools<P: ToolCallingProvider + 'static>(&mut self, name: &str, provider: P);
    pub fn get(&self, name: &str) -> Option<&Arc<dyn LLMProvider>>;
    pub fn get_tool_provider(&self, name: &str) -> Option<&Arc<dyn ToolCallingProvider>>;
    pub fn default(&self) -> Option<&Arc<dyn LLMProvider>>;
    pub fn default_tool_provider(&self) -> Option<&Arc<dyn ToolCallingProvider>>;
    pub fn list_providers(&self) -> Vec<String>;
    pub fn list_tool_providers(&self) -> Vec<String>;
}
```

### 4.4 Provider 实现列表

| Provider | 文件 | 特点 |
|----------|------|------|
| OpenAIProvider | providers/openai.rs | 标准 tool_calls JSON |
| AnthropicProvider | providers/anthropic.rs | 标准 tool_calls JSON |
| DeepSeekProvider | providers/deepseek.rs | **文本标签格式** `<tool>name</tool>` + reasoning_content |
| OllamaProvider | providers/ollama.rs | 无 tool calling |
| MiniMaxProvider | providers/minimax.rs | 待完善 |
| CustomProvider | providers/custom.rs | 自定义端点 |

---

## 五、Tools 模块接口

### 5.1 工具列表

| 工具 | 类名 | 功能 |
|------|------|------|
| Bash | `BashTool` | 执行 shell 命令 |
| Read | `ReadTool` | 读取文件内容 |
| Write | `WriteTool` | 写入文件内容 |
| Edit | `EditTool` | 行级别编辑 |
| Grep | `GrepTool` | 文本搜索 |

### 5.2 Tool Trait 实现要求

所有工具必须实现:

1. **`name()`** - 返回工具唯一标识符 (如 "bash", "read")
2. **`description()`** - 返回工具功能描述
3. **`input_schema()`** - 返回 JSON Schema 格式的参数定义
4. **`execute(call: ToolCall) -> ToolResult`** - 异步执行工具调用

---

## 六、TUI 模块接口

### 6.1 View Trait

```rust
pub trait View {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState);
    fn handle_key(&mut self, key: KeyEvent, ctx: &AppContext) -> Option<Action>;
    fn on_enter(&mut self) {}
    fn on_exit(&mut self) {}
}
```

### 6.2 Action 枚举

```rust
pub enum Action {
    SendMessage(String),
    CancelStreaming,
    AppendMessage(String, String),
    SwitchView(ViewState),
    GoBack,
    ScrollUp,
    ScrollDown,
    ScrollDebugUp,
    ScrollDebugDown,
    ResetScroll,
    InputChar(char),
    InputBackspace,
    InputDelete,
    InputHome,
    InputEnd,
    InputLeft,
    InputRight,
    ClearInput,
    SelectProvider(String),
    SaveApiKey(String),
    SaveModel(String),
    SaveBaseUrl(String),
    FetchModels,
    SelectModel(String),
    ToggleDebug,
    Quit,
}
```

### 6.3 ViewState 枚举

```rust
pub enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
    Debug(DebugViewState),
}

pub enum ViewStateKind {
    Chat,
    Config,
    Debug,
}
```

### 6.4 视图状态类型

```rust
pub struct ChatViewState {
    pub messages: Vec<(String, String)>,
    pub input: String,
    pub cursor_pos: usize,
    pub is_streaming: bool,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub streaming_text: Option<String>,
    pub current_provider: String,
    pub current_model: String,
}

pub enum ConfigViewState {
    SelectProvider,
    Editing(ProviderEditStage),
    FetchingModels { error: Option<String> },
    SelectModel(Vec<String>),
}

pub enum ProviderEditStage {
    SelectProvider,
    ApiKey,
    Model,
    BaseUrl,
    CustomName,
    CustomUrl,
}

pub struct DebugViewState {
    pub messages: Vec<String>,
    pub scroll_offset: usize,
    pub content_height: usize,
}
```

### 6.5 AppContext 和 UIState

```rust
#[derive(Clone)]
pub struct AppContext {
    pub agent: Arc<Agent>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub tool_registry: Arc<ToolRegistry>,
    pub debug_log_path: Option<PathBuf>,
}

pub struct UIState {
    pub streaming_text: Arc<Mutex<Option<String>>>,
    pub debug_messages: Arc<Mutex<Vec<String>>>,
    pub response_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<String, anyhow::Error>>>>>,
    pub agent_command_tx: Arc<Mutex<Option<mpsc::Sender<AgentCommand>>>>,
    pub finalized_message: Arc<Mutex<Option<(String, String)>>>,
    pub debug_tx: Arc<Mutex<Option<mpsc::Sender<String>>>>,
}
```

---

## 七、问题记录

### 7.1 接口不一致问题

| 问题 | 严重程度 | 说明 |
|------|----------|------|
| DeepSeek tool call 格式不同于其他 Provider | **高** | DeepSeek 输出 `<tool>name</tool>` 文本标签，而非结构化 JSON |
| reasoning_content 未反馈给 DeepSeek | **高** | DeepSeek 需要将 reasoning_content 与 tool 结果一起发送 |
| Session 未被实际使用 | 中 | Session 定义了但 Agent 直接持有 Vec<Message> |

### 7.2 建议的标准化

1. **ToolCall 格式统一**: 所有 Provider 输出必须映射到统一的 `ToolCall` 结构
2. **Reasoning 累积**: 单独的 `reasoning_history: Vec<String>` 字段
3. **MCP Bridge**: `MCPToolAdapter` 将 MCP Tool 转换成内部 `ToolDefinition` 格式

---

## 八、文件路径索引

### 核心文件

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/lib.rs` | 9 | 库入口 |
| `src/core/mod.rs` | 9 | Core 模块导出 |
| `src/core/agent.rs` | 553 | **核心 Agent 类型** |
| `src/core/tool.rs` | 87 | Tool trait 和注册表 |
| `src/core/session.rs` | ~100 | Session 和 Message |
| `src/core/provider.rs` | ~100 | Provider 基础类型 |

### Providers

| 文件 | 说明 |
|------|------|
| `src/providers/mod.rs` | Trait 定义和 ProviderRegistry |
| `src/providers/openai.rs` | OpenAI 实现 |
| `src/providers/anthropic.rs` | Anthropic 实现 |
| `src/providers/deepseek.rs` | DeepSeek 实现 |
| `src/providers/ollama.rs` | Ollama 实现 |
| `src/providers/minimax.rs` | MiniMax 实现 |
| `src/providers/custom.rs` | Custom 实现 |

### TUI

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/tui/mod.rs` | 10 | 模块导出 |
| `src/tui/app_controller.rs` | 413 | **AppController** |
| `src/tui/state/action.rs` | 38 | Action 枚举 |
| `src/tui/state/view_state.rs` | 120 | 视图状态类型 |
| `src/tui/state/app_context.rs` | 100 | AppContext, UIState |
| `src/tui/views/view.rs` | 11 | View trait |
| `src/tui/views/chat_view.rs` | ~600 | ChatView |
| `src/tui/views/config_view.rs` | ~400 | ConfigView |
| `src/tui/views/debug_view.rs` | ~200 | DebugView |

---

## 九、待完善

1. **Platform 模块接口** - hooks, skill, intent_gate, category 的接口文档
2. **Orchestration 模块接口** - TaskScheduler, MessageBus, AgentMessage
3. **Integration 模块接口** - MCP protocol 和 client
4. **错误处理规范** - 统一的错误类型和传播方式
5. **日志规范** - tracing 的使用标准

---

*文档状态: 初稿*
*最后更新: 2026-05-19*