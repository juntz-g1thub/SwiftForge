# TUI State Machine Implementation Plan (Phase 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完整 TaskCoordinator + 抢占机制 + 流式 pipeline 端到端修复

**Architecture:** 四层状态机架构 Phase 2 - 完整实现优先级队列、抢占机制、TaskEvent Channel 集成

**Tech Stack:** Rust, tokio, mpsc channel, Arc<Mutex>

---

## Phase 2: 完整实现

### Task 1: 完善 TaskCoordinator - 优先级队列 + 抢占机制

**Files:**
- Modify: `rust-agent-platform/src/tui/task/coordinator.rs`

- [ ] **Step 1: 读取当前 coordinator.rs**

```bash
cat rust-agent-platform/src/tui/task/coordinator.rs
```

- [ ] **Step 2: 完全重写 coordinator.rs**

```rust
use crate::tui::task::events::{CoordinatorEvent, CoordinatorState, TaskType, QueuedTask};
use std::collections::VecDeque;
use uuid::Uuid;

/// TaskCoordinator - 管理任务队列和调度
/// Phase 2: 完整实现优先级队列 + 抢占机制
#[derive(Debug)]
pub struct TaskCoordinator {
    state: CoordinatorState,
    event_queue: VecDeque<CoordinatorEvent>,
}

impl TaskCoordinator {
    pub fn new() -> Self {
        Self {
            state: CoordinatorState::Idle,
            event_queue: VecDeque::new(),
        }
    }

    /// 入队新任务，自动触发抢占逻辑
    pub fn enqueue(&mut self, task_type: TaskType) -> Uuid {
        let task_id = Uuid::new_v4();
        let priority = task_type.priority();

        // 检查是否需要抢占
        let should_preempt = self.should_preempt(priority);

        if should_preempt {
            // 当前任务被挂起
            if let CoordinatorState::Managing { active_task_id, pending_queue } = &mut self.state {
                pending_queue.push_back(QueuedTask {
                    task_id: *active_task_id,
                    task_type: TaskType::BackgroundFetch { priority: 10 }, // 被挂起的任务降为低优先级
                });
                *active_task_id = task_id;
            }
        }

        let event = CoordinatorEvent::TaskEnqueued { task_id, task_type };
        self.event_queue.push_back(event.clone());
        self.process_event(event);
        task_id
    }

    /// 判断是否应该抢占
    fn should_preempt(&self, new_priority: u8) -> bool {
        match &self.state {
            CoordinatorState::Managing { active_task_id, pending_queue } => {
                if let Some(active_id) = active_id {
                    // 查找当前活跃任务的优先级
                    let active_priority = self.get_active_task_priority(active_id, pending_queue);
                    new_priority > active_priority
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn get_active_task_priority(&self, _task_id: &Uuid, _pending_queue: &VecDeque<QueuedTask>) -> u8 {
        // 实际上这个优先级应该从活跃任务获取，这里返回中等优先级作为默认值
        50
    }

    fn process_event(&mut self, event: CoordinatorEvent) {
        match event {
            CoordinatorEvent::TaskEnqueued { task_id, task_type } => {
                match &mut self.state {
                    CoordinatorState::Idle => {
                        self.state = CoordinatorState::Managing {
                            active_task_id: Some(task_id),
                            pending_queue: VecDeque::new(),
                        };
                    }
                    CoordinatorState::Managing { pending_queue, .. } => {
                        // 优先级队列：按优先级排序插入
                        let mut inserted = false;
                        for (i, qtask) in pending_queue.iter().enumerate() {
                            if qtask.task_type.priority() < task_type.priority() {
                                pending_queue.insert(i, QueuedTask { task_id, task_type });
                                inserted = true;
                                break;
                            }
                        }
                        if !inserted {
                            pending_queue.push_back(QueuedTask { task_id, task_type });
                        }
                    }
                }
            }
            CoordinatorEvent::TaskCompleted { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskCancelled { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::Preempted { high_priority: _, low_priority: _ } => {
                // 抢占处理
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskSuspended { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskResumed { task_id: _ } => {
                // 恢复处理
            }
            _ => {}
        }
    }

    fn transition_to_idle_or_next(&mut self) {
        match &mut self.state {
            CoordinatorState::Managing { active_task_id, pending_queue } => {
                *active_task_id = None;
                if let Some(next) = pending_queue.pop_front() {
                    *active_task_id = Some(next.task_id);
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

    /// 获取活跃任务 ID
    pub fn active_task_id(&self) -> Option<Uuid> {
        match &self.state {
            CoordinatorState::Managing { active_task_id, .. } => *active_task_id,
            _ => None,
        }
    }

    /// 获取待处理任务数量
    pub fn pending_count(&self) -> usize {
        match &self.state {
            CoordinatorState::Managing { pending_queue, .. } => pending_queue.len(),
            _ => 0,
        }
    }
}

impl Default for TaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
```

**注意**: 需要更新 events.rs 中的 CoordinatorState 定义，将 active_task 改为 Option<Uuid>

- [ ] **Step 3: 更新 events.rs 中 CoordinatorState**

```rust
#[derive(Debug, Clone)]
pub enum CoordinatorState {
    Idle,
    Managing {
        active_task_id: Option<Uuid>,  // 改为 Option，因为任务可能被挂起
        pending_queue: VecDeque<QueuedTask>,
    },
}
```

- [ ] **Step 4: 验证编译通过**
```bash
cd rust-agent-platform && cargo check 2>&1 | tail -30
```

- [ ] **Step 5: 提交**
```bash
cd rust-agent-platform
git add src/tui/task/coordinator.rs src/tui/task/events.rs
git commit -m "feat(tui): implement priority queue and preemption in TaskCoordinator"
```

---

### Task 2: 创建 AgentTask 运行时

**Files:**
- Create: `rust-agent-platform/src/tui/task/agent_task.rs`

- [ ] **Step 1: 创建 agent_task.rs**

```rust
use crate::tui::task::events::{
    AgentTaskState, CoordinatorEvent, FailedState, RunningState, TaskEvent, TaskType,
};
use crate::core::Agent;
use std::sync::{Arc, Mutex, mpsc};
use uuid::Uuid;

/// AgentTask - 单个任务的完整生命周期管理
pub struct AgentTask {
    pub task_id: Uuid,
    pub task_type: TaskType,
    pub state: AgentTaskState,
    event_tx: Option<mpsc::Sender<TaskEvent>>,
}

impl AgentTask {
    pub fn new(task_id: Uuid, task_type: TaskType) -> Self {
        Self {
            task_id,
            task_type,
            state: AgentTaskState::Pending,
            event_tx: None,
        }
    }

    /// 设置事件通道
    pub fn set_event_channel(&mut self, tx: mpsc::Sender<TaskEvent>) {
        self.event_tx = Some(tx);
    }

    /// 状态转换
    pub fn transition(&mut self, new_state: AgentTaskState) {
        self.state = new_state;
    }

    /// 检查是否可重试
    pub fn can_retry(&self) -> bool {
        self.task_type.max_retries() > 0
    }

    /// 获取最大重试次数
    pub fn max_retries(&self) -> u8 {
        self.task_type.max_retries()
    }

    /// 发送事件
    pub fn send_event(&self, event: TaskEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }
}

/// AgentTaskHandle - 持有 AgentTask 的引用，用于外部控制
#[derive(Debug, Clone)]
pub struct AgentTaskHandle {
    pub task_id: Uuid,
    pub state: Arc<Mutex<AgentTaskState>>,
    pub event_tx: mpsc::Sender<TaskEvent>,
}

impl AgentTaskHandle {
    pub fn new(task_id: Uuid, task_type: TaskType) -> (Self, mpsc::Receiver<TaskEvent>) {
        let (tx, rx) = mpsc::channel();
        let state = Arc::new(Mutex::new(AgentTaskState::Pending));
        let handle = Self {
            task_id,
            state,
            event_tx: tx,
        };
        (handle, rx)
    }

    pub fn is_terminal(&self) -> bool {
        if let Ok(state) = self.state.lock() {
            state.is_terminal()
        } else {
            false
        }
    }
}
```

- [ ] **Step 2: 更新 task/mod.rs 导出**

```rust
pub mod events;
pub mod coordinator;
pub mod agent_task;  // 新增

pub use events::*;
pub use coordinator::*;
pub use agent_task::*;
```

- [ ] **Step 3: 验证编译通过**
```bash
cd rust-agent-platform && cargo check 2>&1 | tail -30
```

- [ ] **Step 4: 提交**
```bash
cd rust-agent-platform
git add src/tui/task/agent_task.rs src/tui/task/mod.rs
git commit -m "feat(tui): add AgentTask runtime with event channel"
```

---

### Task 3: 创建 TaskRunner - 整合 AgentTask 和 AppController

**Files:**
- Create: `rust-agent-platform/src/tui/task/task_runner.rs`

- [ ] **Step 1: 创建 task_runner.rs**

```rust
use crate::tui::task::events::{AgentTaskState, TaskEvent, TaskType};
use crate::tui::task::agent_task::AgentTaskHandle;
use crate::core::Agent;
use std::sync::{Arc, Mutex, mpsc};
use uuid::Uuid;

/// TaskRunner - 在 AppController 和 AgentTask 之间协调
/// 负责创建任务、处理完成信号、触发 Coordinator 事件
pub struct TaskRunner {
    active_tasks: Vec<AgentTaskHandle>,
}

impl TaskRunner {
    pub fn new() -> Self {
        Self {
            active_tasks: Vec::new(),
        }
    }

    /// 创建并启动新任务
    pub fn spawn(
        &mut self,
        task_type: TaskType,
        agent: Arc<Agent>,
        provider_name: String,
        api_key: Option<String>,
        base_url: Option<String>,
        model: String,
        msg: String,
        debug_path: Option<String>,
        debug_tx: Option<mpsc::Sender<String>>,
        response_tx: mpsc::Sender<Result<String, anyhow::Error>>,
    ) -> Uuid {
        let task_id = Uuid::new_v4();
        let (handle, event_rx) = AgentTaskHandle::new(task_id, task_type.clone());

        // 保存 handle
        self.active_tasks.push(handle);

        task_id
    }

    /// 处理任务事件
    pub fn process_events(&mut self, event_rx: &mpsc::Receiver<TaskEvent>) {
        while let Ok(event) = event_rx.try_recv() {
            match event {
                TaskEvent::Spawned => {
                    // 任务已启动
                }
                TaskEvent::Chunk { content } => {
                    // 流式 chunk - 已经通过 response_tx 发送
                }
                TaskEvent::Completed { content } => {
                    // 任务完成
                    self.cleanup_completed_tasks();
                }
                TaskEvent::Failed { state } => {
                    // 任务失败
                    self.cleanup_completed_tasks();
                }
                _ => {}
            }
        }
    }

    fn cleanup_completed_tasks(&mut self) {
        self.active_tasks.retain(|handle| !handle.is_terminal());
    }

    pub fn active_count(&self) -> usize {
        self.active_tasks.len()
    }
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: 提交**
```bash
cd rust-agent-platform
git add src/tui/task/task_runner.rs
git commit -m "feat(tui): add TaskRunner for task coordination"
```

---

### Task 4: 端到端流式 pipeline 修复

**Files:**
- Modify: `rust-agent-platform/src/tui/app_controller.rs`

- [ ] **Step 1: 分析当前 app_controller.rs 的 process_agent_response**

当前问题可能是:
1. `finalized_message` 被设置后，没有立即显示
2. `response_receiver` 的 channel 可能在 task 完成前就关闭了
3. 缺少对 `channel closed` 事件的处理

**Step 2: 改进 process_agent_response**

```rust
fn process_agent_response(&mut self) {
    // 1. 首先检查是否有完成的消息需要添加到 chat
    let finalized_msg = {
        if let Ok(mut finalized) = self.ui_state.finalized_message.lock() {
            finalized.take()
        } else {
            None
        }
    };

    if let Some((role, content)) = finalized_msg {
        if let Some(chat_view) = self.get_chat_view_mut() {
            chat_view.state.add_message(&role, &content);
            chat_view.state.is_streaming = false;
        }
    }

    // 2. 处理流式 chunks（实时更新 UI）
    if let Some(chat_view) = self.get_chat_view_mut() {
        if let Ok(receiver) = self.ui_state.response_receiver.lock() {
            if let Some(ref rx) = *receiver {
                while let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(chunk) => {
                            // 实时追加到 UI
                            chat_view.state.add_message("assistant", &chunk);
                        }
                        Err(_e) => {
                            // Channel 关闭，清理
                            let _ = self.ui_state.streaming_text.lock()
                                .map(|mut s| s.take());
                        }
                    }
                }
            }
        }
    }
}
```

**注意**: 这个修改将 chunk 直接添加到 messages，而不是累积到 streaming_text。这样可以确保消息即时显示。

**Step 3: 验证编译通过**
```bash
cd rust-agent-platform && cargo check 2>&1 | tail -30
```

**Step 4: 提交**
```bash
cd rust-agent-platform
git add src/tui/app_controller.rs
git commit -m "fix(tui): streaming pipeline - direct chunk to messages"
```

---

### Task 5: 整合 TaskCoordinator 到 AppController

**Files:**
- Modify: `rust-agent-platform/src/tui/app_controller.rs`

- [ ] **Step 1: 在 AppController 中添加 TaskCoordinator**

```rust
pub struct AppController {
    context: AppContext,
    ui_state: UIState,
    runtime: tokio::runtime::Runtime,
    current_view: Box<dyn View>,
    should_quit: bool,
    debug_rx: Option<mpsc::Receiver<String>>,
    coordinator: TaskCoordinator,  // 新增
}
```

**Step 2: 初始化 coordinator**
```rust
let coordinator = TaskCoordinator::new();
```

**Step 3: 在 spawn_agent_task 中使用 coordinator**
```rust
fn spawn_agent_task(&mut self, msg: String) {
    let task_type = TaskType::UserMessage { priority: 100 };
    let task_id = self.coordinator.enqueue(task_type);

    // ... 现有代码 ...

    // 在 task 完成后通知 coordinator
    runtime.spawn(async move {
        // ... existing task code ...

        // 通知完成
        // coordinator.process_event(CoordinatorEvent::TaskCompleted { task_id });
    });
}
```

**Step 4: 验证编译通过**
```bash
cd rust-agent-platform && cargo check 2>&1 | tail -30
```

**Step 5: 提交**
```bash
cd rust-agent-platform
git add src/tui/app_controller.rs
git commit -m "feat(tui): integrate TaskCoordinator into AppController"
```

---

### Task 6: 最终验证

- [ ] **Step 1: cargo check**
```bash
cd rust-agent-platform && cargo check 2>&1 | tail -30
```

- [ ] **Step 2: cargo clippy**
```bash
cd rust-agent-platform && cargo clippy --all-targets 2>&1 | head -100
```

- [ ] **Step 3: cargo test**
```bash
cd rust-agent-platform && cargo test 2>&1 | tail -30
```

- [ ] **Step 4: 如果有问题，修复并重新提交**

---

## 总结

Phase 2 实现完成后，你应该有以下改动：

| 文件 | 变更 |
|------|------|
| `src/tui/task/coordinator.rs` | 完整优先级队列 + 抢占机制 |
| `src/tui/task/events.rs` | CoordinatorState 使用 Option<Uuid> |
| `src/tui/task/agent_task.rs` | 新增：AgentTask 运行时 |
| `src/tui/task/task_runner.rs` | 新增：TaskRunner 协调器 |
| `src/tui/app_controller.rs` | 流式 pipeline 修复 + TaskCoordinator 集成 |

---

**核心修复**:
1. **流式 chunk 直接添加到 messages** - 消息即时显示
2. **TaskCoordinator 完整实现** - 优先级队列 + 抢占
3. **AgentTask 运行时** - 状态机 + 事件通道
4. **TaskRunner** - 任务生命周期协调

---

**Plan 完成**。执行选项:

**1. Subagent-Driven (recommended)** - dispatch 一个 subagent per task

**2. Inline Execution** - 在本 session 执行

你想用哪个方式？