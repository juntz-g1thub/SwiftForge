# swiftforge Task Module

TUI 任务调度模块，实现四层状态机架构。

## 文档

| 文档 | 说明 |
|------|------|
| `docs/2026-05-25-tui-message-display-design.md` | TUI 消息显示设计（Reasoning/ToolCall/Answer 分栏） |
| `docs/2026-05-27-tui-state-machine-design.md` | TUI 状态机架构设计 |
| `docs/2026-05-27-tui-state-machine-plan-phase1.md` | Phase 1 实现计划 ✅ 已完成 |
| `docs/2026-05-27-tui-state-machine-plan-phase2.md` | Phase 2 实现计划 ⏳ 待完成 |

## 架构

```
AppController → TaskCoordinator → AgentTask → ViewState
     │              │                │           │
     └── 事件驱动 ───┴── Channel ────┘           │
                                              └── UI 渲染
```

## 核心组件

- **TaskCoordinator**：管理任务队列和调度，支持优先级和抢占
- **AgentTask**：单个任务的生命周期管理
- **events**：事件定义（TaskEvent, CoordinatorEvent, AgentTaskState）

## 状态

| 状态 | 说明 |
|------|------|
| `Pending` | 等待执行 |
| `Running` | 执行中 |
| `Completed` | 完成 |
| `Failed` | 失败 |
| `Suspended` | 挂起 |
| `Cancelled` | 取消 |
