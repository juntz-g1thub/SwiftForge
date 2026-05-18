# DeepSeek 流式返回处理规范

## 1. 概述

本文档定义 DeepSeek API 流式返回（SSE）的处理规范，包括：
- JSON delta 字段类型
- 状态机转换规则
- 标签格式标准
- 前端渲染要求

## 2. DeepSeek SSE 响应格式

### 2.1 标准格式

```json
data: {"choices":[{"delta":{"role":"assistant"}}]}
data: {"choices":[{"delta":{"reasoning_content":"..."}}]}
data: {"choices":[{"delta":{"content":"..."}}]}
data: {"choices":[{"delta":{"tool_calls":[{"function":{"name":"bash","arguments":"{}"}}]}}]}
data: [DONE]
```

### 2.2 delta 字段类型

| 字段 | 类型 | 说明 |
|------|------|------|
| `role` | string | `"assistant"`，仅首帧出现 |
| `reasoning_content` | string | 思考链内容，DeepSeek 特有 |
| `content` | string | 正文回复 |
| `tool_calls` | array | 工具调用数组 |

## 3. 状态机定义

### 3.1 状态枚举

```rust
enum StreamState {
    Idle,       // 初始状态
    Thinking,   // 思考中（reasoning_content）
    Content,    // 正文阶段
    ToolCall,   // 工具调用阶段
}
```

### 3.2 状态转换图

```
                                    ┌─────────────────┐
                                    │     IDLE        │
                                    │   (初始状态)     │
                                    └────────┬────────┘
                                             │
                    reasoning_content 到达    │
                                             ▼
                                    ┌─────────────────┐
                           ┌───────│   THINKING      │◄──────────┐
                           │       │   (思考中)       │           │
                           │       └───────┬─────────┘           │
                           │               │                     │
                           │               │ content 到达          │ tool_calls 到达
                           │               ▼                     │
                           │       ┌─────────────────┐           │
                           │       │    CONTENT      │           │
                           │       │    (正文)       │           │
                           │       └─────────────────┘           │
                           │               │                     │
                           └───────────────┼─────────────────────┘
                                           │
                          tool_calls 到达  │
                                           ▼
                                  ┌─────────────────┐
                                  │   TOOL_CALL    │
                                  │   (工具调用)    │
                                  └────────┬────────┘
                                           │
                                           │ 执行完毕继续
                                           ▼
```

## 4. 标签格式标准

### 4.1 标签定义

| 标签 | 开始 | 结束 | 说明 |
|------|------|------|------|
| 思考链 | `<thinking>` | `\n</thinking>` | DeepSeek 特有 |
| 正文 | `<content>` | `\n</content>` | 普通回复内容 |
| 工具调用 | `<tool>` | `\n</tool>` | 仅包含工具名 |

### 4.2 标签输出格式

```
<thinking>reasoning_content 原文\n</thinking>
<content>正文内容\n</content>
<tool>工具名\n</tool>
```

## 5. 处理规则

### 5.1 优先级规则

**tool_calls > content > reasoning_content**

当同时存在多个字段时，按优先级处理：
1. tool_calls 最优先（可能打断思考）
2. content 次之（可能打断思考）
3. reasoning_content 最后（可能被其他字段打断）

### 5.2 状态转换规则

| 当前状态 | 收到字段 | 操作 |
|----------|----------|------|
| Idle/Content | reasoning_content | 输出 `<thinking>`，进入 Thinking |
| Thinking | content | 输出 `</thinking>`，进入 Content |
| Thinking | tool_calls | 输出 `</thinking>` + `<tool>`，进入 ToolCall |
| Content | tool_calls | 输出 `<tool>`，进入 ToolCall |
| Content | reasoning_content | 输出 `</thinking>` + `<thinking>`，进入 Thinking |
| ToolCall | content | 输出 `</tool>` + content，进入 Content |
| ToolCall | reasoning_content | 输出 `</tool>` + `</thinking>` + `<thinking>` |

### 5.3 空值过滤

- `reasoning_content` 为空时不输出
- `content` 为空时不输出
- `tool_calls` 中 `name` 为空时不输出该工具

### 5.4 工具调用结构

```json
{
  "tool_calls": [{
    "function": {
      "name": "bash",
      "arguments": "{\"command\":\"ls -la\"}"
    }
  }]
}
```

## 6. 前端渲染规范

### 6.1 渲染组件要求

| 标签 | 样式 | 行为 |
|------|------|------|
| `<thinking>` | 灰色/暗色，斜体 | 显示标签，继续累积内容 |
| `</thinking>` | - | 结束当前行，准备新行 |
| `<tool>name</tool>` | 青色/蓝色，加粗 | 单独一行显示 |
| `</tool>` | - | 结束工具行 |

### 6.2 渲染流程

```
收到 <thinking>
    ↓
继续累积，直到收到 </thinking>
    ↓
收到 <tool>
    ↓
继续累积，直到收到 </tool>
    ↓
正常渲染正文内容
```

### 6.3 换行保证

所有标签必须确保前后有换行符：
- `<thinking>` 后必须有 `\n`
- `</thinking>` 前后必须有 `\n`
- `<tool>name</tool>` 前后必须有 `\n`

## 7. 实现示例

### 7.1 Provider 层（deepseek.rs）

```rust
let mut is_thinking = false;

if let Some(reasoning) = delta["reasoning_content"].as_str() {
    if !reasoning.is_empty() {
        if !is_thinking {
            on_chunk("<thinking>\n".to_string());
            is_thinking = true;
        }
        on_chunk(reasoning.to_string());
    }
}

if let Some(content) = delta["content"].as_str() {
    if !content.is_empty() {
        if is_thinking {
            on_chunk("\n</thinking>\n".to_string());
            is_thinking = false;
        }
        on_chunk(content.to_string());
    }
}

if let Some(tool_calls) = delta["tool_calls"].as_array() {
    for tool_call in tool_calls {
        if let Some(func) = tool_call.get("function") {
            let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if !name.is_empty() {
                if is_thinking {
                    on_chunk("\n</thinking>\n".to_string());
                    is_thinking = false;
                }
                on_chunk(format!("<tool>{}</tool>\n", name));
            }
        }
    }
}
```

### 7.2 Agent 层累积

```rust
let on_chunk_wrapper = Box::new(move |chunk: String| {
    // 提取 tool_calls JSON 用于执行
    if chunk.starts_with("<tool>") {
        if let Some(json_start) = chunk.find("{") {
            if let Some(json_end) = chunk.find("</tool>") {
                let json_str = &chunk[json_start..json_end];
                if let Ok(json) = serde_json::from_str(json_str) {
                    tool_calls_json_clone.lock().unwrap().push(json);
                }
            }
        }
    }

    // 转发到 UI
    if let Ok(mut cb) = on_chunk_clone.lock() {
        cb(chunk);
    }
});
```

### 7.3 UI 层渲染（app.rs）

```rust
Event::Text(text) => {
    if text.starts_with("<thinking>") {
        // 显示思考开始标签，累积内容
        current_spans.push(Span::styled("<thinking> ", Style::new().fg(Color::DarkGray)));
    } else if text.starts_with("</thinking>") {
        // 结束思考，换行
        push_line(&mut lines, &mut current_spans);
    } else if text.starts_with("<tool>") {
        // 工具调用开始，换行并显示
        push_line(&mut lines, &mut current_spans);
        let tool_name = extract_tool_name(text);
        current_spans.push(Span::styled(
            format!("<tool>{}</tool>", tool_name),
            Style::new().fg(Color::Cyan).bold()
        ));
        push_line(&mut lines, &mut current_spans);
    } else if text.starts_with("</tool>") {
        push_line(&mut lines, &mut current_spans);
    }
    // ... 其他处理
}
```

## 8. 日志记录规范

### 8.1 必需日志

| 阶段 | 日志内容 |
|------|----------|
| 开始请求 | `Starting request to {provider}` |
| 工具列表 | `Tools available: N` |
| 工具调用 | `Tools: - {name}: {description}` |
| Provider 调用 | `PROVIDER: chat_with_tools_streaming called` |
| Provider 返回 | `PROVIDER: Got {N} tools` |
| Agent 响应 | `AGENT: Got response, content len: {N}, tool_calls: {M}` |
| 工具执行 | `AGENT: Executing tool: {name}` |
| 工具结果 | `AGENT: Tool {name} result: {preview}` |

### 8.2 调试日志格式

```
[HH:MM:SS.mmm] {SOURCE}: {message}
```

示例：
```
[14:28:49.111] STREAM: <thinking>
[14:28:49.111] STREAM: The user wants...
[14:28:49.891] STREAM: <tool>bash</tool>
```

## 9. 修订历史

| 日期 | 版本 | 修改内容 |
|------|------|----------|
| 2026-05-18 | v1.0 | 初始版本，定义状态机和标签规范 |
