# Reasoning/Thinking 内容处理方案

> 文档版本: 1.0
> 生成日期: 2026-06-02
> 分支: feat/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **初稿 - 待审批**

---

## 概述

本文档分析各大 LLM Provider 的 reasoning/thinking 内容处理机制，为 T8（DeepSeek reasoning 累积）提供设计依据。

**TL;DR**：
- DeepSeek/Qwen 等国产模型：需要在多轮对话中**累积并传回** `reasoning_content`
- OpenAI：reasoning tokens **不通过 API 返回**，无需处理
- Anthropic/Ollama/MiniMax：**部分支持**，机制各异

---

## 一、Provider 分类

### 1.1 必须处理 Reasoning 的 Provider

| Provider | Reasoning 字段 | 流式可见 | 多轮必须传回 |
|----------|---------------|---------|-------------|
| DeepSeek | `reasoning_content` | ✅ | ✅ **必须** |
| Qwen (阿里云百炼) | `reasoning_content` 或 `thinking` | ✅ | ✅ 必须 |

### 1.2 不需要/无法处理 Reasoning 的 Provider

| Provider | Reasoning 字段 | 说明 |
|----------|---------------|------|
| OpenAI (GPT-5.5/o1/o3) | 内部 `reasoning_tokens` | ❌ 不通过 API 返回 |

### 1.3 机制各异的 Provider

| Provider | 字段/机制 | 说明 |
|----------|-----------|------|
| Anthropic Claude | `thinking` | 部分支持，需确认 streaming 机制 |
| Ollama | `thinking` / `reasoning` | 不同模型不同，kimi-k2.5、glm-5 等用 `thinking` |
| MiniMax | `thinking` | M2 支持 Interleaved Thinking |

---

## 二、各 Provider 详细分析

### 2.1 DeepSeek

**API**: OpenAI 兼容

#### 流式响应格式

```json
{"choices":[{"index":0,"delta":{"reasoning_content":"让我思考一下..."}}]}
{"choices":[{"index":0,"delta":{"content":"如何"}}]}
{"choices":[{"index":0,"delta":{"content":"帮助"}}]}
{"choices":[{"index":0,"delta":{"content":"你"}}]}
data: [DONE]
```

#### 关键发现

> **多轮对话必须传回 reasoning_content**
>
> 如果在多轮对话中不传回 reasoning_content，API 返回 400 错误：
> ```
> API Error: 400 The reasoning_content in the thinking mode must be passed back to the API.
> ```

#### 代码库实现

```rust
// deepseek.rs stream_chat_with_tools
if let Some(reasoning) = json["choices"][0]["delta"]["reasoning_content"].as_str() {
    if !reasoning.is_empty() {
        on_chunk(reasoning.to_string());  // 无法区分是 reasoning 还是 content
    }
}
if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
    if !content.is_empty() {
        on_chunk(content.to_string());  // 两者都通过同一个 on_chunk 发送
    }
}
```

#### T8 需要的改动

1. 区分 `reasoning_content` 和 `content` 回调
2. 累积 `reasoning_content` 到 `reasoning_history`
3. 下一轮请求时将 reasoning_content 传回

---

### 2.2 OpenAI

**API**: Responses API / Chat Completions API

#### Reasoning 机制

```
GPT-5.5 / o1 / o3 使用内部 reasoning_tokens
- reasoning tokens 不通过 API 返回
- 占据 context window 空间
- 在 usage 对象中可见数量，但不暴露内容
```

#### 控制参数

| 参数 | 值 | 说明 |
|------|---|------|
| `reasoning.effort` | `none`, `minimal`, `low`, `medium`, `high`, `xhigh` | 控制推理努力程度 |

#### 结论

> OpenAI 的 reasoning 内容**不通过 API 返回**，无法累积。

---

### 2.3 Qwen (阿里云百炼)

**API**: OpenAI 兼容

#### 流式响应

```json
// 某些模型
{"choices":[{"index":0,"delta":{"reasoning_content":"..."}}]}
{"choices":[{"index":0,"delta":{"content":"..."}}]}

// 某些模型（原生接口）
{"choices":[{"index":0,"delta":{"thinking":"..."}}]}
```

#### 问题

> Qwen 存在 OpenAI 兼容接口和原生接口混用问题：
> - OpenAI 兼容接口：`reasoning_content`
> - 原生接口：`thinking`（被错误映射到 `reasoning`）

#### T8 需要的改动

同 DeepSeek：累积 `reasoning_content` 并传回。

---

### 2.4 Anthropic Claude

**API**: Anthropic SDK / OpenAI 兼容

#### Reasoning 机制

Claude 3.7 Sonnet 支持 Visible Extended Thinking：
- `thinkingBudget` 参数控制 token 数量
- 部分响应通过 streaming 返回

#### 结论

> 需要进一步确认 streaming 时 thinking 内容是否可见

---

### 2.5 Ollama

**API**: Native API (`/api/chat`)

#### 问题

> 不同模型使用不同字段：
> - deepseek-r1 等：`reasoning`
> - kimi-k2.5、glm-5、minimax-m2.5 等：`thinking`

#### 代码问题

```javascript
// openclaw 项目发现的问题
// 当前代码检查 message.reasoning
// 但某些模型返回的是 message.thinking
else if (chunk.message?.reasoning)  // 错误
    accumulatedContent += chunk.message.reasoning;
// 应该是
else if (chunk.message?.thinking)  // 正确
    accumulatedContent += chunk.message.thinking;
```

---

### 2.6 MiniMax

**API**: Anthropic API 兼容 / OpenAI 兼容

#### Interleaved Thinking

MiniMax-M2 是第一个完整支持 Interleaved Thinking 的开源模型：
- 可以在工具调用之间进行思考
- 支持 Anthropic API 格式

#### 结论

> 需要进一步确认 streaming 机制

---

## 三、方案设计

### 3.1 问题分析

当前实现中，`reasoning_content` 和 `content` 都通过同一个 `on_chunk(String)` 回调发送，UI 无法区分：

```rust
// 当前签名
on_chunk(String)

// 问题：无法区分
on_chunk("thinking...")   // reasoning_content
on_chunk("final answer")  // content
```

### 3.2 解决方案

#### 方案 A：改变回调签名为 `FnMut(StreamingChunk)`（推荐）

```rust
pub enum StreamingChunk {
    Reasoning(String),   // 推理内容，需累积
    Content(String),      // 最终回复，不累积
    ToolCall { name: String, arguments: String },
}

pub async fn chat_with_tools_streaming<F>(&self, messages: Vec<Message>, on_chunk: F) -> Result<ModelResponse>
    where F: FnMut(StreamingChunk) + Send + Sync + 'static;
```

**优点**：
- 类型安全，编译期检查
- UI 可以区分显示
- Agent 可以区分累积

**缺点**：
- 需要更新所有 Provider 实现
- 影响较大

#### 方案 B：保持 `FnMut(String)`，用前缀区分

```rust
on_chunk("[REASONING]thinking content[/REASONING]");
on_chunk("final answer");
```

**优点**：
- 改动小

**缺点**：
- 不够优雅
- 解析容易出错

#### 方案 C：DeepSeek Provider 内部累积

```rust
impl DeepSeekProvider {
    pub fn get_reasoning_history(&self) -> Vec<String> { ... }
}
```

**优点**：
- 改动集中

**缺点**：
- Provider 特定，不通用

---

## 四、实现计划

### 4.1 分阶段实施

#### Phase 1：定义 StreamingChunk 枚举
- 在 `swiftforge-types` 中定义 `StreamingChunk` 枚举
- 更新 `Agent` 添加 `reasoning_history` 字段

#### Phase 2：更新 Provider 接口
- 更新 `ToolCallingProvider::stream_chat_with_tools` 签名
- 更新所有 Provider 实现

#### Phase 3：集成到 Agent
- `chat_with_tools_streaming` 使用 `StreamingChunk::Reasoning` 累积
- 下一轮请求时将 reasoning_content 传回

### 4.2 多轮对话 reasoning 传回格式

```json
{
  "messages": [
    {"role": "user", "content": "问题"},
    {"role": "assistant", "content": "回答", "reasoning_content": "推理过程"},
    {"role": "user", "content": "追问", "reasoning_content": "上轮推理过程"}
  ]
}
```

---

## 五、风险与注意事项

| 风险 | 说明 | 应对 |
|------|------|------|
| reasoning_content 过长 | 占据 context window | 可选截断 |
| Provider 不一致 | 各家字段名不同 | 适配层处理 |
| 多轮累积膨胀 | 消息历史无限增长 | 定期摘要/截断 |

---

## 六、结论

**T8 主要针对 DeepSeek 和 Qwen**，两者都需要在多轮对话中累积并传回 `reasoning_content`。

**推荐方案 A**：改变回调签名为 `StreamingChunk` 枚举。

**待确认**：
- Anthropic streaming 时 thinking 可见性
- MiniMax streaming 机制
- Ollama 各模型的 thinking 字段差异

---

*文档状态: 初稿 - 待审批*
