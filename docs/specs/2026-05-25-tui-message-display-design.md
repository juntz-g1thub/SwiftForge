# TUI 重构详细设计方案

> **⚠️ 已过时** — 本文描述的方案B（独立区块渲染）尚未实现
>
> **当前实现（方案A）**: 请参考 [TUI 消息显示架构文档](../architecture/2026-06-09-tui-message-display-architecture.md)

> 文档版本: 1.0
> 生成日期: 2026-05-25
> 更新日期: 2026-06-09
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **已过时 — 请参考新文档**

---

## 概述

**所属架构**: [平台架构与接口规范](../architecture/2026-05-23-platform-architecture.md)

**功能系统**: TUI (终端用户界面)

**设计目标**: 实现 MVC 架构的 TUI，支持深度思考、输出内容、工具调用的区分显示。

---

## 一、设计目标

### 1.1 分栏式显示

将 AI 响应分为三个明确的视觉区域：

1. **深度思考区域 (Reasoning)** - 显示模型的推理过程
2. **工具调用区域 (Tool Call)** - 显示工具执行
3. **回答区域 (Answer)** - 显示最终回答

### 1.2 交互功能

- 折叠/展开深度思考区域
- 流式输出状态指示
- 移动端适配

---

## 二、视觉设计

### 2.1 整体布局

```
┌─────────────────────────────────────────────────────────────────┐
│ [user]: 如何统计当前目录下所有 .rs 文件的行数？                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ 🌙 DeepSeek Reasoning                      [▼ 折叠] [⏸ 暂停] │ │
│ │ ─────────────────────────────────────────────────────────── │ │
│ │                                                              │ │
│ │ 分析用户需求：需要遍历目录、筛选 .rs 文件、统计行数          │ │
│ │ 方案1: find + wc                                             │ │
│ │ 方案2: 用 Rust 递归遍历                                       │ │
│ │ 决定使用 find 命令，简洁高效                                  │ │
│ │                                                              │ │
│ └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ 我来帮你统计。可以使用以下命令：                               │
│                                                                  │
│ ┌─ 🔧 Tool Call ───────────────────────────────────────────────┐ │
│ │                                                              │ │
│ │   bash: find . -name "*.rs" -exec wc -l {} +                  │ │
│ │                                                              │ │
│ │   ───────────────────────────────────────────────           │ │
│ │                                                              │ │
│ │   Result: 15 files, 4523 lines                               │ │
│ │                                                              │ │
│ └──────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ 统计结果：共 15 个 .rs 文件，总计 4523 行代码。                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 颜色规范

| 区域 | 背景色 | 边框色 | 标题色 | 文字色 |
|------|--------|--------|--------|--------|
| Reasoning | `#1e1e2e` (深紫灰) | `#7c3aed` (紫色) | `#a78bfa` (淡紫) | `#e2e8f0` (灰白) |
| Tool Call | `#0f172a` (深蓝灰) | `#22d3ee` (青色) | `#22d3ee` (青色) | `#e2e8f0` (灰白) |
| Answer | transparent | none | none | `#f8fafc` (白) |

### 2.3 字体规范

| 元素 | 字体 | 大小 | 行高 |
|------|------|------|------|
| 区域标题 | JetBrains Mono / monospace | 13px | 1.4 |
| Reasoning 内容 | JetBrains Mono / monospace | 12px | 1.5 |
| Tool 名 | JetBrains Mono / monospace | 13px | 1.4 |
| Tool 结果 | JetBrains Mono / monospace | 12px | 1.5 |
| Answer 正文 | 系统 sans-serif | 14px | 1.6 |

### 2.4 图标规范

| 类型 | 图标 | 说明 |
|------|------|------|
| Reasoning | 🌙 | 深度思考/月光 |
| Tool Call | 🔧 | 工具调用 |
| 折叠 | ▼ | 展开/折叠 |
| 暂停 | ⏸ | 暂停思考显示 |
| 完成 | ✓ | 思考完成 |
| 错误 | ✗ | 执行失败 |

---

## 三、交互规范

### 3.1 折叠/展开

- 点击标题区域或 [▼ 折叠] 按钮切换
- 折叠时只显示标题栏：`🌙 DeepSeek Reasoning (已折叠) - 点击展开`
- 记住用户的折叠偏好（持久化到配置）

### 3.2 流式输出状态

**思考中**:
- 左边框动画（流动的紫色）表示正在思考
- 标题显示：`🌙 DeepSeek Reasoning...`

**思考完成**:
- 左边框变为静态紫色
- 添加 ✓ 完成标记

**工具调用出现**:
- 从右侧滑入动画
- 高亮边框

**回答出现**:
- 淡入效果

### 3.3 移动端适配

| 屏幕宽度 | 布局调整 |
|----------|----------|
| > 768px | 完整三栏 |
| 480-768px | Reasoning 区域收紧，Tool Call 卡片简化 |
| < 480px | Reasoning 默认折叠，Tool Call 卡片全宽 |

---

## 四、数据结构

### 4.1 MessageBlock

```rust
pub struct MessageBlock {
    pub role: String,           // "user" | "assistant"
    pub reasoning: Option<String>,  // 深度思考内容
    pub tool_calls: Vec<ToolCallBlock>,  // 工具调用
    pub tool_results: Vec<ToolResultBlock>,  // 工具结果
    pub content: String,         // 最终回答
    pub status: MessageStatus,   // streaming / completed / error
}
```

### 4.2 ToolCallBlock

```rust
pub struct ToolCallBlock {
    pub name: String,           // "bash", "read", etc.
    pub arguments: String,      // JSON 格式的参数
}
```

### 4.3 ToolResultBlock

```rust
pub struct ToolResultBlock {
    pub tool_name: String,
    pub output: String,
    pub success: bool,
}
```

### 4.4 MessageStatus

```rust
pub enum MessageStatus {
    Streaming,      // 流式输出中
    Completed,      // 已完成
    Error(String),  // 错误信息
}
```

---

## 五、组件结构

### 5.1 ChatView

```rust
pub struct ChatView {
    pub state: ChatViewState,
}

impl View for ChatView {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState) {
        let messages_area = self.get_messages_area(area);
        self.render_messages(f, messages_area);
        let input_area = self.get_input_area(area);
        self.render_input(f, input_area);
        let status_area = self.get_status_area(area);
        self.render_status(f, status_area);
    }
}
```

### 5.2 渲染流程

```rust
fn render_message(&mut self, msg: &MessageBlock, area: Rect) {
    if msg.reasoning.is_some() {
        let reasoning_area = self.get_reasoning_area(area);
        self.render_reasoning_block(reasoning_area, msg);
    }
    if !msg.tool_calls.is_empty() {
        let tool_area = self.get_tool_area(area);
        self.render_tool_call_block(tool_area, msg);
    }
    for result in &msg.tool_results {
        self.render_tool_result(result);
    }
    if !msg.content.is_empty() {
        self.render_content(msg.content);
    }
}
```

---

## 六、重构任务清单

| 任务 | 描述 | 优先级 |
|------|------|--------|
| 1 | 扩展 ChatViewState 增加 MessageBlock 结构 | 高 |
| 2 | 实现 render_reasoning_block 方法 | 高 |
| 3 | 实现 render_tool_call_block 方法 | 高 |
| 4 | 实现折叠/展开交互 | 中 |
| 5 | 实现流式输出状态动画 | 中 |
| 6 | 移动端适配 | 中 |
| 7 | 删除旧的 `<thinking>`, `<content>`, `<tool_call>` 文本标签解析 | 高 |

---

## 七、与现有代码的对比

### 7.1 当前问题

1. **消息格式不区分内容类型** - 所有内容都是 `Vec<(String, String)>` 的元组
2. **解析文本标签** - 在 `stream_chat_with_tools` 中手动添加 XML 标签
3. **渲染逻辑混合** - 所有内容在同一个 Paragraph 中渲染

### 7.2 重构后优势

1. **类型安全** - MessageBlock 明确区分 reasoning、tool_calls、content
2. **独立渲染** - 每个区域可以独立渲染和更新
3. **可折叠** - 用户可以隐藏不想看的内容

---

## 八、实现状态

> 更新日期: 2026-06-09

### ⚠️ 已过时 — 请参考新文档

[TUI 消息显示架构文档](../architecture/2026-06-09-tui-message-display-architecture.md) 已合并本文内容并反映实际实现状态。

### 实际完成情况

| 任务 | 文件 | 状态 | 说明 |
|------|------|------|------|
| 数据结构定义 | `tui/state/view_state.rs` | ✅ | MessageBlock, ToolCallBlock, **StreamingBlock** |
| StreamingBlock::render() | `tui/state/view_state.rs` | ✅ | 方案A：文本嵌入 box-drawing |
| AgentResponse reasoning | `core/agent.rs` | ✅ | run_agent_loop 返回 AgentResponse |
| finalized_reasoning pipeline | `app_controller.rs` | ✅ | reasoning 从 agent 传递到 UI |
| render_messages 更新 | `tui/views/chat_view.rs` | ✅ | 调用 StreamingBlock::render() |
| 类型导出 | `tui/state/mod.rs` + `tui/mod.rs` | ✅ | MessageBlock, StreamingBlock, BlockType |

### 未实现（方案A 不支持）

| 任务 | 状态 | 说明 |
|------|------|------|
| render_reasoning_block Block widget | ❌ 死代码 | 存在但从未被调用 |
| render_tool_call_block Block widget | ❌ 死代码 | 存在但从未被调用 |
| ToolResultBlock | ❌ | 不存在于代码中 |
| 折叠/展开交互 | ❌ | 方案A 不支持 |
| 流式 Reasoning 实时更新 | ❌ | reasoning 在 agent 完成后一次性传入 |
| 左边框动画 | ❌ | 方案A 不支持 |
| emoji 图标 | ❌ | 未实现 |
| 背景色/配色规范 | ❌ | 纯文本样式 |
| 移动端适配 | ❌ | - |

---

*文档状态: 已过时 — 请参考 [TUI 消息显示架构](../architecture/2026-06-09-tui-message-display-architecture.md)*
