# TUI State Machine Implementation Plan (Phase 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现核心状态机 + 修复消息不显示 bug（Phase 1）

**Architecture:** 四层状态机架构：AppController → TaskCoordinator → AgentTask → ViewState。并发使用 Channel-based 模式，统一事件传递。

**Tech Stack:** Rust, ratatui, tokio, mpsc channel

---

## Phase 1: 核心状态机实现

### Task 1: 创建事件定义模块

**Files:**
- Create: `rust-agent-platform/src/tui/task/events.rs`

- [ ] **Step 1: 创建 events.rs 文件**

```rust
use std::time::Duration;

/// 任务类型（动态优先级）
#[derive(Debug, Clone)]
pub enum TaskType {
    UserMessage { priority: u8 },
    SystemTask { priority: u8 },
    BackgroundFetch { priority: u8 },
}

impl TaskType {
    pub fn priority(&self) -> u8 {
        match self {
            TaskType::UserMessage { priority } => *priority,
            TaskType::SystemTask { priority } => *priority,
            TaskType::BackgroundFetch { priority } => *priority,
        }
    }

    pub fn max_retries(&self) -> u8 {
        match self {
            TaskType::UserMessage { .. } => 3,
            TaskType::SystemTask { .. } => 2,
            TaskType::BackgroundFetch { .. } => 0,
        }
    }
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::SystemTask { priority: 50 }
    }
}

/// 失败状态
#[derive(Debug, Clone)]
pub enum FailedState {
    NetworkError { message: String, retry_count: u8 },
    ProviderError { message: String, provider: String, retry_count: u8 },
    Timeout { waited_secs: u64, limit_secs: u64 },
    Cancelled,
}

impl FailedState {
    pub fn is_retriable(&self) -> bool {
        match self {
            FailedState::NetworkError { retry_count, .. } => *retry_count > 0,
            FailedState::ProviderError { retry_count, .. } => *retry_count > 0,
            FailedState::Timeout { .. } => true,
            FailedState::Cancelled => false,
        }
    }

    pub fn with_decrement(&self) -> Self {
        match self {
            FailedState::NetworkError { message, retry_count } => {
                FailedState::NetworkError { message: message.clone(), retry_count: *retry_count - 1 }
            }
            FailedState::ProviderError { message, provider, retry_count } => {
                FailedState::ProviderError { message: message.clone(), provider: provider.clone(), retry_count: *retry_count - 1 }
            }
            other => other.clone(),
        }
    }
}

/// 运行子状态
#[derive(Debug, Clone, PartialEq)]
pub enum RunningState {
    Initializing,
    WaitingProvider,
    StreamingChunks,
    ProcessingToolCall,
}

/// AgentTask 状态机
#[derive(Debug, Clone)]
pub enum AgentTaskState {
    Pending,
    Running(RunningState),
    Completed,
    Failed(FailedState),
    Suspended,
    Cancelled,
}

impl AgentTaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentTaskState::Completed | AgentTaskState::Failed(_) | AgentTaskState::Cancelled)
    }
}

/// Coordinator 状态
#[derive(Debug, Clone)]
pub enum CoordinatorState {
    Idle,
    Managing { active_task_id: Uuid, pending_queue: Vec<QueuedTask> },
}

/// 队列任务
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: Uuid,
    pub task_type: TaskType,
    pub state: AgentTaskState,
}

/// Coordinator 事件
#[derive(Debug, Clone)]
pub enum CoordinatorEvent {
    TaskEnqueued { task_id: Uuid, task_type: TaskType },
    TaskStarted { task_id: Uuid },
    TaskCompleted { task_id: Uuid },
    TaskCancelled { task_id: Uuid },
    Preempted { high_priority: Uuid, low_priority: Uuid },
    TaskSuspended { task_id: Uuid },
    TaskResumed { task_id: Uuid },
}

/// Task 事件
#[derive(Debug, Clone)]
pub enum TaskEvent {
    Spawned,
    Chunk { content: String },
    ToolCall { name: String, input: String },
    ToolResult { name: String, result: String },
    Completed { content: String },
    Failed { state: FailedState },
    Suspended,
    Resumed,
}
```

- [ ] **Step 2: 提交**

```bash
git add rust-agent-platform/src/tui/task/events.rs
git commit -m "feat(tui): add TaskEvent, FailedState, and TaskType enums"
```

---

### Task 2: 创建 task 模块

**Files:**
- Create: `rust-agent-platform/src/tui/task/mod.rs`
- Modify: `rust-agent-platform/src/tui/mod.rs`

- [ ] **Step 1: 创建 task/mod.rs**

```rust
pub mod events;
pub mod coordinator;

pub use events::*;
pub use coordinator::*;
```

- [ ] **Step 2: 修改 tui/mod.rs 添加 task 模块**

```rust
pub mod state;
pub mod views;
pub mod components;
pub mod task;  // 新增
pub mod config;
```

- [ ] **Step 3: 提交**

```bash
git add rust-agent-platform/src/tui/task/mod.rs rust-agent-platform/src/tui/mod.rs
git commit -m "feat(tui): add task module"
```

---

### Task 3: 重写 ViewState Enum

**Files:**
- Modify: `rust-agent-platform/src/tui/state/view_state.rs`

- [ ] **Step 1: 读取当前 view_state.rs**

```bash
cat rust-agent-platform/src/tui/state/view_state.rs
```

- [ ] **Step 2: 重写 ChatContext 和 ConfigContext**

```rust
use std::sync::{Arc, Mutex};

/// Chat 视图上下文
#[derive(Debug, Clone)]
pub struct ChatContext {
    pub current_provider: String,
    pub current_model: String,
}

impl ChatContext {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            current_provider: provider.to_string(),
            current_model: model.to_string(),
        }
    }
}

/// Config 视图上下文
#[derive(Debug, Clone)]
pub struct ConfigContext {
    pub editing_provider: Option<String>,
}

impl Default for ConfigContext {
    fn default() -> Self {
        Self {
            editing_provider: None,
        }
    }
}

/// View 状态枚举（替换原来的 is_streaming 等 bool）
#[derive(Debug, Clone)]
pub enum ViewState {
    Chat(ChatContext),
    Config(ConfigContext),
}

impl ViewState {
    pub fn as_chat(&self) -> Option<&ChatContext> {
        match self {
            ViewState::Chat(ctx) => Some(ctx),
            _ => None,
        }
    }

    pub fn as_config(&self) -> Option<&ConfigContext> {
        match self {
            ViewState::Config(ctx) => Some(ctx),
            _ => None,
        }
    }
}

/// ChatView 状态（从 ChatViewState 重命名）
#[derive(Debug, Clone)]
pub struct ChatViewState {
    pub messages: Vec<(String, String)>,
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub scrollbar_state: ScrollbarState,
    pub debug_scrollbar_state: ScrollbarState,
    pub debug_scroll_offset: usize,
    pub debug_content_height: usize,
    pub is_streaming: bool,
    pub current_provider: String,
    pub current_model: String,
}

impl ChatViewState {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            content_height: 0,
            scrollbar_state: ScrollbarState::new(0),
            debug_scrollbar_state: ScrollbarState::new(0),
            debug_scroll_offset: 0,
            debug_content_height: 0,
            is_streaming: false,
            current_provider: provider.to_string(),
            current_model: model.to_string(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push((role.to_string(), content.to_string()));
    }
}

/// ConfigView 状态
#[derive(Debug, Clone, Default)]
pub struct ConfigViewState {
    pub editing_provider: Option<String>,
}

impl ConfigViewState {
    pub fn new() -> Self {
        Self::default()
    }
}

// ScrollbarState 需要从 ratatui 导入
use ratatui::widgets::ScrollbarState;
```

- [ ] **Step 3: 提交**

```bash
git add rust-agent-platform/src/tui/state/view_state.rs
git commit -m "refactor(tui): rewrite ViewState with ChatContext and ConfigContext"
```

---

### Task 4: 清理 Action Enum（移除 is_streaming 等隐式状态）

**Files:**
- Modify: `rust-agent-platform/src/tui/state/action.rs`

- [ ] **Step 1: 读取当前 action.rs**

```bash
cat rust-agent-platform/src/tui/state/action.rs
```

- [ ] **Step 2: 修改 Action Enum（添加状态转换事件）**

```rust
use crossterm::event::{KeyEvent};

/// 用户操作枚举
#[derive(Debug, Clone)]
pub enum Action {
    // 消息操作
    SendMessage(String),
    CancelStreaming,

    // View 操作
    SwitchView(ViewSwitch),
    GoBack,

    // 滚动操作
    ScrollUp,
    ScrollDown,
    ScrollDebugUp,
    ScrollDebugDown,

    // 配置操作
    SelectProvider(String),
    SaveApiKey(String),
    SaveModel(String),
    SaveBaseUrl(String),
    FetchModels,

    // 调试操作（保留但内部使用日志）
    ToggleDebug,

    // 退出
    Quit,
}

/// View 切换目标
#[derive(Debug, Clone)]
pub enum ViewSwitch {
    Chat(ChatContext),
    Config(ConfigContext),
}

impl From<ViewSwitch> for Action {
    fn from(switch: ViewSwitch) -> Self {
        Action::SwitchView(switch)
    }
}

// 重新导出 ChatContext, ConfigContext
pub use super::view_state::{ChatContext, ConfigContext, ViewState, ChatViewState, ConfigViewState};
```

- [ ] **Step 3: 提交**

```bash
git add rust-agent-platform/src/tui/state/action.rs
git commit -m "refactor(tui): update Action enum with explicit state transitions"
```

---

### Task 5: 修复 spawn_agent_task 的 clear_streaming() 时机问题

**Files:**
- Modify: `rust-agent-platform/src/tui/app_controller.rs`

- [ ] **Step 1: 读取当前 app_controller.rs**

```bash
cat rust-agent-platform/src/tui/app_controller.rs
```

- [ ] **Step 2: 关键修复 - 在 async task 内部调用 clear_streaming()，而不是之前**

```rust
fn spawn_agent_task(&mut self, msg: String) {
    let runtime = self.runtime.handle().clone();

    let (tx, rx) = mpsc::channel();
    *self.ui_state.response_receiver.lock().unwrap() = Some(rx);

    // 修复: 不要在这里 clear_streaming()！
    // 旧代码: self.ui_state.clear_streaming();  // ← 错误！时机关闭

    let provider_name = self.context.config.lock().unwrap().get_provider().to_string();
    let api_key = self.context.config.lock().unwrap().get_api_key(&provider_name);
    let base_url = self.context.config.lock().unwrap().get_base_url(&provider_name);
    let model = self.context.config.lock().unwrap().get_model(&provider_name).to_string();
    let agent = self.context.agent.clone();
    let debug_path = self.context.debug_log_path.clone();

    let streaming_text = Arc::clone(&self.ui_state.streaming_text);
    let debug_messages = Arc::clone(&self.ui_state.debug_messages);
    let finalized_message = Arc::clone(&self.ui_state.finalized_message);

    runtime.spawn(async move {
        // 修复: 在 task 开始时 clear，而不是在 spawn 前
        {
            if let Ok(mut streaming) = streaming_text.lock() {
                *streaming = None;  // 清空之前的 streaming_text
            }
        }

        let model_opt = Some(model.clone());

        let final_agent: Arc<Agent> = match provider_name.as_str() {
            "openai" => {
                let p = OpenAIProvider::new(api_key.unwrap_or_default(), base_url);
                Arc::new(Agent::clone(&agent).with_tool_provider("openai", p))
            }
            "anthropic" => {
                let p = AnthropicProvider::new(api_key.unwrap_or_default(), base_url);
                Arc::new(Agent::clone(&agent).with_tool_provider("anthropic", p))
            }
            "ollama" => {
                let p = OllamaProvider::new(base_url, model_opt);
                Arc::new(Agent::clone(&agent).with_tool_provider("ollama", p))
            }
            "deepseek" => {
                let p = DeepSeekProvider::new(api_key.unwrap_or_default(), base_url, model_opt);
                Arc::new(Agent::clone(&agent).with_tool_provider("deepseek", p))
            }
            "minimax" => {
                let p = MiniMaxProvider::new(api_key.unwrap_or_default(), base_url, model_opt);
                Arc::new(Agent::clone(&agent).with_tool_provider("minimax", p))
            }
            "custom" => {
                let p = CustomProvider::new(
                    "custom".to_string(),
                    api_key.unwrap_or_default(),
                    base_url.unwrap_or_default(),
                    model_opt.unwrap_or_default(),
                );
                Arc::new(Agent::clone(&agent).with_tool_provider("custom", p))
            }
            _ => agent,
        };

        {
            if let Ok(mut msgs) = debug_messages.lock() {
                msgs.push(format!("Starting request to {}", provider_name));
                if msgs.len() > 100 {
                    msgs.remove(0);
                }
            }
        }

        let result = final_agent.run_agent_loop(
            &msg,
            5,
            debug_path.map(|p| p.to_string_lossy().to_string()),
            Some(tx),
        ).await;

        match result {
            Ok(response) => {
                if let Ok(mut msgs) = debug_messages.lock() {
                    msgs.push(format!("Response length: {}", response.len()));
                    if msgs.len() > 100 {
                        msgs.remove(0);
                    }
                }
                // 修复: 直接使用 response 设置 finalized_message
                if !response.is_empty() {
                    if let Ok(mut finalized) = finalized_message.lock() {
                        *finalized = Some(("assistant".to_string(), response));
                    }
                }
            }
            Err(e) => {
                if let Ok(mut msgs) = debug_messages.lock() {
                    msgs.push(format!("Error: {}", e));
                    if msgs.len() > 100 {
                        msgs.remove(0);
                    }
                }
                // 错误处理也要正确设置
                let partial = streaming_text.lock()
                    .map(|s| s.take().unwrap_or_default())
                    .unwrap_or_default();
                if let Ok(mut finalized) = finalized_message.lock() {
                    *finalized = Some(("error".to_string(), format!("{} (partial: {})", e, partial)));
                }
            }
        }
    });
}
```

- [ ] **Step 3: 提交**

```bash
git add rust-agent-platform/src/tui/app_controller.rs
git commit -m "fix(tui): spawn_agent_task clear_streaming timing bug"
```

---

### Task 6: 修复 debug_tx 被覆盖的问题

**Files:**
- Modify: `rust-agent-platform/src/tui/app_controller.rs:217`

- [ ] **Step 1: 找到并删除覆盖 debug_tx 的代码**

在 `spawn_agent_task` 中，删除：
```rust
let (debug_tx, _debug_rx) = mpsc::channel();  // ← 删除这行
```

- [ ] **Step 2: 确保使用正确的 debug_tx**

```rust
let debug_tx = self.ui_state.debug_tx.clone();
// 删除多余的 channel 创建，直接使用 clone 得到的 tx
```

- [ ] **Step 3: 提交**

```bash
git add rust-agent-platform/src/tui/app_controller.rs
git commit -m "fix(tui): remove debug_tx shadowing bug"
```

---

### Task 7: 修改 ChatView 移除 Debug 面板渲染

**Files:**
- Modify: `rust-agent-platform/src/tui/views/chat_view.rs`

- [ ] **Step 1: 简化 render 方法，移除 debug panel**

```rust
impl View for ChatView {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState) {
        // 移除 debug_height 逻辑，简化为只有 messages + input + status
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Messages area
                Constraint::Length(3),   // Input area
                Constraint::Length(1),   // Status bar
            ])
            .split(area);

        self.render_messages(f, chunks[0], ui_state);
        self.render_input(f, chunks[1]);
        self.render_status(f, chunks[2]);

        // 移除 debug panel 相关代码
    }
}
```

- [ ] **Step 2: 提交**

```bash
git add rust-agent-platform/src/tui/views/chat_view.rs
git commit -m "refactor(tui): remove debug panel from ChatView, use logging instead"
```

---

### Task 8: 修改 app_controller.rs 使用新 ViewState Enum

**Files:**
- Modify: `rust-agent-platform/src/tui/app_controller.rs`

- [ ] **Step 1: 更新 handle_action 中的 ViewState 使用**

```rust
Action::SwitchView(view_state) => {
    self.current_view.on_exit();
    self.current_view = match view_state {
        ViewState::Chat(state) => {
            let mut view = ChatView::new(&state.current_provider, &state.current_model);
            view.state = state;
            Box::new(view)
        }
        ViewState::Config(state) => {
            let mut view = ConfigView::new();
            view.state = state;
            Box::new(view)
        }
    };
    self.current_view.on_enter();
}
```

- [ ] **Step 2: 提交**

```bash
git add rust-agent-platform/src/tui/app_controller.rs
git commit -m "refactor(tui): adopt new ViewState enum in AppController"
```

---

### Task 9: 验证构建通过

- [ ] **Step 1: 运行 clippy 检查**

```bash
cd rust-agent-platform && cargo clippy --all-targets 2>&1 | head -50
```

- [ ] **Step 2: 如有错误，修复后重新提交**

- [ ] **Step 3: 运行测试**

```bash
cd rust-agent-platform && cargo test 2>&1 | tail -30
```

---

### Task 10: 创建 TaskCoordinator stub（Phase 1 不完整实现，仅支持单任务）

**Files:**
- Create: `rust-agent-platform/src/tui/task/coordinator.rs`

- [ ] **Step 1: 创建 TaskCoordinator stub**

```rust
use crate::tui::task::events::{CoordinatorEvent, TaskEvent, TaskType, Uuid};
use std::collections::VecDeque;

/// TaskCoordinator - 管理任务队列和调度
/// Phase 1: 仅支持单任务，排队逻辑为 stub
#[derive(Debug)]
pub struct TaskCoordinator {
    state: CoordinatorState,
    event_queue: VecDeque<CoordinatorEvent>,
}

#[derive(Debug, Clone)]
pub enum CoordinatorState {
    Idle,
    Managing { active_task_id: Uuid, pending_queue: VecDeque<QueuedTask> },
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: Uuid,
    pub task_type: TaskType,
}

impl TaskCoordinator {
    pub fn new() -> Self {
        Self {
            state: CoordinatorState::Idle,
            event_queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, task_type: TaskType) -> Uuid {
        let task_id = Uuid::new_v4();
        let event = CoordinatorEvent::TaskEnqueued {
            task_id,
            task_type,
        };
        self.event_queue.push_back(event.clone());
        self.process_event(event);
        task_id
    }

    fn process_event(&mut self, event: CoordinatorEvent) {
        match event {
            CoordinatorEvent::TaskEnqueued { task_id, task_type } => {
                match &mut self.state {
                    CoordinatorState::Idle => {
                        self.state = CoordinatorState::Managing {
                            active_task_id: task_id,
                            pending_queue: VecDeque::new(),
                        };
                    }
                    CoordinatorState::Managing { pending_queue, .. } => {
                        pending_queue.push_back(QueuedTask { task_id, task_type });
                    }
                }
            }
            CoordinatorEvent::TaskCompleted { task_id } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskCancelled { task_id } => {
                self.transition_to_idle_or_next();
            }
            _ => {}
        }
    }

    fn transition_to_idle_or_next(&mut self) {
        match &mut self.state {
            CoordinatorState::Managing { pending_queue, .. } => {
                if let Some(next) = pending_queue.pop_front() {
                    self.state = CoordinatorState::Managing {
                        active_task_id: next.task_id,
                        pending_queue: pending_queue.clone(),
                    };
                } else {
                    self.state = CoordinatorState::Idle;
                }
            }
            _ => {}
        }
    }

    pub fn state(&self) -> &CoordinatorState {
        &self.state
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, CoordinatorState::Idle)
    }
}

impl Default for TaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 提交**

```bash
git add rust-agent-platform/src/tui/task/coordinator.rs
git commit -m "feat(tui): add TaskCoordinator stub for Phase 1"
```

---

## 总结

Phase 1 实现完成后，你应该有以下改动：

| 文件 | 变更 |
|------|------|
| `src/tui/task/events.rs` | 新增：TaskEvent, FailedState, TaskType, AgentTaskState, CoordinatorState |
| `src/tui/task/mod.rs` | 新增：task 模块 |
| `src/tui/task/coordinator.rs` | 新增：TaskCoordinator stub |
| `src/tui/state/view_state.rs` | 重写：ViewState, ChatContext, ConfigContext |
| `src/tui/state/action.rs` | 修改：Action Enum |
| `src/tui/app_controller.rs` | 修复：clear_streaming() 时机、debug_tx 覆盖 |
| `src/tui/views/chat_view.rs` | 移除：Debug panel |

**核心 bug 修复**:
1. `clear_streaming()` 移到 async task 内部
2. `debug_tx` 覆盖问题已修复
3. `finalized_message` 直接使用 `response` 设置

---

**Plan 完成**。文件保存在 `docs/specs/2026-05-27-tui-state-machine-design.md` 对应的计划位置。

**两个执行选项**:

**1. Subagent-Driven (recommended)** - 我 dispatch 一个 subagent per task，task 间 review，快速迭代

**2. Inline Execution** - 在本 session 中使用 executing-plans 执行，带检查点

你想用哪个方式？