# Rust Agent Platform - 架构分析与改进计划

> 生成日期: 2026-05-19
> 更新日期: 2026-05-21
> 分支: feature/tui-refactor

---

## 一、架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                           main.rs                              │
│                    (Binary Entry Point)                         │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                          TUI Layer                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  AppController (413行)                                │    │
│  │  ├── current_view: Box<dyn View>  (Chat/Config/Debug)│    │
│  │  ├── context: AppContext       (共享状态)             │    │
│  │  ├── ui_state: UIState        (UI状态)               │    │
│  │  └── runtime: Runtime         (Tokio单线程)          │    │
│  └──────────────────────────────────────────────────────┘    │
│                              │                                 │
│         ┌────────────────────┼────────────────────┐           │
│         ▼                    ▼                    ▼           │
│  ┌────────────┐      ┌────────────┐      ┌────────────┐      │
│  │ ChatView   │      │ ConfigView │      │ DebugView  │      │
│  │ (399行)    │      │ (400+行)  │      │ (200+行)  │      │
│  └────────────┘      └────────────┘      └────────────┘      │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼ (mpsc channel)
┌─────────────────────────────────────────────────────────────┐
│                        Agent Core (core/)                     │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  Agent (553行)                                       │    │
│  │  ├── config: AgentConfig                             │    │
│  │  ├── providers: ProviderRegistry                    │    │
│  │  ├── tool_registry: Option<Arc<ToolRegistry>>       │    │
│  │  └── scheduler/message_bus: Option<Arc<...>>        │    │
│  │                                                      │    │
│  │  run_agent_loop() → chat_with_tools_streaming() →   │    │
│  │       Provider流式调用 → 解析tool_calls → 执行循环   │    │
│  └──────────────────────────────────────────────────────┘    │
│                              │                                │
│         ┌────────────────────┴────────────────────┐          │
│         ▼                                         ▼          │
│  ┌─────────────────────┐              ┌─────────────────────┐│
│  │  ToolRegistry       │              │  ProviderRegistry   ││
│  │  HashMap<Name,Tool>│              │  LLM/ToolProvider   ││
│  └─────────────────────┘              └─────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                      Providers (providers/)                    │
│  ┌────────┐  ┌──────────┐  ┌────────┐  ┌───────┐  ┌────────┐│
│  │ OpenAI │  │ Anthropic │  │DeepSeek│  │ Ollama│  │MiniMax ││
│  │ GPT-4  │  │ Claude    │  │ V4     │  │Local  │  │        ││
│  └────────┘  └──────────┘  └────────┘  └───────┘  └────────┘│
│                                                                    │
│  LLMProvider Trait:                                                │
│    - chat(messages) → ModelResponse                              │
│    - stream_chat(messages, on_chunk)                              │
│                                                                    │
│  ToolCallingProvider Trait:                                        │
│    - chat_with_tools(messages, tools) → ModelResponse             │
│    - stream_chat_with_tools(messages, tools, on_chunk)            │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                        Platform (platform/)                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐│
│  │ IntentGate  │  │   Hooks     │  │   Skill     │  │  Boulder  ││
│  │ (同步)       │  │(async RwLock)│  │(async RwLock)│  │(Mutex<>)  ││
│  └─────────────┘  └─────────────┘  └─────────────┘  └───────────┘│
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    Orchestration (orchestration/)               │
│  ┌─────────────────────┐        ┌─────────────────────┐   │
│  │   TaskScheduler      │        │    MessageBus        │   │
│  │   Arc<RwLock<...>>  │        │   Arc<RwLock<...>>   │   │
│  └─────────────────────┘        └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、问题汇总表

| 模块 | 问题数 | 严重度 | 状态 | 说明 |
|------|--------|--------|------|------|
| TUI Frontend | 2 | 中 | ✅ 已解决 | 重构完成，但debug panel有bug |
| Provider接口 | 1 | 高 | ⚠️ 部分解决 | DeepSeek解析已实现，但reasoning无累积 |
| Agent Loop | 2 | 中 | ⚠️ 待改进 | 工具串行执行、无并发 |
| Session管理 | 2 | 中 | ❌ 未解决 | 未被使用，无context window管理 |
| Tool System | 1 | 中 | ❌ 未解决 | 硬编码注册，无动态加载 |
| Orchestration | 1 | 低 | ❌ 未解决 | TaskScheduler/MessageBus未被使用 |
| MCP Client | 2 | 中 | ❌ 未解决 | client存在但未集成到Agent |
| Platform | 0 | - | ✅ 已实现 | IntentGate/Hooks/Skill/Boulder完整实现 |

---

## 三、TUI Frontend (前端)

### 3.1 当前状态 ✅ 重构完成

**文件**: `src/tui/app_controller.rs` (413行)

**已完成改进**:
1. ✅ 拆分为 MVC 架构: AppController + Views + State + Components
2. ✅ 移除嵌套Runtime，改为 `Builder::new_current_thread()`
3. ✅ 移除 AppMode 混乱，改为 View trait + ViewState enum
4. ✅ 状态分离: AppContext (共享) + UIState (UI状态) + ViewState (各View私有)

**目标架构**:
```
AppController (状态机)
    │
    ├── ChatView (聊天界面状态)
    ├── ConfigView (配置界面状态)
    └── DebugView (调试面板状态)
```

### 3.2 仍存在的问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| View指针转换 | `get_chat_view_mut()` 使用原始指针转换，类型安全无法保证 | 中 |
| Debug Panel Bug | 消息发送后不显示、debug窗口不显示（见bug报告） | 高 |

### 3.3 文件结构

```
src/tui/
├── mod.rs (10行)           # 模块导出
├── app_controller.rs (413行) # 主控制器
├── config.rs               # ConfigManager
├── state/
│   ├── action.rs (38行)    # Action 枚举
│   ├── app_context.rs      # AppContext, UIState
│   └── view_state.rs       # ChatViewState, ConfigViewState, DebugViewState
├── views/
│   ├── view.rs (11行)      # View trait
│   ├── chat_view.rs (399行)
│   ├── config_view.rs (400+行)
│   └── debug_view.rs (200+行)
└── components/
    ├── message_list.rs
    ├── input_area.rs
    ├── scroll_bar.rs
    └── status_bar.rs
```

---

## 四、Provider 接口

### 4.1 当前状态

**Trait定义** (`providers/mod.rs`):

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

### 4.2 Provider实现对比

| Provider | Tool Call格式 | Streaming | 状态 |
|----------|---------------|-----------|------|
| OpenAI | `tool_calls` JSON数组 | ✅ 标准SSE | 正常 |
| Anthropic | `tool_calls` JSON数组 | ✅ 标准SSE | 正常 |
| DeepSeek | **文本标签 `<tool_call>`** | ✅ 自定义格式 | ⚠️ 已实现但有问题 |
| Ollama | 无tool calling | ✅ | 不支持工具 |
| MiniMax | 待完善 | 待完善 | 待实现 |
| Custom | 自定义端点 | 待完善 | 待实现 |

### 4.3 DeepSeek特殊处理 ✅ 已实现

**输入要求** (deepseek.rs):
```rust
"thinking": { "type": "enabled" },
"reasoning_effort": "high"  // 或 "low"
```

**输出解析** (`stream_chat_with_tools`):
- `<thinking>...</thinking>` → reasoning_content
- `<content>...</content>` → 用户可见内容
- `<tool_call>...</tool_call>` → 工具调用

### 4.4 仍存在的问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| reasoning_content无累积反馈 | DeepSeek需要将reasoning_content与tool结果一起发送，但当前没有累积 | 高 |
| 工具串行执行 | `execute_tool_calls()` 使用 for loop，无并发 | 中 |

---

## 五、Agent Loop

### 5.1 当前状态

**核心方法** (`agent.rs` 369-492):

```rust
pub async fn run_agent_loop(
    &self,
    initial_message: &str,
    max_iterations: usize,
    debug_log: Option<String>,
    debug_ui: Option<std::sync::mpsc::Sender<String>>,
    stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>
) -> Result<String>
```

### 5.2 流程图

```
[User Input]
    │
    ▼
messages = [user message]
    │
    ▼
┌─────────────────────────────────────┐
│  for i in 0..max_iterations:        │
│  ┌─────────────────────────────────┐ │
│  │ 1. chat_with_tools_streaming()  │ │
│  │    ↓                            │ │
│  │ 2. parse_tool_calls_from_json()  │ │
│  │    ├─ 从 response.tool_calls     │ │
│  │    └─ fallback: parse_tool_calls │ │
│  │    ↓                            │ │
│  │ 3. 如果无tool_calls → 返回content│ │
│  │    ↓                            │ │
│  │ 4. execute_tool_calls()          │ │
│  │    (串行for loop)               │ │
│  │    ↓                            │ │
│  │ 5. 把结果加入messages           │ │
│  │    ↓                            │ │
│  │ 6. 继续循环                     │ │
│  └─────────────────────────────────┘ │
└─────────────────────────────────────┘
    │
    ▼
[Full Response + Tool Summary]
```

### 5.3 仍存在的问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| reasoning_content丢失 | DeepSeek的reasoning_content需要累积并反馈，但没有机制 | 高 |
| 工具串行执行 | `execute_tool_calls()` 一个个顺序执行，无并发 | 中 |
| 循环终止保护 | max_iterations=5可能被某些情况绕过 | 低 |

---

## 六、Session 管理

### 6.1 当前状态

**实现** (`session.rs` 49行):
```rust
pub struct Session {
    messages: VecDeque<Message>,
    context_window: usize,
}
```

### 6.2 问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| Session未被使用 | Agent直接持有`Vec<Message>`，Session定义但未使用 | 中 |
| 无context window管理 | 消息无限增长，没有截断/摘要机制 | 中 |

---

## 七、Tool System

### 7.1 当前状态

**架构**:
```
Tool trait (async_trait)
    │
    ├── name() / description() / input_schema()
    └── execute(call: ToolCall) → ToolResult

ToolRegistry: HashMap<Name, Box<dyn Tool>>
    ├── register(T)
    ├── execute(call) → ToolResult
    └── get_definitions() → Vec<ToolDefinition>

Built-in Tools (5个):
    ├── BashTool (std::process::Command)
    ├── ReadTool (文件读取)
    ├── WriteTool (文件写入)
    ├── EditTool (行编辑)
    └── GrepTool (搜索)
```

### 7.2 问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| 工具注册硬编码 | 在`AppController::new()`里硬编码注册，无动态加载 | 中 |
| 串行执行 | `execute_tool_calls()` 串行for loop | 中 |

---

## 八、Platform 模块 ✅ 已实现

### 8.1 模块结构

| 模块 | 核心功能 | 多线程 | 文件 |
|------|---------|--------|------|
| IntentGate | 意图分类路由 | ❌ 无 | `intent_gate.rs`, `category.rs` |
| Hooks | 生命周期事件钩子 | ✅ tokio::RwLock | `hooks/types.rs`, `hooks/registry.rs` |
| Skill | SKILL.md加载注册 | ✅ tokio::RwLock | `skill/loader.rs`, `skill/registry.rs` |
| Boulder | TODO持久化 | ✅ std::Mutex | `boulder.rs`, `boulder_db.rs` |

### 8.2 IntentGate

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
    pub fn classify(&self, input: &str) -> IntentCategory;
    pub fn classify_with_confidence(&self, input: &str) → (IntentCategory, f32);
    pub fn route_hint(&self, category: &IntentCategory) → &str;
}
```

### 8.3 Hooks System

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

### 8.4 Skill System

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub commands: Vec<SkillCommand>,
    pub scope: SkillScope,  // Global / Project / User
}

pub enum SkillScope { Global, Project, User }
```

### 8.5 Boulder (TODO Persistence)

```rust
pub struct Boulder {
    pub id: String,
    pub content: String,
    pub status: BoulderStatus,   // Pending/InProgress/Completed/Cancelled
    pub priority: BoulderPriority, // Low/Medium/High
    pub tags: Vec<String>,
}

pub struct BoulderStore { db: BoulderDatabase } // SQLite
```

---

## 九、Orchestration 模块

### 9.1 TaskScheduler

```rust
pub struct TaskScheduler {
    tasks: Arc<RwLock<VecDeque<Task>>>,
}

pub enum TaskPriority { Low, Normal, High, Critical }
pub enum TaskStatus { Pending, Running, Completed, Failed }
```

### 9.2 MessageBus

```rust
pub struct MessageBus {
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
}

pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}
```

### 9.3 问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| 未被Agent使用 | TaskScheduler和MessageBus存在但未被实例化和使用 | 低 |

---

## 十、MCP Client

### 10.1 当前状态

**文件**:
- `src/integration/mcp/protocol.rs` (215行) - JSON-RPC 2.0
- `src/integration/mcp/client.rs` (197行) - HTTP POST

### 10.2 问题

| 问题 | 说明 | 严重度 |
|------|------|--------|
| 未集成到Agent | AppController里没有MCP client实例化代码 | 中 |
| 无tool bridging | MCP tools没有转换成内部ToolDefinition | 中 |

---

## 十一、根因分析

| 问题 | 根因 |
|------|------|
| 重复小bug | TUI状态机混乱 → ✅ 已解决 |
| DeepSeek tool call问题 | Provider接口不一致 → ⚠️ 部分解决 |
| reasoning_content丢失 | 没有累积-反馈机制 → ❌ 未解决 |
| MCP未集成 | client和agent之间缺少bridge → ❌ 未解决 |
| 工具串行执行 | 无并发设计 → ❌ 未解决 |
| Session未使用 | Agent直接持有Vec<Message> → ❌ 未解决 |

---

## 十二、讨论进度

| 模块 | 状态 | 结论 |
|------|------|------|
| TUI Frontend | ✅ 已解决 | 重构完成，debug panel有bug待修复 |
| Provider 接口 | ⚠️ 部分解决 | DeepSeek解析已实现，reasoning无累积 |
| Agent Loop | ⚠️ 待改进 | reasoning累积、工具并发执行 |
| Session 管理 | ❌ 未解决 | 需要设计context window管理方案 |
| Tool System | ❌ 未解决 | 需要动态加载机制 |
| Platform | ✅ 已实现 | 文档需要更新 |
| Orchestration | ❌ 未解决 | 需要集成到Agent |
| MCP Client | ❌ 未解决 | 需要MCPToolAdapter |

---

*文档版本: 1.1*
*最后更新: 2026-05-21*
