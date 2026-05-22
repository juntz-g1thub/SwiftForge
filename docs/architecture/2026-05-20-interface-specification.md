# Rust Agent Platform - 接口规范文档

> 文档版本: 1.2
> 生成日期: 2026-05-19
> 更新日期: 2026-05-22
> 分支: feature/tui-refactor
> 状态: **初稿 - 待完善**

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
├── log/                # 统一日志模块 (新)
├── tui/                # 终端用户界面 (ratatui 0.26)
├── platform/           # 平台功能 (boulder, hooks, skill, intent_gate)
├── orchestration/       # 多智能体编排 (TaskScheduler, MessageBus)
└── integration/        # 外部集成 (MCP)
```

---

## 三、Log 模块接口 ✅ 新增

### 3.1 设计目标

- **独立模块**：`src/log/` 作为独立的日志系统
- **仅文件输出**：日志仅输出到文件，不打印到控制台
- **多级别支持**：TRACE, DEBUG, INFO, WARN, ERROR
- **全局宏简化**：通过 `log::info!()` 等宏简化调用

### 3.2 类型定义

```rust
// log/level.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

// log/writer.rs
pub struct FileWriter {
    file: Arc<Mutex<File>>,
    level: LogLevel,
}

impl FileWriter {
    pub fn new(path: PathBuf, level: LogLevel) -> Result<Self>;
    pub fn write(&self, level: LogLevel, msg: &str);
}

// log/mod.rs
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
    pub fn log(&self, level: LogLevel, msg: &str);
}
```

### 3.3 全局宏

```rust
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        // 调用 Log::log()
    };
}

macro_rules! trace { ($($arg:tt)*) => { log!(LogLevel::TRACE, $($arg)*); } }
macro_rules! debug { ($($arg:tt)*) => { log!(LogLevel::DEBUG, $($arg)*); } }
macro_rules! info { ($($arg:tt)*) => { log!(LogLevel::INFO, $($arg)*); } }
macro_rules! warn { ($($arg:tt)*) => { log!(LogLevel::WARN, $($arg)*); } }
macro_rules! error { ($($arg:tt)*) => { log!(LogLevel::ERROR, $($arg)*); } }
```

### 3.4 使用方式

```rust
use crate::log::{info, error, LogLevel};

// 初始化
let writer = FileWriter::new(log_path, LogLevel::DEBUG)?;
let log = Log::new(Arc::new(writer), "agent");

// 使用宏
info!("Agent started");
error!("Tool {} not found", name);

// 直接使用 Log 实例
log.info("Hello");
log.error("Failed");
```

### 3.5 文件路径

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/log/mod.rs` | ~50 | Log, LogLevel, FileWriter, 全局宏 |
| `src/log/level.rs` | ~30 | LogLevel 枚举 |
| `src/log/writer.rs` | ~80 | FileWriter 单例 |

---

## 四、Core 模块接口

### 4.1 Agent (core/agent.rs)

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
    reasoning_history: Arc<Mutex<Vec<String>>>,  // 新增：Reasoning 历史累积
}
```

**reasoning_history 管理方法**:

| 方法 | 签名 | 说明 |
|------|------|------|
| `add_reasoning` | `fn add_reasoning(&self, content: String)` | 添加 reasoning_content |
| `get_reasoning_history` | `fn get_reasoning_history(&self) -> Vec<String>` | 获取所有 reasoning |
| `clear_reasoning` | `fn clear_reasoning(&self)` | 清空历史 |
| `format_reasoning_for_next_turn` | `fn format_reasoning_for_next_turn(&self) -> Option<String>` | 格式化用于下一轮 |

**reasoning 格式化格式**（发送给 DeepSeek）:

```json
{
  "role": "user",
  "content": "<tool_result>结果</tool_result>\n<reasoning>之前的思考过程</reasoning>"
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
| `parse_tool_calls_from_json` | `fn parse_tool_calls_from_json(&self, tool_calls: &[serde_json::Value]) -> Vec<ToolCall>` | 从JSON解析工具调用 |
| `execute_tool_calls` | `async fn execute_tool_calls(&self, calls: Vec<ToolCall>) -> Result<Vec<ToolResult>>` | 执行工具调用列表（并发） |
| `execute_independent_tool_calls` | `async fn execute_independent_tool_calls(&self, calls: Vec<ToolCall>) -> Vec<ToolResult>` | 并发执行无依赖工具 |
| `execute_sequential_tool_calls` | `async fn execute_sequential_tool_calls(&self, calls: Vec<ToolCall>) -> Vec<ToolResult>` | 顺序执行有依赖工具 |
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
    pub depends_on: Option<Vec<String>>,  // 新增：依赖的其他工具名
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

**依赖分析策略**:
1. **无依赖工具**（`depends_on.is_none()`）：立即并发执行
2. **有依赖工具**：按依赖顺序执行
3. 依赖定义通过分析工具名和参数自动检测

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

> ⚠️ **注意**: Session 当前未被 Agent 使用，Agent 直接持有 `Vec<Message>`。

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

## 五、Providers 模块接口

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

| Provider | 文件 | Tool Call格式 | Streaming |
|----------|------|--------------|-----------|
| OpenAIProvider | `providers/openai.rs` | `tool_calls` JSON数组 | ✅ 标准SSE |
| AnthropicProvider | `providers/anthropic.rs` | `tool_calls` JSON数组 | ✅ 标准SSE |
| DeepSeekProvider | `providers/deepseek.rs` | **文本标签** `<tool_call>` + reasoning_content | ✅ 自定义格式 |
| OllamaProvider | `providers/ollama.rs` | 无tool calling | ✅ |
| MiniMaxProvider | `providers/minimax.rs` | 待完善 | 待完善 |
| CustomProvider | `providers/custom.rs` | 自定义端点 | 待完善 |

---

## 六、Tools 模块接口

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

> ⚠️ **注意**: 工具当前在 `AppController::new()` 中硬编码注册，无动态加载机制。

---

## 七、TUI 模块接口

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

## 八、Platform 模块接口 ✅ 已实现

### 7.1 IntentGate (platform/intent_gate.rs)

```rust
pub struct IntentGate;

impl IntentGate {
    pub fn new() -> Self;
    pub fn classify(&self, input: &str) -> IntentCategory;
    pub fn classify_with_confidence(&self, input: &str) -> (IntentCategory, f32);
    pub fn route_hint(&self, category: &IntentCategory) -> &str;
}

pub enum IntentCategory {
    Research,
    Implementation,
    Investigation,
    Evaluation,
    Fix,
    OpenEnded,
    Trivial,
}
```

**路由建议**:
| Category | Route Hint |
|----------|------------|
| Research | "explore/librarian → synthesize → answer" |
| Implementation | "plan → delegate or execute" |
| Investigation | "explore → report findings" |
| Evaluation | "evaluate → propose → wait for confirmation" |
| Fix | "diagnose → fix minimally" |
| OpenEnded | "assess codebase first → propose approach" |
| Trivial | "direct tools only" |

### 7.2 Hooks System (platform/hooks/)

**HookEvent 枚举** (21个事件):
```rust
pub enum HookEvent {
    OnStartup,
    OnShutdown,
    OnError(String),
    OnWarning(String),
    OnInfo(String),
    OnDebug(String),
    OnAgentCreated(String),
    OnAgentDestroyed(String),
    OnSessionStart,
    OnSessionEnd,
    OnMessageReceived(String),
    OnMessageSent(String),
    OnToolCall(String),
    OnToolResult(String, bool),
    OnProviderCall(String),
    OnProviderResponse(bool),
}

pub struct HookContext {
    pub event: HookEvent,
    pub timestamp: std::time::Instant,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub type HookFn = Arc<dyn Fn(HookContext) -> Result<(), anyhow::Error> + Send + Sync>;

pub struct HookRegistry {
    hooks: RwLock<HashMap<String, Vec<(HookFn, i32)>>>,
}

impl HookRegistry {
    pub fn new() -> Self;
    pub async fn register(&self, event_name: &str, priority: i32, hook: HookFn);
    pub async fn dispatch(&self, event: HookEvent) -> Result<(), anyhow::Error>;
    pub async fn list_hooks(&self) -> Vec<String>;
}
```

### 7.3 Skill System (platform/skill/)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub commands: Vec<SkillCommand>,
    pub scope: SkillScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCommand {
    pub name: String,
    pub description: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillScope {
    Global,
    Project,
    User,
}

pub struct SkillLoader;

impl SkillLoader {
    pub fn new() -> Self;
    pub fn load_skill(&self, path: &Path) -> Result<Skill>;
    pub fn parse_skill(&self, content: &str) -> Result<Skill>;
    pub fn load_from_directory(&self, dir: &Path) -> Result<Vec<Skill>>;
}

#[derive(Clone)]
pub struct RegisteredSkill {
    pub skill: Arc<Skill>,
    pub enabled: bool,
}

pub struct SkillRegistry {
    skills: RwLock<HashMap<String, RegisteredSkill>>,
}

impl SkillRegistry {
    pub fn new() -> Self;
    pub async fn register(&self, skill: Skill);
    pub async fn get(&self, name: &str) -> Option<Arc<Skill>>;
    pub async fn enable(&self, name: &str) -> bool;
    pub async fn disable(&self, name: &str) -> bool;
    pub async fn list_skills(&self) -> Vec<String>;
    pub async fn list_enabled(&self) -> Vec<String>;
}
```

### 7.4 Boulder - TODO Persistence (platform/boulder/)

```rust
pub enum BoulderStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

pub enum BoulderPriority {
    Low,
    Medium,
    High,
}

pub struct Boulder {
    pub id: String,
    pub content: String,
    pub status: BoulderStatus,
    pub priority: BoulderPriority,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

pub struct BoulderStore {
    db: BoulderDatabase,  // SQLite
}

impl BoulderStore {
    pub fn new(data_dir: PathBuf) -> Result<Self>;
    pub fn save(&self, boulder: &Boulder) -> Result<()>;
    pub fn load(&self, id: &str) -> Result<Option<Boulder>>;
    pub fn list(&self) -> Result<Vec<Boulder>>;
    pub fn delete(&self, id: &str) -> Result<()>;
    pub fn create(&self, content: String, priority: BoulderPriority, tags: Vec<String>) -> Result<Boulder>;
    pub fn update_status(&self, id: &str, status: BoulderStatus) -> Result<()>;
}
```

---

## 九、Orchestration 模块接口 ✅ 已实现

### 8.1 TaskScheduler (orchestration/scheduler.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub assigned_to: Option<String>,
    pub status: TaskStatus,
}

pub struct TaskScheduler {
    tasks: Arc<RwLock<VecDeque<Task>>>,
}

impl TaskScheduler {
    pub fn new() -> Self;
    pub async fn add_task(&self, task: Task);
    pub async fn get_next_task(&self) -> Option<Task>;
    pub async fn complete_task(&self, task_id: &str);
    pub async fn fail_task(&self, task_id: &str);
    pub async fn list_pending(&self) -> Vec<Task>;
}
```

### 8.2 MessageBus (orchestration/message_bus.rs)

```rust
#[derive(Debug, Clone)]
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
    pub fn new() -> Self;
    pub async fn subscribe(&self, agent_id: &str, handler: Arc<dyn MessageHandler>);
    pub async fn unsubscribe(&self, agent_id: &str);
    pub async fn send(&self, message: AgentMessage) -> Result<()>;
    pub async fn broadcast(&self, from: &str, subject: &str, body: &str) -> Result<()>;
}
```

> ⚠️ **注意**: TaskScheduler 和 MessageBus 当前存在但未被 Agent 实际使用。

---

## 十、Integration 模块接口

### 9.1 MCP Client (integration/mcp/)

**文件**:
- `protocol.rs` (215行) - JSON-RPC 2.0 协议定义
- `client.rs` (197行) - HTTP POST 客户端

**问题**: MCP Client 存在但未被集成到 Agent。

---

## 十二、问题记录

### 12.1 接口不一致问题

| 问题 | 严重程度 | 说明 |
|------|----------|------|
| DeepSeek tool call 格式不同于其他 Provider | **高** | DeepSeek 输出 `<tool_call>` 文本标签，而非结构化 JSON |
| reasoning_content 未反馈给 DeepSeek | **高** | DeepSeek 需要将 reasoning_content 与 tool 结果一起发送，当前无累积机制 |
| Session 未被实际使用 | 中 | Agent 直接持有 `Vec<Message>`，Session 定义但未使用 |
| 工具串行执行 | 中 | `execute_tool_calls()` 使用 for loop，无并发 |
| 工具注册硬编码 | 中 | `AppController::new()` 硬编码注册，无动态加载 |
| Orchestration 未被使用 | 低 | TaskScheduler/MessageBus 存在但未集成 |
| MCP 未集成 | 中 | client 存在但未集成到 Agent |

### 12.2 建议的标准化

1. **ToolCall 格式统一**: 所有 Provider 输出必须映射到统一的 `ToolCall` 结构
2. **Reasoning 累积**: 单独的 `reasoning_history: Vec<String>` 字段
3. **Context Window 管理**: 实现 `ConversationManager` 统一管理消息历史
4. **并发工具执行**: 使用 `tokio::join!` 或 `futures::future::join_all`
5. **MCPToolAdapter**: 将 MCP Tool 转换成内部 `ToolDefinition` 格式

---

## 十一、文件路径索引

### Log (新增)

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/log/mod.rs` | ~50 | Log, LogLevel, FileWriter, 全局宏 |
| `src/log/level.rs` | ~30 | LogLevel 枚举 |
| `src/log/writer.rs` | ~80 | FileWriter 单例 |

### 核心文件

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/lib.rs` | 9 | 库入口 |
| `src/core/mod.rs` | 9 | Core 模块导出 |
| `src/core/agent.rs` | 553+ | **核心 Agent 类型** |
| `src/core/tool.rs` | 87+ | Tool trait 和注册表 |
| `src/core/session.rs` | 49 | Session 和 Message |
| `src/core/provider.rs` | ~100 | Provider 基础类型 |

### Providers

| 文件 | 说明 |
|------|------|
| `src/providers/mod.rs` | Trait 定义和 ProviderRegistry |
| `src/providers/openai.rs` | OpenAI 实现 (252行) |
| `src/providers/anthropic.rs` | Anthropic 实现 |
| `src/providers/deepseek.rs` | DeepSeek 实现 (426行) |
| `src/providers/ollama.rs` | Ollama 实现 |
| `src/providers/minimax.rs` | MiniMax 实现 |
| `src/providers/custom.rs` | Custom 实现 |

### TUI

| 文件 | 行数 | 说明 |
|------|------|------|
| `src/tui/mod.rs` | 10 | 模块导出 |
| `src/tui/app_controller.rs` | 413 | **AppController** (重构后) |
| `src/tui/config.rs` | ~100 | ConfigManager |
| `src/tui/state/action.rs` | 38 | Action 枚举 |
| `src/tui/state/view_state.rs` | ~120 | 视图状态类型 |
| `src/tui/state/app_context.rs` | ~100 | AppContext, UIState |
| `src/tui/views/view.rs` | 11 | View trait |
| `src/tui/views/chat_view.rs` | 399 | ChatView |
| `src/tui/views/config_view.rs` | ~400 | ConfigView |
| `src/tui/views/debug_view.rs` | ~200 | DebugView |

### Platform

| 文件 | 说明 |
|------|------|
| `src/platform/mod.rs` | 模块导出 |
| `src/platform/intent_gate.rs` | 意图分类 (84行) |
| `src/platform/category.rs` | IntentCategory 枚举 (42行) |
| `src/platform/hooks/mod.rs` | Hooks 模块导出 |
| `src/platform/hooks/types.rs` | HookEvent, HookContext |
| `src/platform/hooks/registry.rs` | HookRegistry |
| `src/platform/skill/mod.rs` | Skill 模块导出 |
| `src/platform/skill/loader.rs` | SkillLoader (114行) |
| `src/platform/skill/registry.rs` | SkillRegistry |
| `src/platform/boulder.rs` | BoulderStore (152行) |
| `src/platform/boulder_db.rs` | BoulderDatabase (149行) |

### Orchestration

| 文件 | 说明 |
|------|------|
| `src/orchestration/mod.rs` | 模块导出 |
| `src/orchestration/scheduler.rs` | TaskScheduler (82行) |
| `src/orchestration/message_bus.rs` | MessageBus (68行) |

### Integration

| 文件 | 说明 |
|------|------|
| `src/integration/mod.rs` | 模块导出 |
| `src/integration/mcp/protocol.rs` | MCP JSON-RPC 协议 (215行) |
| `src/integration/mcp/client.rs` | MCP HTTP Client (197行) |

---

## 十三、待完善

1. ~~**Platform 模块接口**~~ ✅ 已补充 (v1.1)
2. ~~**Orchestration 模块接口**~~ ✅ 已补充 (v1.1)
3. **Integration 模块接口** - MCP protocol 和 client 完整文档
4. **错误处理规范** - 统一的错误类型和传播方式
5. ~~**日志规范**~~ ✅ 已补充 Log 模块 (v1.2)

---

*文档版本: 1.2*
*最后更新: 2026-05-22*
