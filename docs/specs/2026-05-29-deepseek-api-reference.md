# DeepSeek API Reference

> 版本: 1.0
> 日期: 2026-05-29
> 状态: L4 记录 (API 参考文档)
> 来源: [DeepSeek API Docs](https://api-docs.deepseek.com)

---

## 一、概述

DeepSeek API 是一个 OpenAI 兼容的 API 平台，提供对 DeepSeek 高级语言模型的访问，包括 DeepSeek-V4 和 DeepSeek-R1 系列模型。

**Base URL**: `https://api.deepseek.com`

**认证方式**: Bearer Token

```http
Authorization: Bearer {DEEPSEEK_API_KEY}
Content-Type: application/json
```

---

## 二、支持的模型

| 模型 ID | 说明 |
|---------|------|
| `deepseek-v4-flash` | 快速响应，适合日常使用 |
| `deepseek-v4-pro` | 高性能，适合复杂任务 |

**注意**: 代码库中旧模型 `deepseek-chat` 已废弃，建议使用上述新模型。

---

## 三、API 端点

### 3.1 列出可用模型

```
GET /models
```

**请求示例**:

```bash
curl https://api.deepseek.com/models \
  -H "Authorization: Bearer {API_KEY}"
```

**响应**:

```json
{
  "object": "list",
  "data": [
    {
      "id": "deepseek-v4-flash",
      "object": "model",
      "owned_by": "deepseek"
    },
    {
      "id": "deepseek-v4-pro",
      "object": "model",
      "owned_by": "deepseek"
    }
  ]
}
```

---

### 3.2 查询账户余额

```
GET /user/balance
```

**请求示例**:

```bash
curl https://api.deepseek.com/user/balance \
  -H "Authorization: Bearer {API_KEY}"
```

**响应**:

```json
{
  "is_available": true,
  "balance_infos": [
    {
      "currency": "CNY",
      "total_balance": "110.00",
      "granted_balance": "10.00",
      "topped_up_balance": "100.00"
    }
  ]
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `is_available` | boolean | 账户余额是否足够进行 API 调用 |
| `balance_infos[].currency` | string | 货币单位 (CNY, USD) |
| `balance_infos[].total_balance` | string | 总余额 |
| `balance_infos[].granted_balance` | string | 赠送余额 |
| `balance_infos[].topped_up_balance` | string | 充值余额 |

---

### 3.3 聊天补全 (Chat Completions)

```
POST /chat/completions
```

这是主要的对端点，用于与 DeepSeek 模型进行对话。

#### 请求头

| 头信息 | 必填 | 说明 |
|--------|------|------|
| `Content-Type` | 是 | `application/json` |
| `Authorization` | 是 | `Bearer {API_KEY}` |

#### 请求体参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `model` | string | 是 | - | 模型 ID: `deepseek-v4-flash`, `deepseek-v4-pro` |
| `messages` | array | 是 | - | 消息列表 |
| `thinking` | object | 否 | `{"type": "enabled"}` | 思维模式控制 |
| `thinking.type` | string | 否 | `"enabled"` | `"enabled"` 或 `"disabled"` |
| `reasoning_effort` | string | 否 | `"high"` | 推理努力程度: `"high"`, `"max"`, `"low"` |
| `max_tokens` | integer | 否 | - | 最大生成 token 数 |
| `response_format` | object | 否 | `{"type": "text"}` | 输出格式 |
| `response_format.type` | string | 否 | `"text"` | `"text"` 或 `"json_object"` |
| `stop` | string/array | 否 | - | 停止序列 (最多 16 个) |
| `stream` | boolean | 否 | `false` | 是否使用流式响应 |
| `stream_options` | object | 否 | - | 流式选项 (仅在 `stream: true` 时有效) |
| `stream_options.include_usage` | boolean | 否 | - | 是否在 `data: [DONE]` 前发送 usage chunk |
| `temperature` | number | 否 | `1` | 采样温度 (0-2) |
| `top_p` | number | 否 | - | 核采样概率 (0-1) |

#### 消息对象 (messages[])

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `role` | string | 是 | 角色: `system`, `user`, `assistant`, `tool` |
| `content` | string | 是 | 消息内容 |
| `name` | string | 否 | 参与者名称 |
| `prefix` | boolean | 否 | 强制以指定内容开头 (Beta) |
| `tool_call_id` | string | 工具消息必填 | 工具调用 ID |

#### 请求示例 (非流式)

```bash
curl https://api.deepseek.com/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {API_KEY}" \
  -d '{
    "model": "deepseek-v4-flash",
    "messages": [
      {"role": "system", "content": "你是一个有用的助手。"},
      {"role": "user", "content": "你好！"}
    ],
    "thinking": {"type": "enabled"},
    "reasoning_effort": "high"
  }'
```

#### 请求示例 (流式)

```bash
curl https://api.deepseek.com/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {API_KEY}" \
  -d '{
    "model": "deepseek-v4-flash",
    "messages": [
      {"role": "user", "content": "用 Python 写一个快速排序"}
    ],
    "thinking": {"type": "enabled"},
    "reasoning_effort": "high",
    "stream": true
  }'
```

#### 响应 (非流式)

```json
{
  "id": "chatcmpl-12345",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "deepseek-v4-flash",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "这里是回复内容",
        "reasoning_content": "这里是推理过程 (当 thinking.type=enabled 时)"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 50,
    "total_tokens": 70,
    "completion_tokens_details": {
      "reasoning_tokens": 30
    }
  }
}
```

#### 响应字段说明

| 字段 | 说明 |
|------|------|
| `id` | 聊天补全的唯一标识符 |
| `object` | 对象类型，固定为 `"chat.completion"` |
| `created` | Unix 时间戳 |
| `model` | 实际使用的模型 |
| `choices[].message.content` | 助手回复内容 |
| `choices[].message.reasoning_content` | 推理内容 (仅在 thinking 模式启用时) |
| `choices[].finish_reason` | 停止原因: `stop`, `length`, `content_filter`, `tool_calls`, `insufficient_system_resource` |
| `usage.prompt_tokens` | 提示 token 数 |
| `usage.completion_tokens` | 完成 token 数 |
| `usage.completion_tokens_details.reasoning_tokens` | 推理 token 数 |

#### 流式响应 (SSE)

```json
data: {"choices":[{"index":0,"delta":{"role":"assistant","content":""}}]}
data: {"choices":[{"index":0,"delta":{"content":"你好"}}]}
data: {"choices":[{"index":0,"delta":{"reasoning_content":"让我思考一下..."}}]}
data: {"choices":[{"index":0,"delta":{"content":"，如何"}}]}
data: {"choices":[{"index":0,"delta":{"content":"帮助你？"}}]}
...
data: [DONE]
```

**注意**: 
- `reasoning_content` 字段表示思维链内容
- 流式响应中 `finish_reason` 不会在每个 chunk 中出现，只在最后出现

---

## 四、错误码

| HTTP 状态码 | 错误码 | 说明 | 原因 | 解决方案 |
|------------|--------|------|------|---------|
| 400 | Invalid Format | 请求格式无效 | 请求体格式错误 | 根据错误提示修改请求体 |
| 401 | Authentication Fails | 认证失败 | API Key 错误或无效 | 检查 API Key 或创建新的 |
| 402 | Insufficient Balance | 余额不足 | 账户余额耗尽 | 检查余额并充值 |
| 422 | Invalid Parameters | 参数无效 | 请求包含无效参数 | 根据错误提示修改参数 |
| 429 | Rate Limit Reached | 请求过于频繁 | 请求速度超过限制 | 降低请求频率或切换 provider |
| 500 | Server Error | 服务器内部错误 | DeepSeek 服务器问题 | 稍后重试，如持续发生请联系 support |
| 503 | Server Overloaded | 服务器过载 | 流量负载过高 | 稍后重试 |

---

## 五、代码库实现对照

### 5.1 当前实现的端点

| 函数 | HTTP 方法 | 端点 | 状态 |
|------|-----------|------|------|
| `list_models()` | GET | `/v1/models` | ✅ 已实现 |
| `chat()` | POST | `/chat/completions` | ✅ 已实现 |
| `chat_with_tools()` | POST | `/chat/completions` | ✅ 已实现 |
| `stream_chat()` | POST | `/chat/completions` (stream=true) | ✅ 已实现 |
| `stream_chat_with_tools()` | POST | `/chat/completions` (stream=true) | ✅ 已实现 |

### 5.2 待实现的端点

| 函数 | HTTP 方法 | 端点 | 说明 |
|------|-----------|------|------|
| `get_balance()` | GET | `/user/balance` | ❌ 未实现 |

### 5.3 模型配置

| 文件 | 当前值 | 建议值 |
|------|--------|--------|
| `providers/deepseek.rs:58` | `deepseek-chat` | `deepseek-v4-flash` |
| `tui/config.rs:56` | `deepseek-v4-pro` | `deepseek-v4-flash` |

### 5.4 请求体差异

**代码库当前实现**:

```rust
// 基础聊天
serde_json::json!({
    "model": self.model,
    "messages": [...],
    "thinking": {"type": "enabled"},
    "reasoning_effort": "high"
})

// 带工具聊天
serde_json::json!({
    "model": self.model,
    "messages": [...],
    "tools": [...],
    "tool_choice": "auto",
    "stream": true,
    "thinking": {"type": "enabled"},
    "reasoning_effort": "low"
})
```

**差异说明**:
- 代码未实现 `response_format` 参数
- 代码未实现 `max_tokens` 参数
- 代码未实现 `stream_options`

---

## 六、使用示例

### 6.1 Rust 代码调用示例

```rust
use crate::providers::deepseek::DeepSeekProvider;

let provider = DeepSeekProvider::new(
    "your-api-key".to_string(),
    None, // 使用默认 base_url
    Some("deepseek-v4-flash".to_string())
);

// 非流式聊天
let messages = vec![
    Message { role: "user".to_string(), content: "你好".to_string() }
];
let response = provider.chat(messages).await?;
println!("{}", response.content);

// 流式聊天
provider.stream_chat(messages, Box::new(|chunk| {
    print!("{}", chunk);
})).await?;
```

### 6.2 cURL 调用示例

```bash
# 非流式
curl https://api.deepseek.com/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {API_KEY}" \
  -d '{
    "model": "deepseek-v4-flash",
    "messages": [{"role": "user", "content": "解释一下什么是机器学习"}]
  }'

# 流式
curl https://api.deepseek.com/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {API_KEY}" \
  -d '{
    "model": "deepseek-v4-flash",
    "messages": [{"role": "user", "content": "解释一下什么是机器学习"}],
    "stream": true
  }'
```

---

## 七、限制与配额

| 限制项 | 说明 |
|--------|------|
| 速率限制 | 取决于账户等级 |
| 最大上下文 | 参考模型规格 |
| 停止序列 | 最多 16 个 |
| 消息数组大小 | 受 token 限制 |

---

## 八、参考链接

- [DeepSeek API 官方文档](https://api-docs.deepseek.com)
- [DeepSeek Platform](https://platform.deepseek.com)
- [API 密钥管理](https://platform.deepseek.com/api_keys)