# Bug: Debug 日志系统重构与消息不显示问题

> 日期: 2026-05-28
> 状态: 搁置
> 优先级: 高
> 关联: bug-2026-05-19-tui-messages-not-showing-analysis, bug-2026-05-26-streaming-pipeline-*

---

## 一、问题描述

### 1.1 主要问题

消息发送后 TUI 不显示 LLM 响应内容。请求一直挂起，用户在 7-35 秒后手动取消。

### 1.2 调试过程发现的问题

#### 问题 A：调试日志系统过于简陋

**现象**：
- 使用手动文件写入 (`if let Some(ref path) = debug_path { ... }`)
- 每个日志点重复编写文件操作代码
- 非 debug 模式无法方便地控制日志输出

**影响**：
- 无法精确定位问题出在哪一步
- 调试效率低

#### 问题 B：二进制文件与代码不一致

**现象**：
- 日志文件显示旧格式 `[HH:MM:SS.mmm] Action: SendMessage`
- 新代码使用 tracing 格式应该输出 `INFO rust_agent_platform::tui::app_controller - info!: handle_action`
- 日志文件中没有 `SPAWN:`、`CHUNK:` 等新添加的日志

**根因**：
- 用户运行的不是新编译的 binary
- 或 cargo build 没有正确编译新代码

#### 问题 C：streaming pipeline 复杂导致问题难定位

**数据流链路**：
```
spawn_agent_task()
    │
    ├─► runtime::spawn(async move { ... })
    │       │
    │       └─► run_agent_loop()
    │               │
    │               └─► chat_with_tools_streaming()
    │                       │
    │                       └─► Provider::stream_chat_with_tools()
    │                               │
    │                               └─► on_chunk(tx.send())
    │
    └─► process_agent_response()
            │
            ├─► finalized_message.take()
            │       └─► messages.push()
            │
            └─► rx.try_recv()
                    └─► messages.push()
```

**可能断裂点**：
1. `spawn_agent_task` 函数是否被调用
2. `runtime::spawn()` 是否成功
3. `run_agent_loop()` 是否开始执行
4. Provider 是否调用 `on_chunk`
5. `tx.send()` 是否成功
6. `rx.recv()` 是否成功
7. `messages.push()` 是否执行
8. `terminal.draw()` 是否渲染

---

## 二、已实施的修复

### 2.1 日志系统重构

**改动文件**: `main.rs`, `app_controller.rs`

**新日志系统**:
- 使用 `tracing` crate + `tracing-appender`
- 日志输出到 `~/.fastcode/ragent_*.log`
- debug 模式: TRACE 级别
- 非 debug 模式: INFO 级别

**关键日志点**:
```rust
trace!("loop: drawing")
trace!("loop: key event {:?}", key)
info!(action = ?action, "handle_action")
trace!(msg_len = msg.len(), "SPAWN: spawn_agent_task called")
debug!(provider = %provider_name, "SPAWN: task started")
trace!(debug_sender_is_some = debug_sender.is_some(), "SPAWN: before run_agent_loop")
debug!(chunk_len = chunk.len(), "CHUNK: received")
debug!(role = %role, content_len = content.len(), "FINALIZED: adding to messages")
trace!(chunk_count = streaming_chunks.len(), "CHUNKS: adding to messages")
```

### 2.2 Phase 1 修改

| 文件 | 变更 |
|------|------|
| `src/tui/task/events.rs` | 新增: TaskEvent, FailedState, TaskType, AgentTaskState |
| `src/tui/task/mod.rs` | 新增: task 模块 |
| `src/tui/task/coordinator.rs` | 新增: TaskCoordinator stub |
| `src/tui/state/view_state.rs` | 重写: ViewState, ChatContext, ConfigContext |
| `src/tui/app_controller.rs` | 修复: clear_streaming() 时机, debug_tx 覆盖 |
| `src/tui/views/chat_view.rs` | 移除: Debug panel |

### 2.3 Phase 2 修改

| 文件 | 变更 |
|------|------|
| `src/tui/task/coordinator.rs` | 完整优先级队列 + 抢占机制 |
| `src/tui/task/agent_task.rs` | 新增: AgentTask 运行时 |
| `src/tui/app_controller.rs` | 流式 pipeline 修复 (chunk 直接添加到 messages) |

---

## 三、日志格式对比

### 3.1 旧格式（当前日志文件）
```
[16:32:39.135] Action: SendMessage("hello")
[16:32:48.505] Action: CancelStreaming
```

### 3.2 新格式（期望）
```
2026-05-28T16:32:39.135Z INFO rust_agent_platform::tui::app_controller:12 - info!: handle_action action=SendMessage("hello")
2026-05-28T16:32:39.136Z TRACE rust_agent_platform::tui::app_controller:195 - trace!: SPAWN: spawn_agent_task called msg_len=5
2026-05-28T16:32:39.137Z DEBUG rust_agent_platform::tui::app_controller:211 - debug!: SPAWN: task started provider=minimax
2026-05-28T16:32:39.500Z DEBUG rust_agent_platform::tui::app_controller:368 - debug!: CHUNK: received chunk_len=2
...
```

---

## 四、待解决问题

### 4.1 消息不显示

**状态**: 未解决

**可能原因**:
1. Provider API 配置错误
2. 网络超时
3. Provider 不支持 streaming 模式
4. 代码逻辑在某个环节断裂

### 4.2 二进制文件与代码不一致

**状态**: 待确认

**需要**:
- 用户使用正确的 binary 运行程序
- 验证新日志格式是否生效

### 4.3 日志系统完整性

**状态**: 部分完成

**待完成**:
- agent.rs 中的日志仍使用旧的 `log` closure 模式
- Provider 层的日志需要补充

---

## 五、后续建议

### 5.1 立即步骤

1. **确认 binary 正确运行**
   ```bash
   cd rust-agent-platform
   cargo build --bin ragent --force-auth-sudo
   ./target/debug/ragent --debug
   ```

2. **检查新日志格式**
   - 查看 `~/.fastcode/ragent_*.log` 是否包含 tracing 格式
   - 确认 `SPAWN:`、`CHUNK:` 日志是否出现

### 5.2 深入调试方向

1. **简化 streaming pipeline**
   - 考虑移除中间状态（`streaming_text`、`finalized_message`）
   - 让数据直接写入最终位置（`messages`）

2. **添加端到端测试**
   - 验证 "用户输入" → "messages 显示" 的完整路径
   - Mock Provider 返回测试数据

3. **Provider 层日志**
   - 在 `stream_chat_with_tools` 中添加日志
   - 确认 SSE 数据是否正确接收

---

## 六、相关文档

- `docs/specs/2026-05-27-tui-state-machine-design.md` - 状态机设计规范
- `docs/specs/2026-05-27-tui-state-machine-plan-phase1.md` - Phase 1 实现计划
- `docs/specs/2026-05-27-tui-state-machine-plan-phase2.md` - Phase 2 实现计划
- `docs/records/bugs/bug-2026-05-19-tui-messages-not-showing-analysis.md` - 早期分析
- `docs/records/bugs/bug-2026-05-26-streaming-pipeline-architecture.md` - Pipeline 分析

---

*最后更新: 2026-05-28 22:20*
*状态: 问题搁置，等待二进制文件验证*