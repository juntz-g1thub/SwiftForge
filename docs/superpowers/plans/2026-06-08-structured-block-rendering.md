# 结构化区块渲染实现计划

> **状态**: ✅ **已实现**（方案A — 文本嵌入）
>
> 完整架构文档: [TUI 消息显示架构](../../architecture/2026-06-09-tui-message-display-architecture.md)

> 相关设计文档: [TUI 重构详细设计方案（旧）](../../specs/2026-05-25-tui-message-display-design.md)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 assistant 回复渲染为结构化区块：Reasoning（紫框）+ Tool Call（青框）+ Answer（纯文本）

**实际实现**: 采用方案A — `StreamingBlock::render()` 文本嵌入 Paragraph，而非原设计的独立 `f.render_widget` 方案B。reasoning区块使用ASCII box-drawing字符（`┌─┐│└┘`），Tool Call区块同理可复用。

**Tech Stack:** Rust, ratatui

---

## 数据流变化

```
当前 (扁平):
  agent.run_agent_loop() → String(content)
    → finalized_message: (role, content)
    → messages: Vec<(String, String)>
    → 渲染: "[assistant model]: content"

目标 (结构化):
  agent.run_agent_loop() → (content, reasoning, tool_calls)
    → finalized_message: (role, content) + finalized_reasoning + finalized_tool_calls
    → messages: Vec<MessageBlock>  ← role, reasoning, tool_calls, content, status
    → 渲染: Reasoning 框 | Tool Call 框 | Answer(纯文本)
```

---

## 相关文件

| 文件 | 改动 |
|------|------|
| `swiftforge/src/tui/state/view_state.rs` | 新增 MessageBlock 等结构体，改 messages 类型 |
| `swiftforge/src/core/agent.rs` | `run_agent_loop` 返回类型扩展为结构化数据 |
| `swiftforge/src/tui/app_controller.rs` | pipeline 传递 reasoning + tool_calls |
| `swiftforge/src/tui/app_controller.rs` | UIState 新增 finalized_reasoning / finalized_tool_calls |
| `swiftforge/src/tui/views/chat_view.rs` | render_messages 调用 render_reasoning_block / render_tool_call_block |
| `swiftforge/tests/tui_state_test.rs` | 适配 MessageBlock 新类型 |

---

### Task 1: 定义数据结构 + 升级 messages 类型

**Files:**
- Modify: `swiftforge/src/tui/state/view_state.rs`

```rust
// 新增（在 StreamingState 之后, ChatViewState 之前）
#[derive(Debug, Clone)]
pub struct ToolCallBlock {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct MessageBlock {
    pub role: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallBlock>,
    pub content: String,
    pub status: StreamingState,
}
```

```rust
// 修改 ChatViewState.messages: Vec<(String, String)> → Vec<MessageBlock>

// 修改 ChatViewState.add_message:
// 简单版 - 只填 role + content
pub fn add_message(&mut self, role: &str, content: &str) {
    self.messages.push(MessageBlock {
        role: role.to_string(),
        reasoning: None,
        tool_calls: Vec::new(),
        content: content.to_string(),
        status: StreamingState::Completed,
    });
}

// 新增 - 结构化版
pub fn add_structured_message(&mut self, role: &str, content: &str, reasoning: Option<String>, tool_calls: Vec<ToolCallBlock>) {
    self.messages.push(MessageBlock {
        role: role.to_string(),
        reasoning,
        tool_calls,
        content: content.to_string(),
        status: StreamingState::Completed,
    });
}
```

- [x] **Step 1: 在 view_state.rs 添加 ToolCallBlock、MessageBlock 结构体**
- [x] **Step 2: 修改 ChatViewState.messages 类型** — `Vec<(String, String)>` → `Vec<MessageBlock>`
- [x] **Step 3: 更新 add_message** — 适配新类型，新增 add_structured_message
- [x] **Step 4: 导出新类型** — 确认 `mod.rs` 有导出
- [x] **Step 5: 验证编译** — `cargo build`

---

### Task 2: Agent 返回结构化数据

**Files:**
- Modify: `swiftforge/src/core/agent.rs`

`run_agent_loop` 当前返回 `Result<String>`（只有 content）。改成返回包含 reasoning 的结构体：

```rust
// 新增（在 impl Agent 外部）
pub struct AgentResponse {
    pub content: String,
    pub reasoning: Option<String>,
}

// 修改 run_agent_loop 返回值
pub async fn run_agent_loop(...) -> Result<AgentResponse> {
    // ... 已有逻辑 ...
    // response.content, reasoning 在 loop 中已捕获
    // 需要在最后一个 chat_with_tools_streaming 调用后获取 reasoning

    // 在函数末尾:
    let final_reasoning = /* 从最后一次调用的 reasoning_content 获取 */;
    Ok(AgentResponse {
        content: full_response,
        reasoning: final_reasoning,
    })
}
```

注意：`run_agent_loop` 内部在每次迭代中调用 `chat_with_tools_streaming`，该方法返回的 `ModelResponse` 已包含 `reasoning_content`。关键是在最后一次迭代（无 tool_calls 时）保存 reasoning。

- [x] **Step 1: 定义 AgentResponse 结构体**
- [x] **Step 2: 修改 `run_agent_loop` 签名和返回值** — 收集最后一次迭代的 reasoning
- [x] **Step 3: 更新 `app_controller.rs` 中调用处** — 解构 AgentResponse
- [x] **Step 4: 验证编译** — `cargo build`

---

### Task 3: Pipeline 传递 reasoning 到 UI

**Files:**
- Modify: `swiftforge/src/tui/app_controller.rs`

当前 `finalized_message: Arc<Mutex<Option<(String, String)>>>` 存 `(role, content)`。
新增 `finalized_reasoning: Arc<Mutex<Option<String>>>`。

在 `spawn_agent_task` 中：
```rust
// 在 result handler 中:
Ok(response) => {
    if let Ok(mut finalized) = finalized_message.lock() {
        *finalized = Some(("assistant".to_string(), response.content));
    }
    if let Some(reasoning) = response.reasoning {
        if let Ok(mut fr) = finalized_reasoning.lock() {
            *fr = Some(reasoning);
        }
    }
}
```

在 `process_agent_response` 中：
```rust
// 读取 reasoning
let finalized_reasoning = self.ui_state.finalized_reasoning.lock()
    .ok()
    .and_then(|mut r| r.take());

// 创建 MessageBlock
chat_view.state.add_structured_message(
    "assistant", &content,
    finalized_reasoning,
    vec![], // tool_calls 暂留空
);
```

UIState 新增字段：
```rust
pub finalized_reasoning: Arc<Mutex<Option<String>>>,
```

- [x] **Step 1: UIState 新增 finalized_reasoning 字段**
- [x] **Step 2: spawn_agent_task 中解构 AgentResponse、写入 finalized_reasoning**
- [x] **Step 3: process_agent_response 中读取 reasoning、调用 add_structured_message**
- [x] **Step 4: 验证编译**

---

### Task 4: 渲染 Reasoning 区块

**Files:**
- Modify: `swiftforge/src/tui/views/chat_view.rs`

修改 `render_messages`：
```rust
fn render_messages(&mut self, f: &mut Frame, area: Rect, ui_state: &UIState) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &self.state.messages {
        let label_style = match msg.role.as_str() {
            "user" => Style::new().green().bold(),
            "assistant" => Style::new().cyan().bold(),
            "system" => Style::new().yellow().bold(),
            "error" => Style::new().red().bold(),
            _ => Style::new().cyan().bold(),
        };
        let label = Self::format_prefix(&msg.role, &self.state.current_model);
        lines.push(Line::from(Span::styled(label, label_style)));

        if let Some(ref reasoning) = msg.reasoning {
            let mut b = StreamingBlock::new(BlockType::Reasoning, "Reasoning", area.width as usize);
            b.append(reasoning);
            b.set_completed();
            lines.extend(b.render());
        }

        lines.push(Line::from(Span::raw(msg.content.clone())));
    }

    // streaming section (unchanged)...
}
```

**已实现方案A**： Reasoning 区块通过 `StreamingBlock::render()` 生成文本嵌入 `Paragraph`，使用 box-drawing 字符。

- [x] **Step 1: 修改 render_messages 对 MessageBlock 类型做 match 渲染**
- [x] **Step 2: user 消息保持 `[user]:` 前缀，assistant 的 content 无前缀**
- [x] **Step 3: Reasoning 内容用文本区块嵌入** — 通过 `StreamingBlock::render()`
- [x] **Step 4: 删除旧的 `format!("{}: ", prefix)` 方式中对 assistant 的硬编码**（format_prefix 中已区分 user/其他）
- [x] **Step 5: 验证编译**

---

### Task 5: 更新测试

**Files:**
- Modify: `swiftforge/tests/tui_state_test.rs`

- [x] **Step 1: 修改 `test_chat_view_state_add_message`** — 适配 messages 索引由 `(String, String)` 改为 `MessageBlock`
- [x] **Step 2: 修改 `test_streaming_pipeline_data_flow`** — 同上
- [x] **Step 3: 修改 `test_chat_view_state_scroll_offset`** — 同上
- [x] **Step 4: 运行测试** — `cargo test --test tui_state_test --test task_coordinator_test`

---

## 验证检查清单

- [x] `[user]: hello` 格式不变
- [x] assistant 回复中 reasoning 内容显示在紫色文本区块中
- [x] assistant 回复中 answer 内容显示为纯文本（无前缀）
- [x] streaming 状态保持 `[assistant model]: text▌` 不变
- [x] `cargo build --bin swiftforge` 通过
- [x] `cargo test --test tui_state_test` 通过
