# TUI 重构 Bug 报告

> 生成日期: 2026-05-19
> 分支: feature/tui-refactor
> 状态: 待修复

---

## 概述

重构后的 TUI 存在两个主要问题：
1. **非debug模式下输入消息后界面没有任何反应**
2. **debug模式下debug窗口不显示**

本文档详细分析问题根因，并与旧实现进行逐项对比。

---

## 问题1: 消息不显示

### 现象
输入消息并按Enter后，消息没有出现在聊天区域（只有流式传输时的临时显示）。

### 根因分析

**旧实现流程** (app.rs 555-581):
```rust
// 主循环中处理 response_receiver
if let Some(ref receiver) = self.response_receiver {
    match receiver.try_recv() {
        Ok(Ok(chunk)) => {
            // 1. 追加到 streaming_text
            if let Some(ref mut streaming) = self.streaming_text {
                streaming.push_str(&chunk);
            } else {
                self.streaming_text = Some(chunk);
            }
        }
        Ok(Err(e)) => {
            // 2. 错误情况 - 添加到 messages 并显示
            let partial = self.streaming_text.clone().unwrap_or_default();
            self.messages.push(("error".to_string(), format!("{} (partial: {})", e, partial)));
            self.streaming_text = None;
            self.response_receiver = None;
        }
        Err(TryRecvError::Disconnected) => {
            // 3. 通道关闭 - 最终响应完成
            if let Some(final_text) = self.streaming_text.take() {
                if !final_text.is_empty() {
                    // 关键: 把最终响应添加到 messages！
                    self.messages.push(("assistant".to_string(), final_text));
                }
            }
            self.response_receiver = None;
        }
        Err(TryRecvError::Empty) => {}
    }
}
```

**新实现问题** (app_controller.rs 326-343):
```rust
fn process_agent_response(&self) {
    if let Ok(receiver) = self.ui_state.response_receiver.lock() {
        if let Some(ref rx) = *receiver {
            while let Ok(result) = rx.try_recv() {
                match result {
                    Ok(chunk) => {
                        self.ui_state.append_streaming(&chunk);
                    }
                    Err(_e) => {
                        if let Ok(mut streaming) = self.ui_state.streaming_text.lock() {
                            let _ = streaming.take();
                        }
                        // 问题: 没有处理 Disconnected 情况！
                    }
                }
            }
        }
    }
}
```

**缺失的关键逻辑**:
1. 没有处理 `TryRecvError::Disconnected` - 即 Agent 完成时没有把 `streaming_text` 添加到 `ChatView.state.messages`
2. 异步任务完成后 (`spawn_agent_task`) 也没有通知 ChatView 将 streaming 转为最终消息

---

## 问题2: Debug窗口不显示

### 现象
使用 `--debug` 启动时，debug面板完全不显示。

### 根因分析

**旧实现布局** (app.rs 669-680):
```rust
fn render_chat(&mut self, f: &mut Frame) {
    // debug_height 根据 show_debug 动态计算
    let debug_height = if self.show_debug { 8 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),              // 0: 消息区域
            Constraint::Length(3),            // 1: 输入框
            Constraint::Length(1),            // 2: 状态栏
            Constraint::Length(debug_height), // 3: Debug面板（动态高度）
        ].as_ref())
        .split(f.size());

    // ...渲染消息、输入框、状态栏...

    // 如果 show_debug 为 true，渲染 debug 面板
    if self.show_debug {
        let debug_lines: Vec<Line> = self.debug_messages.iter()...
        let debug_para = Paragraph::new(Text::from(scrollable_debug_lines))
            .block(Block::default().borders(Borders::ALL).title("Debug Log (↑↓ scroll)"))
            .style(Style::new().red());
        f.render_widget(debug_para, chunks[3]);
    }
}
```

**新实现布局** (chat_view.rs 173-187):
```rust
fn render(&mut self, f: &mut Frame, area: Rect, _ctx: &AppContext, ui_state: &UIState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),      // 0: 消息区域
            Constraint::Length(3),   // 1: 输入框
            Constraint::Length(1),    // 2: 状态栏
            // 问题: 根本没有第四个区域！
        ])
        .split(area);

    self.render_messages(f, chunks[0], ui_state);
    self.render_input(f, chunks[1]);
    self.render_status(f, chunks[2]);
}
```

**缺失内容**:
1. ChatView 不知道 `show_debug` 状态（没有这个字段）
2. 没有第四个区域来显示 debug 面板
3. debug_messages 从 UIState 获取但从未渲染

---

## 问题3: Debug消息传递缺失

### 根因分析

**旧实现** (app.rs 1034):
```rust
// send_to_provider 函数中
match agent.run_agent_loop(
    &user_input,
    5,
    debug_log_path.clone().map(|p| p.to_string_lossy().to_string()),
    debug_tx.clone(),  // <-- 传递 debug_tx 通道
    Some(tx.clone())
).await
```

**新实现** (app_controller.rs 249-255):
```rust
let result = final_agent.run_agent_loop(
    &msg,
    5,
    debug_path.map(|p| p.to_string_lossy().to_string()),
    None,  // <-- 问题: 传递的是 None，没有 debug_tx！
    Some(tx),
).await;
```

**影响**: Agent 无法通过 channel 发送 debug 消息，只能写入文件。

---

## 问题汇总表

| # | 问题 | 严重度 | 旧代码位置 | 新代码位置 |
|---|------|--------|-----------|-----------|
| 1 | 消息不显示 - Disconnected未处理 | 高 | app.rs 572-578 | app_controller.rs 326-343 |
| 2 | 消息不显示 - 最终响应未添加到messages | 高 | app.rs 573-576 | app_controller.rs 257-267 |
| 3 | Debug窗口不显示 - 缺少第四区域 | 高 | app.rs 669-680 | chat_view.rs 173-187 |
| 4 | Debug窗口不显示 - 缺少show_debug状态 | 高 | app.rs 408 | chat_view.rs |
| 5 | Debug消息未传递 - debug_tx为None | 中 | app.rs 1034 | app_controller.rs 253 |
| 6 | debug_messages未渲染 | 中 | app.rs 769-798 | chat_view.rs |

---

## 修复方案

### 方案概述

1. **修复消息显示**: 在 `spawn_agent_task` 完成后，将最终响应添加到 `ChatView.state.messages`
2. **添加Debug面板**: 修改 `ChatView.render()` 添加第四区域，显示 `ui_state.debug_messages`
3. **传递Debug通道**: 将 `debug_tx` 传递给 `run_agent_loop`

---

### 修复1: process_agent_response 完善 + 消息最终化

需要修改 `app_controller.rs`:

1. 添加完成通知机制（因为异步任务无法直接修改UI状态）
2. 在 `spawn_agent_task` 的 Ok/Err 分支中设置完成标记
3. 在 `process_agent_response` 中检测完成并更新 messages

**最佳方案**: 使用一个单独的 `channel` 来通知 ChatView 更新消息。

```rust
// 在 UIState 中添加
pub struct UIState {
    // ...现有字段...
    pub message_finalized_tx: Arc<Mutex<Option<mpsc::Sender<(String, String)>>>>,
}

// spawn_agent_task 完成后
message_finalized_tx.send(("assistant".to_string(), final_text));

// process_agent_response 检查
if let Ok(msg) = finalized_rx.try_recv() {
    chat_view.state.add_message(&msg.0, &msg.1);
    chat_view.state.is_streaming = false;
}
```

### 修复2: ChatView 添加 Debug 面板

修改 `chat_view.rs`:

```rust
fn render(&mut self, f: &mut Frame, area: Rect, ctx: &AppContext, ui_state: &UIState) {
    // 检查是否显示 debug
    let show_debug = ctx.debug_log_path.is_some();
    let debug_height = if show_debug { 8 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),              // 消息
            Constraint::Length(3),             // 输入
            Constraint::Length(1),              // 状态
            Constraint::Length(debug_height),   // Debug
        ])
        .split(area);

    // ...现有渲染...

    // Debug 面板
    if show_debug {
        self.render_debug_panel(f, chunks[3], ui_state);
    }
}
```

### 修复3: 传递 Debug 通道

修改 `app_controller.rs` 的 `spawn_agent_task`:

```rust
// 创建 debug_tx 通道
let (debug_tx, debug_rx) = mpsc::channel();

// 传递给 run_agent_loop
let result = final_agent.run_agent_loop(
    &msg,
    5,
    debug_path.map(|p| p.to_string_lossy().to_string()),
    Some(debug_tx),  // 改为 Some(debug_tx)
    Some(tx),
).await;

// 在主循环中处理 debug_rx
```

---

## 修复计划

### 步骤1: 修改 state/app_context.rs
- 添加 `message_finalized_tx` 字段

### 步骤2: 修改 tui/mod.rs
- 更新导出

### 步骤3: 修改 app_controller.rs
- 在 `spawn_agent_task` 完成时发送最终消息
- 在 `run()` 主循环中添加 `debug_rx` 处理
- 传递 `debug_tx` 给 `run_agent_loop`

### 步骤4: 修改 views/chat_view.rs
- 添加 `show_debug` 检测
- 添加 Debug 面板渲染
- 处理完成的流式消息

---

## 验证清单

修复后需要验证:

- [ ] 输入消息 → 消息出现在聊天历史（非流式区域）
- [ ] 消息响应完成后流式区域清空
- [ ] `--debug` 模式下 Debug 面板显示
- [ ] Debug 日志实时更新
- [ ] ↑↓ 键可以滚动 Debug 日志

---

*文档状态: 待修复*