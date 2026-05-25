# Rust Agent Platform - 架构与接口规范

> 文档版本: 2.0
> 生成日期: 2026-05-19
> 更新日期: 2026-05-23
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **进行中**

> **重要说明**: 本文档描述**目标架构 (Target Architecture)**，实现可能落后于文档，需要持续更新以反映目标。

---

## 目录

| 章节 | 内容 | 行数 |
|------|------|------|
| [一、架构总览](#一架构总览) | 系统整体架构图 | [33-126](#一架构总览) |
| [二、问题状态汇总](#二问题状态汇总) | 各模块问题与状态 | [128-141](#二问题状态汇总) |
| [三、核心类型定义](#三核心类型定义) | Agent、Tool 类型定义 | [144-246](#三核心类型定义) |
| [四、MCP 工具统一架构](#四mcp-工具统一架构) | MCP 适配层设计 | [250-411](#四mcp-工具统一架构) |
| [五、Provider 接口](#五provider-接口) | LLM/TC Provider Trait | [415-463](#五provider-接口) |
| [六、TUI 模块](#六tui-模块) | View/Action/ViewState | [466-578](#六tui-模块) |
| [七、Platform 模块](#七platform-模块) | IntentGate/Hooks/Skill/Boulder | [581-658](#七platform-模块) |
| [八、Orchestration 模块](#八orchestration-模块) | TaskScheduler/MessageBus | [661-704](#八orchestration-模块) |
| [九、Log 模块](#九log-模块) | 日志系统设计 | [708-759](#九log-模块) |
| [十、文件路径索引](#十文件路径索引) | 所有源文件路径 | [762-810](#十文件路径索引) |
| [附录A: 架构设计任务跟踪](../specs/2026-05-23-architecture-tasks.md) | 重构任务清单与讨论进度 | - |

---

## 一、架构总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              main.rs                                          │
│                         (Binary Entry Point)                                  │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            TUI Layer (MVC)                                   │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │  AppController                                                      │    │
│  │  ├── current_view: Box<dyn View>   (ChatView / ConfigView)          │    │
│  │  ├── context: AppContext          (共享状态)                       │    │
│  │  ├── ui_state: UIState            (UI状态)                         │    │
│  │  └── runtime: Runtime             (Tokio单线程)                   │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                              │                                               │
│              ┌───────────────┴───────────────┐                              │
│              ▼                               ▼                              │
│    ┌─────────────────┐           ┌─────────────────┐                       │
│    │    ChatView     │           │   ConfigView   │                       │
│    └─────────────────┘           └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼ (mpsc channel)
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Agent Core (core/)                                │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │  Agent                                                              │    │
│  │  ├── config: AgentConfig                                           │    │
│  │  ├── providers: ProviderRegistry                                   │    │
│  │  ├── tool_registry: Option<Arc<ToolRegistry>>                     │    │
│  │  ├── reasoning_history: Arc<Mutex<Vec<String>>>                  │    │
│  │  └── scheduler/message_bus: Option<Arc<...>>                       │    │
│  │                                                                      │    │
│  │  run_agent_loop() → chat_with_tools_streaming() →                  │    │
│  │       Provider流式调用 → 解析tool_calls → 执行循环                 │    │
│  └────────────────────────────────────────────────────────────────────┘    │
│                              │                                               │
│              ┌───────────────┴───────────────┐                              │
│              ▼                               ▼                              │
│    ┌─────────────────┐           ┌─────────────────┐                     │
│    │  ToolRegistry   │           │ ProviderRegistry│                     │
│    │  内置 + MCP工具  │           │ LLM/ToolProvider│                     │
│    └─────────────────┘           └─────────────────┘                     │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Providers (providers/)                               │
│  ┌────────┐  ┌──────────┐  ┌────────┐  ┌───────┐  ┌────────┐               │
│  │ OpenAI │  │ Anthropic│  │DeepSeek│  │ Ollama│  │MiniMax │               │
│  │ GPT-4  │  │ Claude   │  │  V4    │  │Local  │  │        │               │
│  └────────┘  └──────────┘  └────────┘  └───────┘  └────────┘               │
│                                                                              │
│  LLMProvider Trait:                                                          │
│    - chat(messages) → ModelResponse                                          │
│    - stream_chat(messages, on_chunk)                                         │
│                                                                              │
│  ToolCallingProvider Trait:                                                  │
│    - chat_with_tools(messages, tools) → ModelResponse                        │
│    - stream_chat_with_tools(messages, tools, on_chunk)                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Tool System (统一入口)                                  │
│  ┌────────────────────────────────────────────────────────────────────┐    │
│  │  ToolRegistry (HashMap<String, Box<dyn Tool>>)                     │    │
│  │                                                                      │    │
│  │  ┌──────────────────────┐     ┌──────────────────────────────────┐ │    │
│  │  │   Built-in Tools     │     │    MCP Tools (via Adapter)        │ │    │
│  │  │                      │     │                                   │ │    │
│  │  │  • BashTool         │     │  McpToolAdapter ← MCPClient       │ │    │
│  │  │  • ReadTool         │     │       └─► tools/list              │ │    │
│  │  │  • WriteTool        │     │       └─► tools/call              │ │    │
│  │  │  • EditTool         │     │                                   │ │    │
│  │  │  • GrepTool         │     │  Vec<McpToolAdapter>              │ │    │
│  │  └──────────────────────┘     └──────────────────────────────────┘ │    │
│  └────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     Platform (platform/)                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐         │
│  │ IntentGate  │  │   Hooks     │  │   Skill     │  │  Boulder  │         │
│  │  (同步)      │  │(async RwLock)│ │(async RwLock)│ │(Mutex<>)  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘  └───────────┘         │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Orchestration (orchestration/)                             │
│  ┌─────────────────────┐        ┌─────────────────────┐                   │
│  │   TaskScheduler     │        │    MessageBus       │                   │
│  │   Arc<RwLock<...>>  │        │   Arc<RwLock<...>>   │                   │
│  └─────────────────────┘        └─────────────────────┘                   │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Log Module (src/log/) ✅ 新增                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                        │
│  │   Log       │  │  FileWriter │  │  LogLevel   │                        │
│  │ (全局宏)    │  │  (单例)     │  │ TRACE-ERROR │                        │
│  └─────────────┘  └─────────────┘  └─────────────┘                        │
│                                                                              │
│  日志输出到 ~/.fastcode/ragent.log，替代前端 Debug Panel                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 二、问题状态汇总

| 模块 | 问题数 | 严重度 | 状态 | 说明 |
|------|--------|--------|------|------|
| TUI Frontend | 2 | 中 | ⚠️ 改进中 | MVC重构完成，Debug Panel将移除 |
| Provider接口 | 1 | 高 | ⚠️ 部分解决 | DeepSeek解析已实现，reasoning无累积 |
| Agent Loop | 2 | 中 | ⚠️ 待改进 | 工具串行执行、无并发 |
| Session管理 | 2 | 中 | ❌ 未解决 | 未被使用，无context window管理 |
| Tool System | 3 | 中 | ⚠️ 改进中 | 硬编码注册→MCP统一，串行→并发 |
| Orchestration | 1 | 低 | ❌ 未解决 | TaskScheduler/MessageBus未被使用 |
| MCP Client | 2 | 中 | ⚠️ 改进中 | 已有协议和client，正设计适配层 |
| Platform | 0 | - | ✅ 已实现 | IntentGate/Hooks/Skill/Boulder完整 |
| Log Module | 0 | - | 📋 规划中 | 替代Debug Panel，统一日志系统 |

---

## 三、核心类型定义

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

#[derive(Clone)]
pub struct Agent {
    config: AgentConfig,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    providers: ProviderRegistry,
    tool_registry: Option<Arc<ToolRegistry>>,
    reasoning_history: Arc<Mutex<Vec<String>>>,  // DeepSeek reasoning 累积
}
```

**Agent 公开方法**:

| 方法 | 签名 | 说明 |
|------|------|------|
| `new` | `fn new(config: AgentConfig) -> Self` | 创建 Agent |
| `with_provider` | `fn with_provider<P: LLMProvider + 'static>(self, name: &str, provider: P) -> Self` | 添加 LLM Provider |
| `with_tool_provider` | `fn with_tool_provider<P: ToolCallingProvider + 'static>(self, name: &str, provider: P) -> Self` | 添加 Tool-Calling Provider |
| `with_tool_registry` | `fn with_tool_registry(self, registry: Arc<ToolRegistry>) -> Self` | 设置工具注册表 |
| `with_scheduler` | `fn with_scheduler(self, scheduler: Arc<TaskScheduler>) -> Self` | 设置任务调度器 |
| `with_message_bus` | `fn with_message_bus(self, message_bus: Arc<MessageBus>) -> Self` | 设置消息总线 |
| `list_tools` | `fn list_tools(&self) -> Vec<String>` | 列出所有工具 |
| `get_tool_definitions` | `fn get_tool_definitions(&self) -> Vec<ToolDefinition>` | 获取工具定义列表 |
| `call_tool` | `async fn call_tool(&self, name: &str, arguments: JsonValue) -> Result<ToolResult>` | 调用单个工具 |
| `execute_tool_calls` | `async fn execute_tool_calls(&self, calls: Vec<ToolCall>) -> Result<Vec<ToolResult>>` | 串行执行工具 |
| `execute_independent_tool_calls` | `async fn execute_independent_tool_calls(&self, calls: Vec<ToolCall>) -> Vec<ToolResult>` | 并发执行无依赖工具 |
| `chat_with_tools_streaming` | `async fn chat_with_tools_streaming<F>(&self, messages, debug_log, debug_ui, on_chunk: F) -> Result<ModelResponse>` | 流式工具调用 |
| `run_agent_loop` | `async fn run_agent_loop(...) -> Result<String>` | **核心循环** |

### 3.2 Tool (core/tool.rs)

**类型定义**:

```rust
pub struct ToolCall {
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
    pub depends_on: Option<Vec<String>>,  // 依赖分析
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

## 四、MCP 工具统一架构 (integration/mcp/)

### 4.1 架构概览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        MCP 工具统一入口                                      │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │  ToolRegistry                                                       │   │
│  │                                                                      │   │
│  │  内置工具 (5个)              MCP 工具 (动态加载)                      │   │
│  │  ┌──────────────────┐       ┌──────────────────────────────────┐   │   │
│  │  │ • BashTool       │       │  McpToolAdapter                  │   │   │
│  │  │ • ReadTool       │       │  ├── name: String                │   │   │
│  │  │ • WriteTool      │       │  ├── description: String        │   │   │
│  │  │ • EditTool       │       │  ├── input_schema: JsonValue     │   │   │
│  │  │ • GrepTool       │       │  └── mcp_client: Arc<MCPClient>  │   │   │
│  │  └──────────────────┘       └──────────────────────────────────┘   │   │
│  │                                                                      │   │
│  │  register(BashTool::new())        load_mcp_tools() → register()    │   │
│  └────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         MCP 适配层                                          │
│                                                                              │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────────────┐    │
│  │ McpToolAdapter │  │McpConnectionPool│  │    McpToolLoader         │    │
│  │               │  │                │  │                            │    │
│  │ 实现 Tool trait │  │ 管理多MCP客户端│  │ 从MCP服务器加载工具       │    │
│  │ 调用 mcp_client │  │ 连接池架构     │  │ 注册到 ToolRegistry       │    │
│  │ 转换 ContentBlock│  │ 单server起步  │  │                            │    │
│  └────────────────┘  └────────────────┘  └────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      MCP Client (已有)                                       │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │  MCPClient                                                          │   │
│  │  ├── server_url: String                                            │   │
│  │  ├── connected: Arc<RwLock<bool>>                                 │   │
│  │  ├── initialized: Arc<RwLock<bool>>                               │   │
│  │  └── request_id: Arc<RwLock<u32>>                                 │   │
│  │                                                                      │   │
│  │  connect() → initialize() → list_tools() → call_tool()            │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  协议: HTTP + JSON-RPC 2.0                                                  │
│  方法: initialize, tools/list, tools/call                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 McpToolAdapter

```rust
// src/integration/mcp/adapter.rs

pub struct McpToolAdapter {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    mcp_client: Arc<MCPClient>,
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }
    fn input_schema(&self) -> serde_json::Value { self.input_schema.clone() }

    async fn execute(&self, call: ToolCall) -> ToolResult {
        let arguments = serde_json::to_value(&call.arguments)
            .unwrap_or(serde_json::Value::Null);

        match self.mcp_client.call_tool(&self.name, arguments).await {
            Ok(content_blocks) => {
                // Vec<ContentBlock> → ToolResult
                let output = content_blocks
                    .iter()
                    .filter_map(|cb| cb.text.clone())
                    .collect::<Vec<_>>()
                    .join("\n");

                ToolResult {
                    success: true,
                    output: Some(output),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }
}
```

### 4.3 McpConnectionPool

```rust
// src/integration/mcp/pool.rs

pub struct McpConnectionPool {
    clients: HashMap<String, Arc<MCPClient>>,
    default_server: Option<String>,
}

impl McpConnectionPool {
    pub fn new() -> Self;
    pub fn add_server(&mut self, name: &str, url: &str) -> Result<()>;
    pub async fn connect(&self, name: &str) -> Result<()>;
    pub async fn initialize(&self, name: &str, client_name: &str, version: &str) -> Result<()>;
    pub fn client(&self, name: &str) -> Option<&Arc<MCPClient>>;
    pub fn default_client(&self) -> Option<&Arc<MCPClient>>;
}
```

### 4.4 McpToolLoader

```rust
// src/integration/mcp/loader.rs

pub struct McpToolLoader {
    pool: Arc<Mutex<McpConnectionPool>>,
    registry: Arc<ToolRegistry>,
}

impl McpToolLoader {
    pub fn new(pool: Arc<Mutex<McpConnectionPool>>, registry: Arc<ToolRegistry>) -> Self;
    pub async fn load_tools(&self, server_name: &str) -> Result<usize>;
}
```

### 4.5 MCP 调用流程

```
Agent::run_agent_loop()
    │
    ▼
ToolRegistry.execute(ToolCall { name: "read_file", arguments: {...} })
    │
    ├── 内置工具: 直接执行 → ToolResult
    │
    └── MCP工具:
            │
            ▼
        McpToolAdapter::execute()
            │
            ├── mcp_client.call_tool(name, arguments)
            │       │
            │       └── HTTP POST /tools/call
            │           { name: "read_file", arguments: {...} }
            │
            ├── Vec<ContentBlock> ← HTTP响应
            │
            └── convert → ToolResult { success, output, error }
```

---

## 五、Provider 接口

### 5.1 LLMProvider Trait

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn list_models(&self) -> Result<Vec<String>>;
    async fn stream_chat(&self, messages: Vec<Message>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()>;
}
```

### 5.2 ToolCallingProvider Trait

```rust
#[async_trait]
pub trait ToolCallingProvider: Send + Sync {
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()>;
}
```

### 5.3 Provider 实现对比

| Provider | 文件 | Tool Call格式 | Streaming | 状态 |
|----------|------|--------------|-----------|------|
| OpenAIProvider | `providers/openai.rs` | `tool_calls` JSON数组 | ✅ 标准SSE | 正常 |
| AnthropicProvider | `providers/anthropic.rs` | `tool_calls` JSON数组 | ✅ 标准SSE | 正常 |
| DeepSeekProvider | `providers/deepseek.rs` | `tool_calls` JSON数组 | ✅ 标准SSE | 正常 |
| OllamaProvider | `providers/ollama.rs` | 无tool calling | ✅ | 不支持工具 |
| MiniMaxProvider | `providers/minimax.rs` | 待完善 | 待完善 | 待实现 |
| CustomProvider | `providers/custom.rs` | 自定义端点 | 待完善 | 待实现 |

### 5.4 DeepSeek 思考模式

**输入要求**:
```rust
"thinking": { "type": "enabled" },
"reasoning_effort": "high"  // 或 "low"
```

**输出格式**:
- `reasoning_content` → 推理内容（流式返回）
- `content` → 用户可见内容
- `tool_calls` → JSON数组格式（与其他 Provider 一致）

> **注意**: DeepSeek 原实现中在流式输出时手动添加 `<thinking>`, `<content>`, `<tool_call>` 等文本标签用于前端区分显示，这些标签应删除，改为直接使用标准 JSON 格式，通过 `reasoning_content` / `content` / `tool_calls` 字段区分。

---

## 六、TUI 模块

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
    Quit,
}
```

> **Note**: `ToggleDebug`, `ScrollDebugUp`, `ScrollDebugDown` 已移除（由 Log 模块替代）

### 6.3 ViewState 枚举

```rust
pub enum ViewState {
    Chat(ChatViewState),
    Config(ConfigViewState),
}

pub enum ViewStateKind {
    Chat,
    Config,
}
```

> **Note**: `Debug(DebugViewState)` 已移除

### 6.4 视图状态类型

```rust
// 消息块 - 支持深度思考、工具调用、内容分离显示
pub struct MessageBlock {
    pub role: String,                              // "user" | "assistant"
    pub reasoning: Option<String>,                 // 深度思考内容
    pub tool_calls: Vec<ToolCallBlock>,            // 工具调用列表
    pub tool_results: Vec<ToolResultBlock>,        // 工具结果列表
    pub content: String,                           // 最终回答
    pub status: MessageStatus,                     // 状态
}

pub struct ToolCallBlock {
    pub name: String,                              // "bash", "read" 等
    pub arguments: String,                         // JSON 格式参数
}

pub struct ToolResultBlock {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}

pub enum MessageStatus {
    Streaming,                                     // 流式输出中
    Completed,                                     // 已完成
    Error(String),                                 // 错误信息
}

// 简化的消息格式（保留用于向后兼容）
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct ChatViewState {
    pub messages: Vec<MessageBlock>,                // 新格式：支持分栏显示
    pub input: String,
    pub cursor_pos: usize,
    pub is_streaming: bool,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub streaming_text: Option<String>,
    pub current_provider: String,
    pub current_model: String,
    pub reasoning_collapsed: bool,                 // 思考区域是否折叠
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
```

### 6.5 AppContext 和 UIState

```rust
#[derive(Clone)]
pub struct AppContext {
    pub agent: Arc<Agent>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub tool_registry: Arc<ToolRegistry>,
}

pub struct UIState {
    pub streaming_text: Arc<Mutex<Option<String>>>,
    pub debug_messages: Arc<Mutex<Vec<String>>>,  // 由 Log 模块替代
    pub response_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<String, anyhow::Error>>>>>,
    pub finalized_message: Arc<Mutex<Option<(String, String)>>>,
}
```

> **Note**: `debug_log_path` 已移除，debug 日志写入 `~/.fastcode/ragent.log`

---

## 七、Platform 模块

### 7.1 IntentGate

```rust
pub enum IntentCategory {
    Research,
    Implementation,
    Investigation,
    Evaluation,
    Fix,
    OpenEnded,
    Trivial,
}

impl IntentGate {
    pub fn new() -> Self;
    pub fn classify(&self, input: &str) -> IntentCategory;
    pub fn classify_with_confidence(&self, input: &str) -> (IntentCategory, f32);
    pub fn route_hint(&self, category: &IntentCategory) -> &str;
}
```

### 7.2 Hooks System

```rust
pub enum HookEvent {
    OnStartup, OnShutdown,
    OnError(String), OnWarning(String), OnInfo(String), OnDebug(String),
    OnAgentCreated(String), OnAgentDestroyed(String),
    OnSessionStart, OnSessionEnd,
    OnMessageReceived(String), OnMessageSent(String),
    OnToolCall(String), OnToolResult(String, bool),
    OnProviderCall(String), OnProviderResponse(bool),
}

pub struct HookRegistry {
    hooks: RwLock<HashMap<String, Vec<(HookFn, i32)>>>,
}
```

### 7.3 Skill System

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub commands: Vec<SkillCommand>,
    pub scope: SkillScope,  // Global / Project / User
}

pub enum SkillScope { Global, Project, User }

pub struct SkillRegistry {
    skills: RwLock<HashMap<String, RegisteredSkill>>,
}
```

### 7.4 Boulder

```rust
pub enum BoulderStatus {
    Pending, InProgress, Completed, Cancelled,
}

pub enum BoulderPriority { Low, Medium, High }

pub struct Boulder {
    pub id: String,
    pub content: String,
    pub status: BoulderStatus,
    pub priority: BoulderPriority,
    pub tags: Vec<String>,
}

pub struct BoulderStore { db: BoulderDatabase } // SQLite
```

---

## 八、Orchestration 模块

### 8.1 TaskScheduler

```rust
pub enum TaskPriority { Low, Normal, High, Critical }
pub enum TaskStatus { Pending, Running, Completed, Failed }

pub struct TaskScheduler {
    tasks: Arc<RwLock<VecDeque<Task>>>,
}

impl TaskScheduler {
    pub async fn add_task(&self, task: Task);
    pub async fn get_next_task(&self) -> Option<Task>;
    pub async fn complete_task(&self, task_id: &str);
    pub async fn list_pending(&self) -> Vec<Task>;
}
```

### 8.2 MessageBus

```rust
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub trait MessageHandler: Send + Sync {
    fn handle(&self, message: AgentMessage) -> Result<()>;
}

pub struct MessageBus {
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
}

impl MessageBus {
    pub async fn subscribe(&self, agent_id: &str, handler: Arc<dyn MessageHandler>);
    pub async fn send(&self, message: AgentMessage) -> Result<()>;
    pub async fn broadcast(&self, from: &str, subject: &str, body: &str) -> Result<()>;
}
```

---

## 九、Log 模块

### 9.1 类型定义

```rust
pub enum LogLevel {
    TRACE, DEBUG, INFO, WARN, ERROR,
}

pub struct FileWriter {
    file: Arc<Mutex<File>>,
    level: LogLevel,
}

pub struct Log {
    writer: Arc<FileWriter>,
    module: String,
}

impl Log {
    pub fn new(writer: Arc<FileWriter>, module: impl Into<String>) -> Self;
    pub fn trace(&self, msg: &str);
    pub fn debug(&self, msg: &str);
    pub fn info(&self, msg: &str);
    pub fn warn(&self, msg: &str);
    pub fn error(&self, msg: &str);
}
```

### 9.2 全局宏

```rust
macro_rules! trace { ($($arg:tt)*) => { log!(LogLevel::TRACE, $($arg)*); } }
macro_rules! debug { ($($arg:tt)*) => { log!(LogLevel::DEBUG, $($arg)*); } }
macro_rules! info  { ($($arg:tt)*) => { log!(LogLevel::INFO, $($arg)*); } }
macro_rules! warn  { ($($arg:tt)*) => { log!(LogLevel::WARN, $($arg)*); } }
macro_rules! error { ($($arg:tt)*) => { log!(LogLevel::ERROR, $($arg)*); } }
```

### 9.3 日志格式

```
[HH:MM:SS.mmm] [LEVEL] [MODULE] message
```

示例：
```
[14:28:49.111] [INFO] [agent] Agent loop started
[14:28:49.234] [DEBUG] [mcp] Calling tool 'read_file' with args: {...}
[14:28:50.567] [ERROR] [mcp] MCP call failed: connection refused
```

---

## 十、文件路径索引

### 新增模块

| 文件 | 说明 |
|------|------|
| `src/log/mod.rs` | Log, LogLevel, FileWriter, 全局宏 |
| `src/log/level.rs` | LogLevel 枚举 |
| `src/log/writer.rs` | FileWriter 单例 |
| `src/integration/mcp/adapter.rs` | McpToolAdapter |
| `src/integration/mcp/pool.rs` | McpConnectionPool |
| `src/integration/mcp/loader.rs` | McpToolLoader |

### 核心文件

| 文件 | 说明 |
|------|------|
| `src/core/agent.rs` | Agent 类型 |
| `src/core/tool.rs` | Tool trait 和 ToolRegistry |
| `src/core/session.rs` | Session 和 Message |

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

| 文件 | 说明 |
|------|------|
| `src/tui/app_controller.rs` | AppController |
| `src/tui/views/chat_view.rs` | ChatView |
| `src/tui/views/config_view.rs` | ConfigView |
| `src/tui/views/debug_view.rs` | **已移除** (由 Log 模块替代) |

### Integration

| 文件 | 说明 |
|------|------|
| `src/integration/mcp/protocol.rs` | MCP JSON-RPC 协议 |
| `src/integration/mcp/client.rs` | MCP HTTP Client |

---

## 十一、文件路径索引