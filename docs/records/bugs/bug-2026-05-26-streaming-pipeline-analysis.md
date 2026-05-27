# Bug Report: TUI 重构 - 流式输出管道失效

> 生成日期: 2026-05-26
> 分支: feat/tui-refactor
> 状态: 已分析，待修复
> 优先级: P0

---

## 问题现象

用户输入消息后，按 Enter，模型输出不可见。Debug 模式行为待确认。

---

## 根因分析

### 问题定位

**文件**: `rust-agent-platform/src/tui/app_controller.rs:272-304`

```rust
match result {
    Ok(response) => {  // ← response 是 run_agent_loop 直接返回的完整字符串
        if let Ok(mut streaming) = streaming_text.lock() {
            if let Some(final_text) = streaming.take() {  // ← streaming_text 永远是 None！
                if !final_text.is_empty() {
                    if let Ok(mut finalized) = finalized_message.lock() {
                        *finalized = Some(("assistant".to_string(), final_text));
                    }
                }
            }
        }
    }
}
```

**根因**: `response` 变量（完整字符串）被完全忽略。`streaming_text.take()` 永远返回 `None`，因为 `streaming_text` 只通过 `response_receiver` 累积 chunks，从未被迁移到 `finalized_message`。

---

## 完整数据流分析

### 管道拓扑

```
spawn_agent_task(msg)
    │
    ├─ response_receiver = rx (line 201)
    ├─ clear_streaming() → streaming_text = None (line 203)
    │
    └─ [async task]
        └─ run_agent_loop(..., Some(tx))
            │
            ├─ on_chunk(chunk) → tx.send(Ok(chunk)) → rx
            │                         ↑
            │              process_agent_response 每帧调用
            │                         ↓
            │              rx.try_recv() → append_streaming(chunk)
            │                         ↓
            │              streaming_text 累积中（但从不显示）
            │
            └─ return Ok(response)  ← response 直接返回，不写 finalized_message
```

### 三条数据通路

| 通路 | 起点 | 终点 | 状态 |
|------|------|------|------|
| **A** | `response_receiver` | `streaming_text` | ✅ 累积中，但从不消费 |
| **B** | `run_agent_loop` 返回 | `finalized_message` | ❌ response 变量被忽略 |
| **C** | `ChatView.render_messages` | `ui_state.streaming_text` 读取 | ✅ 渲染层正常，但数据不通 |

### 问题点

1. **spawn_agent_task 结果处理**：只尝试从 `streaming_text` 取值，但 `streaming_text` 由通路A 填充，通路B 从未写入

2. **debug_tx 被覆盖**：`app_controller.rs:217` 创建新 channel 覆盖了 line 215 从 UIState 获取的 `debug_tx`，debug 消息无法到达 UI

3. **无 UI 更新信号**：`process_agent_response` 累积 `streaming_text` 但没有触发任何重绘信号

4. **process_agent_response 只做读取，不做迁移**：当 `rx` 关闭时（`Err`），没有逻辑将 `streaming_text` 迁移到 `finalized_message`

---

## 修复方向

### 方案A（最小改动）

在 `spawn_agent_task` 的 `Ok(response)` 分支直接用 `response` 设置 `finalized_message`：

```rust
Ok(response) => {
    if let Ok(mut finalized) = finalized_message.lock() {
        *finalized = Some(("assistant".to_string(), response));
    }
}
```

### 方案B（流式优先）

在 `process_agent_response` 中，当 `rx` 返回 `Err`（channel 关闭）时，将 `streaming_text` 迁移到 `finalized_message`

### 方案C（架构重构）

统一 streaming pipeline：
- 移除 `finalized_message`，所有输出统一通过 `streaming_text`
- 或统一通过 `response_receiver`，完全移除 `streaming_text`

---

## 相关文件

| 文件 | 问题 |
|------|------|
| `src/tui/app_controller.rs:272-304` | spawn_agent_task 结果处理忽略 response |
| `src/tui/app_controller.rs:217` | debug_tx 被覆盖 |
| `src/tui/app_controller.rs:350-384` | process_agent_response 只读取不迁移 |
| `src/tui/views/chat_view.rs:165-174` | render 正确但数据不通 |
| `src/core/agent.rs:378-383` | on_chunk 正确发送 chunks |

---

## 待验证事项

- [ ] `--debug` 模式下 debug panel 是否有输出（验证 run_agent_loop 是否执行）
- [ ] Provider 配置是否正确（验证 streaming 是否真的发生）
- [ ] `get_chat_view_mut()` 是否返回 `Some`（指针转换是否安全）

---

## 架构层面的问题

详见: `docs/records/bugs/bug-2026-05-26-streaming-pipeline-architecture.md`

---

*文档状态: 已分析，待修复验证*