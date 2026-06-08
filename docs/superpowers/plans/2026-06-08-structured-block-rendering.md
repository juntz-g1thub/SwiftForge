# 结构化区块渲染实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 assistant 回复渲染为结构化区块：Reasoning（紫框）+ Tool Call（青框）+ Answer（纯文本）

**Architecture:** 三个环节：(1) 定义 `MessageBlock` 数据结构并替换 `Vec<(String, String)>`，(2) 将 reasoning content 从 agent 传递到 UI pipeline，(3) render_messages 调用已有的 `render_reasoning_block` / `render_tool_call_block`

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

- [ ] **Step 1: 在 view_state.rs 添加 ToolCallBlock、MessageBlock 结构体**
- [ ] **Step 2: 修改 ChatViewState.messages 类型** — `Vec<(String, String)>` → `Vec<MessageBlock>`
- [ ] **Step 3: 更新 add_message** — 适配新类型，新增 add_structured_message
- [ ] **Step 4: 导出新类型** — 确认 `mod.rs` 有导出
- [ ] **Step 5: 验证编译** — `cargo build`

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

- [ ] **Step 1: 定义 AgentResponse 结构体**
- [ ] **Step 2: 修改 `run_agent_loop` 签名和返回值** — 收集最后一次迭代的 reasoning
- [ ] **Step 3: 更新 `app_controller.rs` 中调用处** — 解构 AgentResponse
- [ ] **Step 4: 验证编译** — `cargo build`

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

- [ ] **Step 1: UIState 新增 finalized_reasoning 字段**
- [ ] **Step 2: spawn_agent_task 中解构 AgentResponse、写入 finalized_reasoning**
- [ ] **Step 3: process_agent_response 中读取 reasoning、调用 add_structured_message**
- [ ] **Step 4: 验证编译**

---

### Task 4: 渲染 Reasoning 区块

**Files:**
- Modify: `swiftforge/src/tui/views/chat_view.rs`

修改 `render_messages`：
```rust
fn render_messages(&mut self, f: &mut Frame, area: Rect, ui_state: &UIState) {
    let mut lines: Vec<Line> = Vec::new();
    let mut current_y = area.y();

    for msg in &self.state.messages {
        // Reasoning block
        if let Some(ref reasoning) = msg.reasoning {
            let block_height = reasoning.lines().count() as u16 + 2; // 2 for borders
            let block_area = Rect::new(area.x, current_y, area.width, block_height);
            self.render_reasoning_block(f, block_area, reasoning, false);
            current_y += block_height;
        }

        // Tool call blocks
        for tc in &msg.tool_calls {
            let block_height = 5; // fixed height for tool calls
            let block_area = Rect::new(area.x, current_y, area.width, block_height);
            // TODO: wire render_tool_call_block when tool calls are populated
            current_y += block_height;
        }

        // Content (answer) - no prefix for assistant
        let role_style = match msg.role.as_str() {
            "user" => Style::new().green().bold(),
            "assistant" => Style::new().cyan().bold(),  // still used for user; answer has no prefix
            "system" => Style::new().yellow().bold(),
            "error" => Style::new().red().bold(),
            _ => Style::new().white(),
        };

        if msg.role == "user" {
            let prefix = Self::format_prefix(&msg.role, &self.state.current_model);
            lines.push(Line::from(Span::styled(prefix, role_style)));
        }
        // assistant answer: no prefix, plain text
        lines.push(Line::from(Span::raw(msg.content.clone())));
    }

    // streaming section (unchanged)...
}
```

注意：当前 `render_messages` 把所有内容渲染到单一个 `Paragraph`（带滚动）。Reasoning 区块需要被渲染到独立的 `Rect` 区域（用 `f.render_widget`），这跟当前的 Paragraph 滚动机制冲突。要么：
- A. 把 Reasoning 区块也嵌入到 Paragraph 的纯文本中（模拟边框）
- B. 把整个消息区域改成按区块布局，放弃统一的 Paragraph

**建议选 A**（最小改动）：Reasoning 和 Tool Call 区块也用文本方式嵌入到 `lines` 中，带 ASCII 边框，而不是用 `f.render_widget` 渲染独立区块。这样滚动机制不变。

- [ ] **Step 1: 修改 render_messages 对 MessageBlock 类型做 match 渲染**
- [ ] **Step 2: user 消息保持 `[user]:` 前缀，assistant 的 content 无前缀**
- [ ] **Step 3: Reasoning 内容用文本区块嵌入** （不用独立的 f.render_widget）
- [ ] **Step 4: 删除旧的 `format!("{}: ", prefix)` 方式中对 assistant 的硬编码**（format_prefix 中已区分 user/其他）
- [ ] **Step 5: 验证编译**

---

### Task 5: 更新测试

**Files:**
- Modify: `swiftforge/tests/tui_state_test.rs`

- [ ] **Step 1: 修改 `test_chat_view_state_add_message`** — 适配 messages 索引由 `(String, String)` 改为 `MessageBlock`
- [ ] **Step 2: 修改 `test_streaming_pipeline_data_flow`** — 同上
- [ ] **Step 3: 修改 `test_chat_view_state_scroll_offset`** — 同上
- [ ] **Step 4: 运行测试** — `cargo test --test tui_state_test --test task_coordinator_test`

---

## 验证检查清单

- [ ] `[user]: hello` 格式不变
- [ ] assistant 回复中 reasoning 内容显示在紫色文本区块中
- [ ] assistant 回复中 answer 内容显示为纯文本（无前缀）
- [ ] streaming 状态保持 `[assistant model]: text▌` 不变
- [ ] `cargo build --bin swiftforge` 通过
- [ ] `cargo test --test tui_state_test` 通过
