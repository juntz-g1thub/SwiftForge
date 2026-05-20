# Rust Agent Platform - 架构分析与改进计划

> 生成日期: 2026-05-19
> 项目分支: feature/rust-agent-phase2

---

## 一、架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                        TUI Frontend                         │
│                     (ratatui 0.26)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  Chat View   │  │ Input Area  │  │   Debug Panel       │ │
│  │  (Messages)  │  │ (Cmd Input) │  │   (↑↓ Scroll)       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└────────────────────────┬───────────────────────────────────┘
                         │ mpsc::channel
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    Agent Core (core/)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Agent     │  │  Session    │  │   Tool Registry      │ │
│  │   (553行)   │  │  (Messages) │  │   (5 built-in)      │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└────────────────────────┬───────────────────────────────────┘
                         │
              ┌──────────┴───────────┐
              ▼                      ▼
┌─────────────────────┐    ┌─────────────────────┐
│   LLMProvider       │    │ ToolCallingProvider  │
│   (trait)           │    │   (trait)           │
└─────────────────────┘    └─────────────────────┘
              │                      │
              ▼                      ▼
┌─────────────────────────────────────────────────────────────┐
│               Providers (providers/)                        │
│  ┌──────┐ ┌──────────┐ ┌────────┐ ┌───────┐ ┌────────┐     │
│  │OpenAI│ │Anthropic │ │DeepSeek│ │Ollama │ │MiniMax │     │
│  └──────┘ └──────────┘ └────────┘ └───────┘ └────────┘     │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、问题汇总表

| 模块 | 问题数 | 严重度 | 状态 |
|------|--------|--------|------|
| TUI Frontend | 5 | 高 | 待讨论 |
| Provider接口 | 4 | 高 | 待讨论 |
| Agent Loop | 3 | 高 | 待讨论 |
| Session管理 | 2 | 中 | 待讨论 |
| Tool System | 2 | 中 | 待讨论 |
| MCP Client | 3 | 中 | 待讨论 |

---

## 三、TUI Frontend (前端)

### 3.1 当前状态

**文件**: `src/tui/app.rs` (1330行)

**核心问题**:
1. 大一统 `App` struct - 600+行，多个职责混杂
2. 嵌套 `tokio::runtime::Runtime::new()` 反模式
3. 双重状态来源 (`streaming_text` + `response_receiver`)
4. `AppMode` 状态机混乱 (7种模式)
5. render/handle_key/event poll 代码全搅在一起

### 3.2 问题详解

#### 问题1: App struct 过于庞大

```rust
pub struct App {
    config: ConfigManager,
    mode: AppMode,
    messages: Vec<(String, String)>,
    input: String,
    cursor_char_pos: usize,
    should_quit: bool,
    streaming_text: Option<String>,
    response_receiver: Option<mpsc::Receiver<Result<String>>>,
    fetched_models: Vec<String>,
    model_fetch_error: Option<String>,
    model_fetch_receiver: Option<mpsc::Receiver<Result<Vec<String>>>>,
    system_prompt: Option<String>,
    scrollbar_state: ScrollbarState,
    scroll_offset: usize,
    content_height: usize,
    debug_scroll_offset: usize,
    debug_content_height: usize,
    agent: Agent,
    tool_registry: Arc<ToolRegistry>,
    show_debug: bool,
    debug_messages: Vec<String>,
    debug_log_path: Option<std::path::PathBuf>,
    debug_rx: Option<std::sync::mpsc::Receiver<String>>,
    debug_tx: Option<std::sync::mpsc::Sender<String>>,
}  // 22个字段！
```

#### 问题2: 嵌套Runtime

```rust
// app.rs 965行
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();  // 反模式
    rt.block_on(async {
        // ...
    });
});
```

#### 问题3: 状态机混乱

```rust
pub enum AppMode {
    Chat,
    ConfigProvider,
    ConfigApiKey,
    ConfigModel,
    ConfigUrl,
    ConfigCustomName,
    ConfigCustomUrl,
    ConfigFetchModels,
    ConfigSelectModel,  // 8种模式！
}
```

### 3.3 改进方向

**拆分建议**:
- `App` 拆分为: `ChatState`, `ConfigState`, `DebugState`
- 引入 `AppController` 作为状态机编排
- 移除嵌套Runtime，改为直接使用主线程的tokio runtime

**目标架构**:
```
AppController (状态机)
    │
    ├── ChatView (聊天界面状态)
    ├── ConfigView (配置界面状态)
    └── DebugView (调试面板状态)
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
    async fn stream_chat(&self, messages, on_chunk: FnMut(String)) -> Result<()>;
}

#[async_trait]
pub trait ToolCallingProvider: Send + Sync {
    async fn chat_with_tools(&self, messages, tools) -> Result<ModelResponse>;
    fn provider_name(&self) -> &str;
    async fn stream_chat_with_tools(&self, messages, tools, on_chunk) -> Result<()>;
}
```

### 4.2 问题

1. **接口不一致**: `stream_chat` 和 `stream_chat_with_tools` 使用相同的 `on_chunk(FnMut(String))`，但不同Provider返回内容格式不同
2. **tool call 表示混乱**: OpenAI/Anthropic用结构化JSON，DeepSeek用文本标签
3. **缺少统一响应类型**: `ModelResponse` 字段不完整

### 4.3 Provider实现差异表

| Provider | Tool Call格式 | Streaming | 特殊处理 |
|----------|---------------|-----------|----------|
| OpenAI | `tool_calls` JSON数组 | ✅ | 标准 |
| Anthropic | `tool_calls` JSON数组 | ✅ | 标准 |
| DeepSeek | **文本标签 `<tool>...` + reasoning_content** | ✅ | 非标准！ |
| Ollama | 无tool calling | ✅ | 不支持工具 |
| MiniMax | 未知 | ? | 待实现 |
| Custom | 未知 | ? | 待实现 |

### 4.4 DeepSeek特殊问题

**输入要求**:
```rust
"thinking": { "type": "enabled" },
"reasoning_effort": "high"  // 或 "low"
```

**输出格式** (stream_chat_with_tools):
```rust
"<thinking>\n" + reasoning_content + "\n</thinking>\n" +
"<content>\n" + content + "\n</content>\n" +
"<tool>\n" + name + "\n</tool>\n"  // 纯文本标签！
```

### 4.5 改进方向

1. **统一StreamChunk enum**: 区分 `Content`, `Thinking`, `ToolCall`, `Error`
2. **Provider adapter层**: 每个Provider实现一个Adapter，将自家格式转为统一格式
3. **Tool call 标准化**: 区分 "structured" vs "text_tag" 两种模式

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
│  │ 1. chat_with_tools_streaming() │ │
│  │    ↓                            │ │
│  │ 2. 解析 tool_calls              │ │
│  │    ├─ 从 JSON response.tool_calls│ │
│  │    └─ fallback: regex content   │ │
│  │    ↓                            │ │
│  │ 3. 如果无tool_calls → 返回content│ │
│  │    ↓                            │ │
│  │ 4. execute_tool_calls()         │ │
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

### 5.3 问题

1. **DeepSeek tool call解析失败**: `parse_tool_calls()` 期望 `{"tool_calls": [...]}` JSON，但DeepSeek输出 `<tool>name</tool>` 文本
2. **reasoning_content丢失**: DeepSeek的 `reasoning_content` 字段需要随tool结果发送回去，但当前没有累积
3. **单线程执行**: 工具串行执行，没有并发
4. **循环终止保护不足**: max_iterations=5可能被某些情况绕过

### 5.4 改进方向

1. **增加reasoning_history累积机制**
2. **统一ToolCall解析层**: 支持多种格式自动识别
3. **并发工具执行**: 使用 `tokio::join!` 或 `futures::future::join_all`

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

1. **Session定义了但几乎没用**: `Agent` 直接持有 `Vec<Message>`，Session未被使用
2. **没有context window管理**: 消息无限增长，没有截断/摘要机制

### 6.3 改进方向

- 提取 `ConversationManager` 统一管理消息历史
- 实现 context window 管理 (tokens计数 + 截断策略)

---

## 七、Tool System

### 7.1 当前状态

**架构**:
```
Tool trait (async_trait)
    │
    ├── name() / description() / input_schema()
    └── execute(call: ToolCall) -> ToolResult

ToolRegistry (HashMap<String, Box<dyn Tool>>)
    │
    ├── register(Tool)
    ├── execute(ToolCall) -> ToolResult
    └── get_definitions() -> Vec<ToolDefinition>

Built-in Tools:
    ├── BashTool (sh -c)
    ├── ReadTool (文件读取)
    ├── WriteTool (文件写入)
    ├── EditTool (行编辑)
    └── GrepTool (搜索)
```

### 7.2 问题

1. **工具注册硬编码**: 在 `App::new_internal()` 里 (app.rs 440-445)
2. **串行执行**: `execute_tool_calls()` 一个个顺序执行

### 7.3 改进方向

- 配置文件/skill动态加载工具
- 并发执行独立工具

---

## 八、MCP Client

### 8.1 当前状态

**协议** (`protocol.rs` 215行):
- JSON-RPC 2.0 envelope
- MCP types: InitializeParams, ToolsListParams, ToolCallParams等

**Client** (`client.rs` 197行):
- HTTP POST
- `connect()` → `initialize()` → `list_tools()` / `call_tool()`

### 8.2 问题

1. **没有被集成到Agent**: App里没有MCP client实例化代码
2. **没有tool bridging**: MCP tools没有转换成内部 ToolDefinition
3. **连接管理**: 没有重连机制

### 8.3 改进方向

- `MCPToolAdapter`: 将MCP Tool转换成内部 ToolDefinition格式
- 连接池/重连机制

---

## 九、根因分析

| 问题 | 根因 |
|------|------|
| 重复小bug | 大一统App struct + 状态机混乱 |
| DeepSeek tool call失败 | Provider接口和实现不一致 |
| reasoning_content丢失 | 没有累积-反馈机制 |
| MCP未集成 | client和agent之间缺少bridge |

**核心问题**: 架构设计时没有统一的接口约定，各Provider各行其是。

---

## 十、讨论进度

| 模块 | 状态 | 结论 |
|------|------|------|
| TUI Frontend | ✅ 已记录 | 待讨论 |
| Provider 接口 | ✅ 已记录 | 待讨论 |
| Agent Loop | ✅ 已记录 | 待讨论 |
| Session 管理 | ✅ 已记录 | 待讨论 |
| Tool System | ✅ 已记录 | 待讨论 |
| MCP Client | ✅ 已记录 | 待讨论 |

---

*文档版本: 1.0*
*最后更新: 2026-05-19*