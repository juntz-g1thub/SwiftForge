# TUI 消息显示架构

> 文档版本: 1.0
> 更新日期: 2026-06-09
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **已完成**

---

## 概述

本文档描述 SwiftForge TUI 消息显示模块的架构设计，包含两种渲染方案和实际实现状态。

**所属架构**: [平台架构与接口规范](../architecture/2026-05-23-platform-architecture.md)

---

## 一、视觉布局

### 1.1 实际渲染效果（方案A — 当前实现）

```
┌─────────────────────────────────────────────────────────────────┐
│ [user]: 如何统计当前目录下所有 .rs 文件的行数？                  │
│ [assistant deepseek-v4]: │
│ ┌─── Reasoning ✓ ─────────────────────────────────────────────┐ │
│ │Let me think about this step by step.                        │ │
│ │I need to find all .rs files and count their lines.          │ │
│ │Option 1: find + wc -l │ │
│ │Option 2: Rust recursive traversal                             │ │
│ └──────────────────────────────────────────────────────────────┘ │
│ 我来帮你统计。可以使用以下命令： │
│                                                                  │
│ 统计结果：共 15 个 .rs 文件，总计 4523 行代码。                   │
└─────────────────────────────────────────────────────────────────┘
```

**特点**：
- Label独立一行，在 Reasoning 框之前
- Reasoning 使用 ASCII box-drawing 字符（`┌─┐│└┘`）嵌入 Paragraph
- assistant 回答无前缀，纯文本输出
- user 消息显示为 `[user]:` 格式

### 1.2 方案B（未来升级选项）

```
┌─────────────────────────────────────────────────────────────────┐
│ [user]: 如何统计当前目录下所有 .rs 文件的行数？                  │
│                                                                  │
│  ┌─── Reasoning ✓ ─────────────────────────────────────────┐  │
│  │ 🌙 DeepSeek Reasoning [▼ 折叠] [⏸] │  │
│  │──────────────────────────────────────────────────────────│  │
│  │                                                           │  │
│  │ 分析用户需求：需要遍历目录、筛选 .rs 文件、统计行数       │  │
│  │                                                              │ │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│ 我来帮你统计。可以使用以下命令：                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**特点**：
- 每个区块独立 `Rect` + ratatui `Block` widget 渲染
- 支持折叠/展开交互
- 支持左边框流式动画
- 需要重写滚动机制（放弃统一 Paragraph）

---

## 二、数据结构

### 2.1 实际实现（view_state.rs）

```rust
// 区块类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Reasoning,
    ToolCall,
}

// 流式区块 — 可复用的 box-drawing 渲染单元
#[derive(Debug, Clone)]
pub struct StreamingBlock {
    pub block_type: BlockType,
    pub title: String,
    pub content: String,
    pub status: StreamingState,
    width: usize,
}

impl StreamingBlock {
    pub fn new(block_type: BlockType, title: &str, width: usize) -> Self;
    pub fn append(&mut self, text: &str);
    pub fn set_completed(&mut self);
    pub fn render(self) -> Vec<Line<'static>>; // 消耗 self，返回 owned Line
}

// 消息区块 — 存储在 ChatViewState.messages 中
#[derive(Debug, Clone)]
pub struct MessageBlock {
    pub role: String,                              // "user" | "assistant"
    pub reasoning: Option<String>,                 // 深度思考内容
    pub tool_calls: Vec<ToolCallBlock>,             // 工具调用
    pub content: String,                           // 最终回答
    pub status: StreamingState,                    // Idle | Streaming | Completed | Error
}

// 工具调用区块
#[derive(Debug, Clone)]
pub struct ToolCallBlock {
    pub name: String,                              // "bash", "read" 等
    pub arguments: String,                         // JSON 格式参数（字符串）
}

// 流式状态
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingState {
    Idle,
    Streaming,
    Completed,
    Error(String),
}

// Agent响应（core/agent.rs）
pub struct AgentResponse {
    pub content: String,
    pub reasoning: Option<String>,
}
```

**注意**：`ToolResultBlock` 不存在于当前实现中。

### 2.2 ChatViewState

```rust
pub struct ChatViewState {
    pub messages: Vec<MessageBlock>,               // 已完成的消息
    pub input: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub content_height: usize,
    pub scrollbar_state: ratatui::widgets::ScrollbarState,
    pub streaming_state: StreamingState,
    pub current_provider: String,
    pub current_model: String,
}
```

### 2.3 UIState

```rust
pub struct UIState {
    pub streaming_text: Arc<Mutex<Option<String>>>,
    pub response_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<String, anyhow::Error>>>>>,
    pub agent_command_tx: Arc<Mutex<Option<mpsc::Sender<AgentCommand>>>>,
    pub finalized_message: Arc<Mutex<Option<(String, String)>>>,  // (role, content)
    pub finalized_reasoning: Arc<Mutex<Option<String>>>,
}
```

---

## 三、方案A：文本嵌入方案（当前实现）

### 3.1 渲染流程

```
Agent::run_agent_loop() → AgentResponse { content, reasoning }
    ↓
spawn_agent_task: 写入 finalized_message + finalized_reasoning
    ↓
process_agent_response:
    chat_view.state.add_structured_message(role, content, reasoning, tool_calls)
    ↓
ChatView::render_messages():
    for msg in messages:
        label = format_prefix(role, model)     // [user]: / [assistant model]:
        lines.push(label)
        if msg.reasoning:
            block = StreamingBlock::new(BlockType::Reasoning, "Reasoning", area.width)
            block.append(reasoning)
            block.set_completed()
            lines.extend(block.render())      // ← box-drawing 嵌入 Paragraph
        lines.push(content)
    ↓
Paragraph::new(Text::from(lines))
    .wrap(Wrap { trim: true })
    ↓
ratatui 渲染
```

### 3.2 Box-drawing 算法

StreamingBlock 的 `render()` 方法生成符合宽度约束的 ASCII 边框：

```rust
let inner_width = self.width.saturating_sub(2);
let title_str = format!("{}{}", self.title, suffix);  // e.g. "Reasoning ✓"
let top_dash_count = self.width.saturating_sub(title_str.len() + 7);
let bottom_dash_count = inner_width.saturating_sub(2);

let top = format!("┌─── {} {}┐", title_str, "─".repeat(top_dash_count));
let bottom = format!("└{}┘", "─".repeat(bottom_dash_count));

let content_max = inner_width.saturating_sub(2);  // 内容最大宽度
for r_line in content_lines {
    let display_line = format!("│{:<width$}│", r_line, width = content_max);
    lines.push(Line::from(vec![Span::styled(display_line, style)]));
}
```

**宽度验证（width=60）**：

| 组件 | 公式 | 字符数 | 状态 |
|------|------|--------|------|
| TOP | `5 + title_len + 1 + dashes + 1` | 60 | ✅ |
| Content | `1 + content_max + 1` | 58 | ✅ |
| Bottom | `1 + dashes + 1` | 58 | ✅ |

### 3.3 优缺点

| 优点 | 缺点 |
|------|------|
| 实现简单，复用现有 Paragraph+Scrollbar | Reasoning 区块无法独立折叠/展开 |
| 流式友好（内容追加到 `Vec<Line>`） | 左边框无流式动画 |
| 渲染性能好 | 无法单独控制区块背景色/边框色 |
| 跨平台兼容性好（纯文本字符） | Tool Call 区块复用相同结构（待实现） |

---

## 四、方案B：独立区块方案（未来升级）

### 4.1 渲染流程

```
ChatView::render():
    for msg in messages:
        if msg.reasoning:
            reasoning_area = get_reasoning_area(area)  // 独立 Rect
            f.render_widget(render_reasoning_block, reasoning_area)
        if msg.tool_calls:
            tool_area = get_tool_area(area)
            f.render_widget(render_tool_call_block, tool_area)
        render_content(msg.content)
```

### 4.2 优缺点

| 优点 | 缺点 |
|------|------|
| 支持折叠/展开交互 | 需要重写滚动机制 |
| 左边框流式动画 | 实现复杂度高 |
| 每个区块独立背景色/边框色 | 区块高度计算复杂 |
| 更接近设计稿视觉效果 | 需要管理多 Rect 布局 |

### 4.3 升级路径

1. ChatViewState 新增 `streaming_blocks: Vec<StreamingBlock>` 字段存储流式进行中的区块
2. 重写 `render_messages` 支持独立 `Rect` 区块布局
3. 迁移到 `f.render_widget` 而非文本嵌入
4. 实现折叠/展开状态管理

---

## 五、数据流完整链路

```
User Input
    ↓
Action::SendMessage(msg)
    ↓
spawn_agent_task:
    runtime.spawn(async {
        final_agent.run_agent_loop(session, &msg, 5, Some(agent_tx)).await
    })
    ↓
agent_tx sends chunks → streaming_text accumulates
    ↓
Agent completes → AgentResponse { content, reasoning }
    ↓
spawn_agent_task writes:
    finalized_message = Some(("assistant", response.content))
    finalized_reasoning = response.reasoning
    ↓
process_agent_response:
    let (role, content) = finalized_msg.take()
    let reasoning = finalized_reasoning.take()
    chat_view.state.add_structured_message(role, content, reasoning, vec![])
    chat_view.state.streaming_state = StreamingState::Completed
    ↓
ChatView::render_messages:
    for msg in messages:
        lines.extend(msg.reasoning.map(|r| StreamingBlock::render(r)))
        lines.push(content)
```

---

## 六、相关文件

| 文件 | 描述 |
|------|------|
| `swiftforge/src/tui/state/view_state.rs` | StreamingBlock, MessageBlock, ChatViewState |
| `swiftforge/src/tui/state/app_context.rs` | UIState |
| `swiftforge/src/tui/views/chat_view.rs` | render_messages |
| `swiftforge/src/tui/app_controller.rs` | spawn_agent_task, process_agent_response |
| `swiftforge/src/core/agent.rs` | AgentResponse, run_agent_loop |
| `swiftforge/src/core/mod.rs` | AgentResponse re-export |

---

## 七、实现状态

### 已完成

| 功能 | 文件 | 状态 |
|------|------|------|
| MessageBlock 数据结构 | view_state.rs | ✅ |
| StreamingBlock::render() | view_state.rs | ✅ |
| AgentResponse 传递 reasoning | agent.rs | ✅ |
| finalized_reasoning pipeline | app_controller.rs | ✅ |
| render_messages 文本嵌入 | chat_view.rs | ✅ |
| Label 前缀格式（user 无 model） | chat_view.rs | ✅ |
| Box-drawing 算法修复 | view_state.rs | ✅ |

### 未实现

| 功能 | 状态 | 说明 |
|------|------|------|
| ToolResultBlock | ❌ | 不存在于代码中 |
| Tool Call 区块渲染 | ❌ | 预留 tool_calls 字段但未渲染 |
| 折叠/展开交互 | ❌ | 方案A 不支持 |
| 流式 Reasoning 实时更新 | ❌ | reasoning 在 agent 完成后一次性传入 |
| 左边框动画 | ❌ | 方案A 不支持 |
| 背景色/配色规范 | ❌ | 纯文本样式 |
| 移动端适配 | ❌ | - |

---

## 八、与设计文档的差异

### 8.1 原始设计文档

[`docs/specs/2026-05-25-tui-message-display-design.md`](../specs/2026-05-25-tui-message-display-design.md) 描述的方案B（独立区块渲染）：

- `render_reasoning_block` / `render_tool_call_block` 作为独立 `f.render_widget` 方法
- 每个区块有独立 `Rect`，通过 `current_y` 累进计算位置
- 支持折叠/展开、流式动画、emoji 图标

### 8.2 当前实现（方案A）

实际实现采用文本嵌入方案：

- `StreamingBlock::render()` 生成 `Vec<Line<'static>>`
- 所有内容统一放入 `Paragraph` + Scrollbar
- 不支持折叠/展开、动画、emoji

### 8.3 设计文档过时内容

| 文档位置 | 过时内容 | 实际情况 |
|------|------|------|
| §4.1 MessageBlock | `tool_results: Vec<ToolResultBlock>` | 不存在此字段 |
| §4.3 ToolResultBlock | 定义了 ToolResultBlock | 不存在于代码中 |
| §4.4 MessageStatus | `MessageStatus` 枚举 | 实际为 `StreamingState` |
| §5.2 渲染流程 | `render_reasoning_block(reasoning_area, msg)` | 死代码，从未被调用 |
| §9 实现状态 | render_reasoning_block ✅ | 死代码 |
| §9 实现状态 | render_tool_call_block ✅ | 死代码 |
| §9 实现状态 | 折叠交互 ✅ | 不存在 |
| §2.4 图标规范 | 🌙 🔧 ▼⏸ ✓✗ | 未实现 |

---

*文档状态: 已完成 — 反映 2026-06-09 实际实现状态*
