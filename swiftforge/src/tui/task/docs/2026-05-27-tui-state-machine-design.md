# TUI 状态机设计规范

> 版本: 1.0
> 日期: 2026-05-27
> 状态: 设计中
> 关联: brainstormingsession

---

## 一、概述

### 1.1 目的

本规范定义 TUI 模块的**状态机设计原则**，为后续开发提供统一的标准规范。核心目标是：

- 消除隐式状态依赖
- 建立显式状态转换机制
- 统一数据传输语义
- 降低并发 bug 率

### 1.2 背景

当前 TUI 重构存在以下问题：

| 问题 | 根因 |
|------|------|
| 消息不显示 | `clear_streaming()` 时机错误，`finalized_message` 未被设置 |
| Debug 面板失效 | `debug_tx` 被覆盖 |
| 状态机不完整 | Streaming 状态没有终态处理 |
| 多 writer 竞争 | `streaming_text` 被多个消费者共用，无同步 |

### 1.3 范围

覆盖 TUI 模块的所有状态机设计：

- `AppController` - 全局控制器
- `TaskCoordinator` - 任务协调层
- `AgentTask` - 单任务生命周期
- `ViewState` - 视图状态

**不包括**：

- DebugView（已移除，统一使用日志）
- Provider 层（独立模块）

---

## 二、并发安全策略

### 2.1 传输机制选择

**统一使用 Channel-based 模式**，不使用共享状态（`Arc<Mutex>`）传递数据。

**原则**：
- 数据通过 channel 传递，而非共享可变状态
- Channel 提供天然的 FIFO 队列和同步点
- 每个组件持有自己的 receiver，不共享

### 2.2 状态信号

使用**单 Event Channel + 分层 Event Enum**：

```rust
// 统一事件通道
struct TaskSignal {
    tx: mpsc::Sender<TaskEvent>,
}

// 分层事件定义
enum CoordinatorEvent {
    TaskEnqueued { task_id: Uuid, task_type: TaskType },
    TaskStarted { task_id: Uuid },
    TaskCompleted { task_id: Uuid },
    TaskCancelled { task_id: Uuid },
    Preempted { high_priority: Uuid, low_priority: Uuid },
}

enum TaskEvent {
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

### 2.3 UIState 用途

`UIState` 仅用于**渲染数据存储**，不用于跨线程通信：

```rust
struct UIState {
    // 渲染数据（非通信）
    streaming_text: Arc<Mutex<Option<String>>>,
    debug_messages: Arc<Mutex<Vec<String>>>,
    // ...
}
```

- View 通过 `&UIState` 读取渲染数据
- 数据更新通过 Event Channel 信号触发

---

## 三、层级状态机定义

### 3.1 四层结构概览

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: AppController                                      │
│ 职责: 全局协调、View 切换                                    │
│ State: ViewState::Chat | Config                            │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: TaskCoordinator                                    │
│ 职责: 任务队列管理、优先级调度、抢占控制                      │
│ State: CoordinatorState                                    │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: AgentTask                                          │
│ 职责: 单任务生命周期、流式处理、错误重试                      │
│ State: AgentTaskState                                      │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: ViewState                                          │
│ 职责: 视图渲染状态、用户交互                                 │
│ State: ChatContext | ConfigContext                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Layer 1: AppController

### 状态定义

```rust
enum ViewState {
    Chat(ChatContext),
    Config(ConfigContext),
}

struct ChatContext {
    current_provider: String,
    current_model: String,
}

struct ConfigContext {
    editing_provider: Option<String>,
}
```

### 职责

- 持有 `TaskCoordinator` 实例
- 处理用户输入，转发为 Task
- 管理 View 切换

---

## Layer 2: TaskCoordinator

### 状态定义

```rust
enum CoordinatorState {
    Idle,
    Managing { active_task: Uuid, pending_queue: Vec<QueuedTask> },
}

struct QueuedTask {
    task_id: Uuid,
    task_type: TaskType,
    priority: u8,
    state: AgentTaskState,
}
```

### 队列策略

**优先级队列 + 抢占机制**：

```rust
enum TaskType {
    UserMessage { priority: u8 },
    SystemTask { priority: u8 },
    BackgroundFetch { priority: u8 },
}
```

**优先级默认值**：
- `UserMessage`: priority = 100
- `SystemTask`: priority = 50
- `BackgroundFetch`: priority = 10

**抢占行为**：
- 高优先级任务到达时，低优先级任务挂起（Suspended）
- 挂起的任务可恢复（Resumed）

### 事件定义

```rust
enum CoordinatorEvent {
    TaskEnqueued { task_id: Uuid, task_type: TaskType },
    TaskStarted { task_id: Uuid },
    TaskCompleted { task_id: Uuid },
    TaskCancelled { task_id: Uuid },
    Preempted { high_priority: Uuid, low_priority: Uuid },
    TaskSuspended { task_id: Uuid },
    TaskResumed { task_id: Uuid },
}
```

---

## Layer 3: AgentTask

### 状态定义

```rust
enum AgentTaskState {
    Pending,                          // 等待执行
    Running(RunningState),            // 运行中（含子状态）
    Completed,                         // 成功完成
    Failed(FailedState),               // 失败
    Suspended,                         // 被挂起（等待恢复）
    Cancelled,                         // 被取消
}

enum RunningState {
    Initializing,      // 初始化中
    WaitingProvider,  // 等待 Provider 响应
    StreamingChunks,  // 流式接收 Chunks
    ProcessingToolCall, // 处理工具调用
}

enum FailedState {
    NetworkError { message: String, retry_count: u8 },
    ProviderError { message: String, provider: String, retry_count: u8 },
    Timeout { waited_secs: u64, limit_secs: u64 },
    Cancelled,
}
```

### 状态转换表

| 当前状态 | 事件 | 目标状态 | Guard 条件 |
|----------|------|----------|------------|
| Pending | TaskStarted | Running(Initializing) | - |
| Running(Initializing) | ProviderReady | Running(WaitingProvider) | - |
| Running(WaitingProvider) | FirstChunk | Running(StreamingChunks) | - |
| Running(StreamingChunks) | ChunkReceived | Running(StreamingChunks) | - |
| Running(StreamingChunks) | TaskCompleted | Completed | channel closed |
| Running(StreamingChunks) | ToolCallStarted | Running(ProcessingToolCall) | - |
| Running(ProcessingToolCall) | ToolCallCompleted | Running(StreamingChunks) | - |
| Running(*) | CancelRequested | Cancelled | - |
| Running(*) | SuspendRequested | Suspended | priority < active_priority |
| Suspended | ResumeRequested | Running(StreamingChunks) | - |
| Pending | Error | Failed | - |
| Running(*) | Error | Failed | - |

### 重试策略

**按 TaskType 区分重试次数**：

```rust
impl TaskType {
    fn max_retries(&self) -> u8 {
        match self {
            TaskType::UserMessage { .. } => 3,
            TaskType::SystemTask { .. } => 2,
            TaskType::BackgroundFetch { .. } => 0, // 不重试
        }
    }
}
```

**可重试错误判断**：

```rust
fn is_retriable(error: &FailedState) -> bool {
    match error {
        FailedState::NetworkError { retry_count, .. } => *retry_count > 0,
        FailedState::ProviderError { retry_count, .. } => *retry_count > 0,
        FailedState::Timeout { .. } => true,
        FailedState::Cancelled => false,
    }
}
```

### 事件定义

```rust
enum TaskEvent {
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

---

## Layer 4: ViewState

### 状态定义

```rust
enum ViewState {
    Chat(ChatContext),
    Config(ConfigContext),
}

struct ChatContext {
    current_provider: String,
    current_model: String,
    // 迁移自旧 ChatViewState
}

struct ConfigContext {
    editing_provider: Option<String>,
}
```

### 职责

- 纯渲染状态
- 不参与业务逻辑
- 通过 `&UIState` 读取共享渲染数据

---

## 四、数据流设计

### 4.1 统一数据流

```
User Input
    │
    ▼
AppController.handle_action(Action::SendMessage)
    │
    ▼
TaskCoordinator.enqueue(TaskType::UserMessage { priority: 100 })
    │
    ▼
AgentTask.spawn() → run_agent_loop()
    │
    ├───► TaskEvent::Chunk → Channel → Coordinator → UIState
    │
    └───► TaskEvent::Completed → Channel → Coordinator
              │
              ▼
         ChatView.state.messages.push(("assistant", content))
              │
              ▼
         terminal.draw() → render_messages()
```

### 4.2 Channel 与 Shared State 的边界

| 数据 | 传输机制 | 说明 |
|------|----------|------|
| Task 信号 | Channel | Event Enum 传递 |
| Streaming chunks | Channel | 实时传递 |
| 渲染数据 | UIState (Arc<Mutex>) | 仅 UI 读取 |
| 完成信号 | Channel | CoordinatorEvent |

**原则**：
- Channel 用于时序敏感的信号传递
- Shared State 仅用于渲染数据存储
- 两者不混用

### 4.3 状态表示

**统一使用显式 State Enum**：

```rust
enum AgentState {
    Idle,
    Spawning,
    Streaming,
    Finalizing,
    Cancelled,
    Error(String),
}
```

**禁止使用**：
- 字符串标记状态（`"idle"`, `"running"`）
- 隐式 bool 标志（`is_streaming: bool`）

---

## 五、错误处理

### 5.1 错误分类

```rust
enum FailedState {
    NetworkError { message: String, retry_count: u8 },
    ProviderError { message: String, provider: String, retry_count: u8 },
    Timeout { waited_secs: u64, limit_secs: u64 },
    Cancelled,
}
```

### 5.2 错误处理流程

```
Task Failed
    │
    ▼
is_retriable(error)?
    │
    ├─── YES ──► retry_count > 0?
    │                │
    │                ├─── YES ──► 重试（retry_count - 1）
    │                │
    │                └─── NO  ──► Failed(FailedState)
    │
    └─── NO ──► Failed(FailedState)
```

### 5.3 用户消息错误展示

- 可重试错误：显示 "重试中 (n/3)" + 错误信息
- 不可重试错误：显示完整错误信息 + 部分结果（如果有）

---

## 六、规范约束

### 6.1 状态机设计约束

- [ ] 每个状态机必须有**显式 State Enum**
- [ ] 每个状态必须有**明确的入口/出口条件**
- [ ] 状态转换必须**显式调用**，禁止隐式转换
- [ ] 状态机必须有**明确的终态**

### 6.2 并发设计约束

- [ ] 使用 Channel 传递数据，禁止共享可变状态
- [ ] `Arc<Mutex>` 仅用于 UIState 渲染数据
- [ ] 多个 writer 的场景必须使用 Channel 同步
- [ ] 禁止在 channel 关闭事件上依赖隐式行为

### 6.3 错误处理约束

- [ ] 错误必须分类到 `FailedState` variant
- [ ] 可重试错误必须记录 `retry_count`
- [ ] 用户消息错误必须显示 partial result

### 6.4 View 设计约束

- [ ] View 仅负责渲染，不含业务逻辑
- [ ] View 通过 `&UIState` 读取数据
- [ ] 移除 DebugView，统一使用日志

---

## 七、附录

### 7.1 术语表

| 术语 | 定义 |
|------|------|
| State Enum | 显式状态枚举，如 `enum AgentState { Idle, Running }` |
| State Transition | 状态转换，从一个状态变为另一个状态 |
| Guard Condition | 状态转换的前置条件 |
| Channel-based | 使用 mpsc channel 传递数据的设计模式 |
| Suspend/Resume | 任务挂起和恢复机制 |

### 7.2 参考文档

- `docs/records/bugs/bug-2026-05-26-streaming-pipeline-architecture.md`
- `docs/records/bugs/bug-2026-05-26-streaming-pipeline-analysis.md`

---

*文档状态: 设计完成，待实现*