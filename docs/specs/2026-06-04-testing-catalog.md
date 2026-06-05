# SwiftForge 测试项目详细清单

> 版本: 1.0
> 日期: 2026-06-04
> 状态: 已批准
> 类型: L3 设计/Spec

---

## 一、概述

本文档列出 SwiftForge 项目所有需要测试的功能项目，按模块分类。每项测试都标注了测试场景、输入/输出期望、错误处理路径。

**测试总计**: 85+ 测试项

| 模块 | 测试项数 | 状态 |
|------|---------|------|
| Agent 核心 | ~25 | 缺多项 |
| Session 管理 | ~10 | 部分覆盖 |
| Provider 集成 | 24 | 基本覆盖 |
| Tools 工具 | ~15 | 基本覆盖 |
| Orchestration | ~10 | 部分覆盖 |
| MCP 协议 | ~10 | 基本覆盖 |
| Hooks 事件 | ~20 | 基本覆盖 |
| Skill 加载 | ~10 | 基本覆盖 |
| TUI 界面 | ~15 | 未覆盖 |

---

## 二、Agent 核心模块 (`swiftforge/src/core/agent.rs`)

### 2.1 配置与初始化

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `Agent::new()` 基本创建 | 创建带有效 config 和 llm_provider 的 Agent | Agent 创建成功，name/role/config 正确 |
| `Agent::new()` 无 provider | 创建时不提供 provider | 编译通过，has_provider() 返回 true |
| `with_scheduler()` Builder | 链式调用 `.with_scheduler(scheduler)` | scheduler 被正确存储 |
| `with_message_bus()` Builder | 链式调用 `.with_message_bus(bus)` | message_bus 被正确存储 |
| `with_tool_registry()` Builder | 链式调用 `.with_tool_registry(registry)` | tool_registry 被正确存储 |
| `with_tool_provider()` nil | 调用 `.with_tool_provider(None)` | tool_provider 为 None |

### 2.2 访问器方法

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `name()` | 获取 agent 名称 | 返回 config.name |
| `role()` | 获取 agent 角色 | 返回 config.role |
| `config()` | 获取完整配置 | 返回 AgentConfig 引用 |
| `provider_name()` | 获取 provider 名称 | 委托给 llm_provider.provider_name() |
| `has_tool_registry()` 有 registry | 注册工具后调用 | 返回 true |
| `has_tool_registry()` 无 registry | 未注册工具时调用 | 返回 false |
| `list_tools()` 有 registry | 注册工具后调用 | 返回工具名称列表 |
| `list_tools()` 无 registry | 未注册工具时调用 | 返回空 Vec |
| `get_tool_definitions()` 有 registry | 注册工具后调用 | 返回 Vec<ToolDefinition> |
| `get_tool_definitions()` 无 registry | 未注册工具时调用 | 返回空 Vec |

### 2.3 工具操作

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `call_tool()` 成功 | 注册 EchoTool，调用 `call_tool("echo", {"input": "hello"})` | 返回 `ToolResult { success: true, output: Some("echo: hello") }` |
| `call_tool()` 无 registry | 未注册工具时调用 | 返回错误: "No tool registry configured" |
| `call_tool()` 工具不存在 | 调用未注册的工具名 | 返回错误: "Tool 'unknown' not found" |
| `parse_tool_calls()` JSON 格式 | 传入 `{"tool_calls":[{"name":"read","arguments":{"path":"/test"}}]}` | 返回 1 个 ToolCall，name="read"，arguments 包含 path |
| `parse_tool_calls()` XML 格式 | 传入 `<tool_call>{"name":"read","arguments":{"path":"/test"}}</tool_call>` | 返回 1 个 ToolCall |
| `parse_tool_calls()` 多个工具 | 传入含 2 个 tool_call 的内容 | 返回 2 个 ToolCall |
| `parse_tool_calls()` 无效 JSON | 传入格式错误的 JSON | 返回空 Vec 或忽略无效部分 |
| `parse_tool_calls()` 混合内容 | 传入 `Some text <tool_call>{"name":"echo"}</tool_call> more` | 返回 1 个 ToolCall |
| `parse_tool_calls_from_json()` OpenAI 格式 | 传入 `[{"name":"read","arguments":{}}]` | 正确解析 |
| `parse_tool_calls_from_json()` function 格式 | 传入 `[{"function":{"name":"read","arguments":{}}}]` | 正确解析 (嵌套 function) |
| `parse_tool_calls_from_json()` 缺少字段 | 传入 `[{"name":"read"}]` (无 arguments) | arguments 为空 HashMap |
| `execute_tool_calls()` 成功 | 执行 2 个 echo 工具调用 | 返回 2 个 ToolResult，都 success=true |
| `execute_tool_calls()` 无 registry | 未注册工具时调用 | 返回错误: "No tool registry configured" |
| `execute_tool_calls()` 部分失败 | 执行一个存在的和一个不存在的工具 | 存在的成功，不存在的返回 not found 错误 |

### 2.4 对话操作

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `chat()` 成功 | 发送消息，获取回复 | 返回 ModelResponse，content 非空 |
| `chat()` Provider 错误 | Provider 返回错误 | 错误被包装为 anyhow::Error |
| `chat_with_tools()` 成功 | 带工具定义发送消息 | 返回 ModelResponse |
| `chat_with_tools()` 无 tool provider | 未设置 tool_provider 时调用 | 返回错误: "No tool-calling provider configured" |
| `chat_with_tools()` Provider 错误 | tool provider 返回错误 | 错误被传播 |
| `chat_with_tools_streaming()` 成功 | 流式发送消息，on_chunk 累积 content | content 和 reasoning 正确累积 |
| `chat_with_tools_streaming()` reasoning 分块 | DeepSeek 等支持 reasoning 的 provider | reasoning_content 被正确分离和累积 |
| `chat_with_tools_streaming()` tool_call 累积 | 返回 tool_calls 时 | tool_calls JSON 被正确累积到 Vec |
| `chat_with_tools_streaming()` 无 tool provider | 未设置 tool_provider 时调用 | 返回错误: "No tool-calling provider configured" |

### 2.5 Agent 循环

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `run_agent_loop()` 基本流程 | 发送消息 → chat → 执行工具 → 返回 | 完整执行，返回响应文本 |
| `run_agent_loop()` 无工具调用 | LLM 返回纯文本响应 | 直接返回 content，不执行工具 |
| `run_agent_loop()` 多轮工具 | 连续执行多个工具调用 | 每轮正确添加 message，工具结果被送回 LLM |
| `run_agent_loop()` max_iterations | 设置 max_iterations=2，执行需要 3 次工具的流程 | 第 2 次后停止，返回 "Tool execution completed." |
| `run_agent_loop()` 空响应 | LLM 返回空 content 且无工具调用 | 返回 "Tool execution completed." |
| `run_agent_loop()` compact 触发 | 消息数量超过 context_window 80% | 调用 compact() 总结会话 |
| `run_agent_loop()` compact 失败 | compact() 返回错误 | 记录 warn 日志，继续执行 |
| `run_agent_loop()` 流式 UI | 提供 stream_ui sender | 每次 chunk 发送文本到 UI |

### 2.6 任务操作

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `process_task()` 有任务 | scheduler 有待处理任务 | 返回 Some(Task)，状态变为 Running |
| `process_task()` 空队列 | scheduler 无任务 | 返回 None |
| `process_task()` 无 scheduler | 未设置 scheduler | 返回错误: "No scheduler configured" |
| `complete_task()` 存在 | 调用已存在的 task_id | 任务状态变为 Completed |
| `complete_task()` 不存在 | 调用不存在的 task_id | 无效果 (内部实现) |
| `complete_task()` 无 scheduler | 未设置 scheduler | 返回错误 |
| `fail_task()` 存在 | 调用已存在的 task_id | 任务状态变为 Failed |
| `fail_task()` 无 scheduler | 未设置 scheduler | 返回错误 |

### 2.7 消息总线操作

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `send_message()` 成功 | 发送消息到已订阅的 agent | 消息被投递到 handler |
| `send_message()` 目标不存在 | 发送消息到不存在的 agent | handler 不被调用 |
| `send_message()` 无 message_bus | 未设置 message_bus | 返回错误: "No message bus configured" |
| `broadcast()` 成功 | 广播消息给所有 agent | 所有订阅者收到消息 (除发送者) |
| `broadcast()` 无 message_bus | 未设置 message_bus | 返回错误 |

### 2.8 状态查询

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `is_connected()` 全连接 | scheduler + message_bus 都设置 | 返回 true |
| `is_connected()` 仅 scheduler | 只设置 scheduler | 返回 false |
| `is_connected()` 仅 message_bus | 只设置 message_bus | 返回 false |
| `is_connected()` 都不设置 | 两者都未设置 | 返回 false |
| `has_provider()` 总是 | 任何情况 | 返回 true (因为 Agent 要求 provider) |
| `has_tool_provider()` 已设置 | 设置 tool_provider | 返回 true |
| `has_tool_provider()` 未设置 | 未设置 tool_provider | 返回 false |
| `list_models()` 成功 | 调用 provider.list_models() | 返回模型列表 |
| `list_models()` Provider 错误 | provider 返回错误 | 错误被传播 |

---

## 三、Session 管理模块

### 3.1 SessionManager (session_manager.rs)

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` 成功 | 提供有效 data_dir 路径 | 创建目录，打开 SQLite，返回 Manager |
| `new()` 目录创建失败 | data_dir 无效或无权限 | 返回错误 |
| `new()` DB 打开失败 | 提供无效 DB 路径 | 返回错误 |
| `create_session()` 成功 | 创建新 session | UUID 生成，保存到 DB，缓存，当前 session 切换 |
| `create_session()` DB 保存失败 | DB 写入错误 | 返回错误 |
| `get_session()` 缓存命中 | session 在内存缓存中 | 直接返回，不查 DB |
| `get_session()` 缓存未命中 | session 不在缓存中 | 从 DB 加载，存入缓存 |
| `get_session()` 都不存在 | session 既不在缓存也不在 DB | 返回 None |
| `switch_to()` 成功 | 切换到已存在的 session | 当前 session 更新 |
| `switch_to()` 不存在 | 切换到不存在的 session_id | 返回 SessionError::NotFound |
| `update_session()` 成功 | 更新已存在的 session | 保存到 DB，更新缓存 |
| `update_session()` DB 失败 | DB 写入错误 | 返回错误 |
| `delete_session()` 成功 | 删除已存在的 session | 从 DB 删除，移除缓存 |
| `delete_session()` 删除当前 | 删除当前活动的 session | 当前 session 清除 |
| `delete_session()` 不存在 | 删除不存在的 session | 无效果 |
| `list_sessions()` 有 sessions | DB 中有多个 session | 返回 Vec<(id, name)> |
| `list_sessions()` 空 | DB 中无 session | 返回空 Vec |

### 3.2 Session (libs/swiftforge-types/src/session.rs)

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建新 session | id, name, context_window 设置，消息为空 |
| `add_message()` user | 添加 user 角色消息 | messages.len() = 1，updated_at 更新 |
| `add_message()` assistant | 添加 assistant 角色消息 | messages.len() 增加 |
| `add_message()` 多个 | 添加多条消息 | 按顺序存储 |
| `messages()` | 获取消息列表 | 返回克隆的 Vec<Message> |
| `needs_compaction()` 未达到阈值 | token_count < context_window * 0.8 | 返回 false |
| `needs_compaction()` 达到阈值 | token_count > context_window * 0.8 | 返回 true |
| `estimate_token_count()` | 获取当前 token 估计 | 返回 token_count |
| `compact()` 消息少于 10 条 | messages.len() < 10 | 无操作 (no-op) |
| `compact()` 消息>=10 条 | messages.len() >= 10 | 调用 chat_fn 总结，消息减少 |
| `compact()` chat_fn 失败 | chat_fn 返回错误 | 返回 SessionError::CompactFailed |

---

## 四、Provider 集成模块

### 4.1 Provider 创建

| 测试项 | Provider | 预期结果 |
|--------|----------|----------|
| `OpenAIProvider::new()` 最小参数 | `new("sk-xxx", None)` | provider_name = "openai" |
| `OpenAIProvider::new()` 自定义 base_url | `new("sk-xxx", Some("https://proxy.com"))` | base_url 使用代理地址 |
| `AnthropicProvider::new()` | `new("sk-ant", None)` | provider_name = "anthropic" |
| `DeepSeekProvider::new()` 最小参数 | `new("sk-ds", None)` | provider_name = "deepseek" |
| `DeepSeekProvider::new()` 自定义 model | `new("sk-ds", None, Some("deepseek-coder"))` | 使用指定模型 |
| `OllamaProvider::new()` | `new(None, None)` | provider_name = "ollama"，无 API key |
| `OllamaProvider::new()` 自定义地址 | `new(Some("http://custom:11434"), None)` | 使用自定义地址 |
| `MiniMaxProvider::new()` | `new("sk-mm", None)` | provider_name = "minimax" |
| `CustomProvider::new()` | `new("custom", "sk-xxx", "https://api.custom.com", "custom-model")` | 使用提供的所有参数 |

### 4.2 chat() 方法

| 测试项 | Provider | 测试场景 | 预期结果 |
|--------|----------|----------|----------|
| `chat()` | OpenAI | 发送有效消息 | 返回 content，usage 非零 |
| `chat()` | Anthropic | 发送消息 | 返回 content |
| `chat()` | DeepSeek | 发送消息 | 返回 content，含 reasoning |
| `chat()` | Ollama | 本地模型 | 返回 content |
| `chat()` | MiniMax | 发送消息 | 返回 content |
| `chat()` | Custom | 发送消息 | 返回 content |
| `chat()` | OpenAI | API key 无效 | HTTP 401 错误 |
| `chat()` | OpenAI | 网络错误 | 返回 anyhow::Error |
| `chat()` | OpenAI | 响应无 content | 返回 error: "No content in response" |

### 4.3 stream_chat() 方法

| 测试项 | Provider | 测试场景 | 预期结果 |
|--------|----------|----------|----------|
| `stream_chat()` | OpenAI | 正常流式 | on_chunk 被多次调用，content 累积 |
| `stream_chat()` | Anthropic | 正常流式 | 同上 |
| `stream_chat()` | DeepSeek | 带 reasoning | reasoning 和 content 分开累积 |
| `stream_chat()` | Ollama | 本地流式 | 正确处理 SSE |
| `stream_chat()` | OpenAI | 中途断开 | 已累积的内容保留 |
| `stream_chat()` | OpenAI | 无效 key | HTTP 错误 |

### 4.4 list_models() 方法

| 测试项 | Provider | 测试场景 | 预期结果 |
|--------|----------|----------|----------|
| `list_models()` | OpenAI | 有效 API key | 返回 Vec<String> 包含模型名 |
| `list_models()` | Anthropic | - | 返回硬编码列表: claude-sonnet-4..., claude-3-5-sonnet... 等 6 个 |
| `list_models()` | DeepSeek | 有效 key | 返回 Vec<String> 包含 deepseek-chat 等 |
| `list_models()` | Ollama | 服务运行 | 返回 Vec<String> 来自 `/api/tags` |
| `list_models()` | Ollama | 服务未运行 | HTTP 连接错误 |
| `list_models()` | MiniMax | - | 返回硬编码列表: MiniMax-01, MiniMax-01-Turbo 等 |
| `list_models()` | Custom | - | 返回单个配置模型 |

### 4.5 错误处理

| 测试项 | 场景 | 预期结果 |
|--------|------|----------|
| HTTP 非 200 状态 | OpenAI/DeepSeek 返回 4xx/5xx | `anyhow::bail!("HTTP {status}: {body}")` |
| JSON 解析失败 | 响应不是有效 JSON | 静默忽略，使用 if let Ok |
| 缺少必需字段 | 响应缺少 content | `unwrap_or("")` 或 error |
| 工具调用提取 | OpenAI tool_calls 格式 | 正确提取 name 和 arguments |
| 认证失败 | 错误的 API key | 401 或 403 错误 |

---

## 五、Tools 工具模块

### 5.1 BashTool

| 测试项 | 输入 | 预期结果 |
|--------|------|----------|
| `execute()` echo | `{"command": "echo hello"}` | stdout="hello", success=true |
| `execute()` pwd | `{"command": "pwd"}` | 输出包含 "/" 或盘符 |
| `execute()` 不存在的命令 | `{"command": "nonexistent_cmd_xyz"}` | success=false, error 包含 "not found" |
| `execute()` 管道 | `{"command": "echo test \| cat"}` | stdout="test", success=true |
| `execute()` 退出码非零 | `{"command": "ls /nonexistent"}` | success=false, error 包含 "No such file" |
| `execute()` 缺少 command | `{}` | 执行 `sh -c ""`，空输出 |
| `execute()` 特殊字符 | `{"command": "echo $HOME"}` | 输出环境变量值 |
| `execute()` 超长运行 | `{"command": "sleep 10"}` | 阻塞直到完成 |

### 5.2 ReadTool

| 测试项 | 输入 | 预期结果 |
|--------|------|----------|
| `execute()` 存在的文件 | `{"path": "/tmp/test.txt"}` (文件有 "hello") | output="hello", success=true |
| `execute()` 空文件 | `{"path": "/tmp/empty.txt"}` | output="", success=true |
| `execute()` 不存在的文件 | `{"path": "/nonexistent/file.txt"}` | success=false, error 包含 "No such file" |
| `execute()` 无权限读取 | `{"path": "/root/restricted"}` | success=false, error 包含 "Permission denied" |
| `execute()` 目录路径 | `{"path": "/tmp"}` | 输出目录内容或错误取决于实现 |
| `execute()` 缺少 path | `{}` | 尝试读取空路径，可能失败 |
| `execute()` 大文件 | `{"path": "/tmp/large.bin"}` (100MB+) | 能读取，不限制大小 |
| `execute()` 二进制文件 | `{"path": "/tmp/binary.dat"}` | 可能产生 UTF-8 转换错误或成功 |

### 5.3 WriteTool

| 测试项 | 输入 | 预期结果 |
|--------|------|----------|
| `execute()` 创建新文件 | `{"path": "/tmp/new.txt", "content": "hello"}` | success=true, 文件存在且内容为 "hello" |
| `execute()` 覆盖文件 | `{"path": "/tmp/existing.txt", "content": "new content"}` | 原有内容被覆盖 |
| `execute()` 嵌套路径 | `{"path": "/tmp/a/b/c.txt", "content": "deep"}` | **失败**: 父目录不存在时不自动创建 |
| `execute()` 覆盖只读文件 | `{"path": "/tmp/readonly.txt", "content": "x"}` (权限 444) | **失败**: Permission denied |
| `execute()` 缺少 path | `{"content": "test"}` | path 为空字符串，行为取决于系统 |
| `execute()` 缺少 content | `{"path": "/tmp/test.txt"}` | 写入空字符串 |
| `execute()` Unicode 内容 | `{"path": "/tmp/unicode.txt", "content": "你好世界"}` | 正确写入 UTF-8 |

### 5.4 EditTool

| 测试项 | 输入 | 预期结果 |
|--------|------|----------|
| `execute()` 正常替换 | path 有 "hello world"，oldString="world"，newString="rust" | 文件内容变为 "hello rust" |
| `execute()` oldString 不存在 | oldString 在文件中不存在 | success=false, error="oldString '...' not found in file" |
| `execute()` 多个匹配 | 文件中多个 "test"，oldString="test" | **只替换第一个** |
| `execute()` oldString=newString | 无实际更改 | 报告成功，但文件不变 |
| `execute()` 空 oldString | oldString="" | 匹配文件开头 |
| `execute()` 文件不存在 | path 不存在 | success=false, error 包含 "No such file" |
| `execute()` 无写入权限 | path 权限只读 | success=false, error 包含 "Permission denied" |
| `execute()` 缺少 oldString | `{"path": "/tmp/a.txt", "newString": "x"}` | oldString 为空 |
| `execute()` 缺少 newString | `{"path": "/tmp/a.txt", "oldString": "x"}` | newString 为空，等同于删除 |

### 5.5 GrepTool

| 测试项 | 输入 | 预期结果 |
|--------|------|----------|
| `execute()` 匹配单行 | path 有 "line1\nneedle\nline3", pattern="needle" | output="2: needle", success=true |
| `execute()` 匹配多行 | path 有多个 "error" | 每行一条，格式 "行号: 内容" |
| `execute()` 无匹配 | pattern 不在文件中 | output="", success=true |
| `execute()` 空 pattern | pattern="" | **匹配所有行** |
| `execute()` 正则字符转义 | pattern="file(1).txt" | 当作**字面字符串**搜索，不解析为正则 |
| `execute()` 文件不存在 | path 不存在 | success=false, error 包含 "No such file" |
| `execute()` 无读取权限 | path 权限拒绝 | success=false, error 包含 "Permission denied" |
| `execute()` 二进制文件 | path 是二进制 | 可能有 UTF-8 解码错误，跳过失败行 |

### 5.6 ToolCallParser

| 测试项 | 输入 | 预期结果 |
|--------|------|----------|
| `parse()` JSON 格式 | 纯 JSON: `{"tool_calls":[{"name":"echo","arguments":{}}]}` | 返回 1 个 ToolCall |
| `parse()` XML 格式 | `<tool_call>{"name":"echo","arguments":{}}</tool_call>` | 返回 1 个 ToolCall |
| `parse()` 混合内容 | `Prefix <tool_call>{"name":"x"}</tool_call> Suffix` | 返回 1 个 ToolCall |
| `parse()` 多个 tool_call | 2 个 `<tool_call>` 标签 | 返回 2 个 ToolCall |
| `parse()` 无效 JSON | JSON 格式错误 | 返回空 Vec |
| `parse()` 无 tool_call 标签 | 纯文本 "hello world" | 返回空 Vec |
| `parse_from_json()` OpenAI 格式 | `[{"name":"echo","arguments":{}}]` | 正确解析 |
| `parse_from_json()` function 格式 | `[{"function":{"name":"echo","arguments":{}}}]` | 正确解析 |
| `parse_from_json()` 缺少 name | `[{"arguments":{}}]` | name 为空字符串 |
| `parse_from_json()` 缺少 arguments | `[{"name":"echo"}]` | arguments 为空 HashMap |
| `has_tool_calls()` 有工具 | 包含 `<tool_call>` | 返回 true |
| `has_tool_calls()` 无工具 | 纯文本 | 返回 false |

---

## 六、Orchestration 模块

### 6.1 TaskScheduler

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建新 scheduler | 内部 VecDeque 为空 |
| `add_task()` Normal | 添加普通优先级任务 | 插入到队列中部某位置 |
| `add_task()` Critical | 添加最高优先级任务 | **插入到最前面** (所有其他之前) |
| `add_task()` High | 添加高优先级任务 | 在 Normal/Low 之前，在 Critical 之后 |
| `add_task()` Low | 添加低优先级任务 | **插入到最后** |
| `add_task()` 同优先级 FIFO | 添加 2 个 Normal 任务 | 第一个先被取出 |
| `get_next_task()` 有任务 | 队列有任务 | 返回 Some(Task)，status 变为 Running |
| `get_next_task()` 空队列 | 队列为空 | 返回 None |
| `complete_task()` 存在 | 调用已存在的 task_id | 状态变为 Completed |
| `complete_task()` 不存在 | 调用不存在的 task_id | 无效果 |
| `fail_task()` 存在 | 调用已存在的 task_id | 状态变为 Failed |
| `fail_task()` 不存在 | 调用不存在的 task_id | 无效果 |
| `list_pending()` 有 pending | 有 Pending 任务 | 返回 Vec<Task> |
| `list_pending()` 全运行中 | 所有任务都在运行 | 返回空 Vec |
| `list_pending()` 混合状态 | 2 个 Pending，1 个 Completed | 返回 2 个 Pending |

### 6.2 MessageBus

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建新的 message bus | 内部 HashMap 为空 |
| `subscribe()` 新 agent | 为不存在的 agent 注册 handler | 创建空 Vec，添加 handler |
| `subscribe()` 多次 | 同一 agent 注册多个 handler | 所有 handler 都被调用 |
| `subscribe()` 已存在 agent | 为已存在的 agent 添加 handler | 追加到现有 Vec |
| `unsubscribe()` 存在 | 移除 agent 的所有 handler | Vec 被清空 |
| `unsubscribe()` 不存在 | 移除不存在的 agent | 无效果 |
| `send()` 目标存在 | 发送消息给已订阅的 agent | handler 被调用，收到消息 |
| `send()` 目标不存在 | 发送给未订阅的 agent | 无 handler 被调用 |
| `send()` 多个 handler | 发送给有多个 handler 的 agent | **所有 handler 按注册顺序被调用** |
| `broadcast()` 多个 agent | 广播给 3 个已订阅 agent | 每个 agent 的所有 handler 被调用 |
| `broadcast()` 排除发送者 | broadcast("agent1", ...) 从 agent1 调用 | agent1 不收到消息 |
| `broadcast()` handler 返回错误 | 任一 handler 返回 Err | **传播错误，停止广播** |

---

## 七、MCP 协议模块

### 7.1 MCPClient 连接管理

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建带 URL 的 client | 30s timeout, is_connected=false, is_initialized=false |
| `connect()` 成功 | 服务器可达 | is_connected=true (内部可能只是验证) |
| `connect()` 失败 | 服务器不可达 | 返回错误 |
| `disconnect()` | 断开连接 | is_connected=false, is_initialized=false, capabilities=None |
| `is_connected()` 初始 | 新创建的 client | false |
| `is_connected()` 连接后 | connect() 成功 | true |
| `is_connected()` 断开后 | disconnect() 后 | false |
| `is_initialized()` 初始 | 新创建的 client | false |
| `is_initialized()` 初始化后 | initialize() 成功 | true |
| `get_capabilities()` 已初始化 | 调用已初始化的 client | 返回 ServerCapabilities |
| `get_capabilities()` 未初始化 | 调用未初始化的 client | 返回错误: "Not initialized" |

### 7.2 JSON-RPC 协议

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `JsonRpcRequest::new()` | 创建请求 | version="2.0", id 递增 |
| `JsonRpcRequest::with_params()` | 添加 params | params 被设置 |
| `send_request()` 成功 | 有效 JSON-RPC 响应 | 返回解析后的 result |
| `send_request()` 非 200 | HTTP 错误状态 | 返回错误: "MCP server returned error: {status}" |
| `send_request()` JSON 解析失败 | 响应不是有效 JSON | 返回错误: "Failed to parse JSON-RPC response" |
| `send_request()` 版本无效 | version 不是 "2.0" | 返回错误: "Invalid JSON-RPC version: {version}" |
| `send_request()` ID 不匹配 | 响应 id 与请求 id 不同 | 返回错误: "Response ID mismatch" |
| `send_request()` JSON-RPC 错误 | 响应包含 error 对象 | 返回错误: "JSON-RPC error (code={code}): {message}" |
| `send_request()` 无 result | 响应无 result 字段 | 返回错误: "No result in JSON-RPC response" |

### 7.3 MCP 方法

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `initialize()` 成功 | 连接后调用 | is_initialized=true, capabilities 被缓存 |
| `initialize()` 未连接 | 未 connect 就调用 | 返回错误: "Not connected to MCP server" |
| `initialize()` 重复调用 | 已初始化后再次调用 | **返回缓存的 capabilities**，不重新发送 |
| `list_tools()` 已初始化 | initialize() 后调用 | 返回 Vec<Tool> |
| `list_tools()` 未连接 | 未 connect 就调用 | 返回错误: "Not connected to MCP server" |
| `list_tools()` 未初始化 | connect 后未 initialize 就调用 | 返回错误: "Not initialized - call initialize() first" |
| `call_tool()` 已初始化 | 调用存在的工具 | 返回 ToolResult |
| `call_tool()` 未连接 | 未 connect 就调用 | 返回错误: "Not connected to MCP server" |
| `call_tool()` 未初始化 | connect 后未 initialize 就调用 | 返回错误: "Not initialized - call initialize() first" |

---

## 八、Hooks 事件模块

### 8.1 HookEvent 类型

| Hook | 触发时机 | 测试场景 |
|------|----------|----------|
| OnStartup | 应用启动 | 触发时所有注册的 hook 被调用 |
| OnShutdown | 应用关闭 | 触发时所有注册的 hook 被调用 |
| OnError(String) | 错误发生时 | 传入错误消息字符串 |
| OnWarning(String) | 警告发生时 | 传入警告消息字符串 |
| OnInfo(String) | 信息日志 | 传入信息消息字符串 |
| OnDebug(String) | 调试日志 | 传入调试消息字符串 |
| OnAgentCreated(String) | Agent 创建时 | 传入 agent name |
| OnAgentDestroyed(String) | Agent 销毁时 | 传入 agent name |
| OnSessionStart | Session 启动时 | 触发时 hook 被调用 |
| OnSessionEnd | Session 结束时 | 触发时 hook 被调用 |
| OnMessageReceived(String) | 收到消息时 | 传入消息内容 |
| OnMessageSent(String) | 发送消息时 | 传入消息内容 |
| OnToolCall(String) | 工具调用时 | 传入工具名称 |
| OnToolResult(String, bool) | 工具返回时 | 传入工具名和 success 状态 |
| OnProviderCall(String) | Provider 调用时 | 传入 provider 类型 |
| OnProviderResponse(bool) | Provider 响应时 | 传入 success 状态 |

### 8.2 HookRegistry

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建空 registry | hooks 为空 |
| `register()` 新事件 | 注册第一个 hook | hooks 中创建条目 |
| `register()` 已有事件 | 追加 hook 到已有事件 | Vec 追加 |
| `register()` 带 priority | 注册时指定 priority | **priority 被存储但不被使用** |
| `dispatch()` 有 hook | 触发已注册的事件 | hook 被调用 |
| `dispatch()` 无 hook | 触发未注册的事件 | 无操作 (Ok) |
| `dispatch()` 多个 hooks | 同一事件有多个 hooks | **按注册顺序调用**，priority 不影响 |
| `dispatch()` hook 返回错误 | hook 返回 Err | **传播错误**，停止执行 |
| `dispatch()` 上下文传递 | dispatch 传入 event | HookContext 包含正确的事件 |
| `list_hooks()` 有注册 | 有已注册的 hooks | 返回事件名称列表 |
| `list_hooks()` 空 | 无注册 | 返回空 Vec |

### 8.3 HookContext

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new(event)` | 创建上下文 | event 被存储，timestamp 设置，metadata 为空 |
| `with_metadata()` | 添加元数据 | metadata 中添加键值对 |
| `metadata` 传递 | 多个 hooks 收到上下文 | 每个收到独立的 Clone |

---

## 九、Skill 加载模块

### 9.1 SkillLoader

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建新 loader | 成功创建 |
| `load_skill()` 存在文件 | 加载有效的 SKILL.md | 返回 Ok(Skill)，name/description/scope 正确 |
| `load_skill()` 文件不存在 | 路径不存在 | 返回错误: "Failed to read skill file: ..." |
| `load_skill()` frontmatter 格式 | `--- name: Test ... ---` | 正确解析 frontmatter |
| `load_skill()` inline YAML 格式 | `name: Test\ndescription: ...` | 正确解析 inline YAML |
| `parse_skill()` 完整 frontmatter | 包含 name, description, scope | 各字段正确提取 |
| `parse_skill()` 缺少 name | frontmatter 无 name | 使用默认值: "Unknown Skill" |
| `parse_skill()` 缺少 description | frontmatter 无 description | 使用默认值: "No description" |
| `parse_skill()` 缺少 scope | frontmatter 无 scope | 使用默认值: "Project" (SkillScope::Project) |
| `parse_skill()` scope=Global | scope: Global | SkillScope::Global |
| `parse_skill()` scope=User | scope: User | SkillScope::User |
| `parse_skill()` scope=Invalid | scope: SomethingElse | 默认为 Project |
| `load_from_directory()` 有效目录 | 目录有 3 个 .md 文件 | 返回 3 个 Skills |
| `load_from_directory()` 部分文件失败 | 3 个 .md，1 个不存在 | **跳过失败的文件**，返回 2 个 |
| `load_from_directory()` 非目录 | 路径是文件不是目录 | 返回空 Vec |

### 9.2 SkillRegistry

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| `new()` | 创建空 registry | HashMap 为空 |
| `register()` 新 skill | 注册新 skill | 插入 Map，enabled=true |
| `register()` 已存在 | 注册同名 skill | **覆盖**原有 |
| `get()` 存在 | 获取已注册的 skill | 返回 Some(Arc<Skill>) |
| `get()` 不存在 | 获取未注册的 skill | 返回 None |
| `enable()` 存在 | 启用已存在的 skill | 返回 true |
| `enable()` 不存在 | 启用不存在的 skill | 返回 false |
| `disable()` 存在 | 禁用已存在的 skill | 返回 true |
| `disable()` 不存在 | 禁用不存在的 skill | 返回 false |
| `list_skills()` 有注册 | 有 3 个 skills | 返回 3 个名称 |
| `list_skills()` 空 | 无注册 | 返回空 Vec |
| `list_enabled()` 部分启用 | 3 个 skill，2 个 enabled | 返回 2 个名称 |

---

## 十、TUI 界面模块

### 10.1 ChatView 渲染

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| 布局渲染 | 4 部分垂直布局 | messages, input, status, debug_panel 都显示 |
| 用户消息样式 | role=user | 绿色粗体 |
| Assistant 消息样式 | role=assistant | 青色粗体 |
| System 消息样式 | role=system | 黄色粗体 |
| Error 消息样式 | role=error | 红色粗体 |
| 流式指示器 | 正在接收流式响应 | 闪烁 `▌` 光标在内容末尾 |
| CJK 宽度 | 显示中文 "你好" | 占用 4 个单元格宽度 |
| 文本换行 | 长行自动换行 | 考虑 display width |
| 滚动条状态 | 内容超出视口 | scrollbar_position 同步更新 |

### 10.2 用户输入处理

| 测试项 | 输入场景 | 预期结果 |
|--------|---------|----------|
| 字符输入 | 输入 'a' | input_buffer 追加 'a' |
| Backspace | 输入 "abc" 后按 Backspace | 删除 'c'，buffer="ab" |
| Backspace 行首 | 输入 "" 后按 Backspace | 无效果 |
| Delete | 输入 "abc"，cursor 在 'b' 后，按 Delete | 删除 'c'，buffer="ab" |
| 左箭头 | 输入 "abc"，cursor 在 'c' 后，按 Left | cursor 移到 'b' 后 |
| 右箭头 | 输入 "abc"，cursor 在 'a' 前，按 Right | cursor 移到 'b' 前 |
| Home | 输入 "abc"，cursor 在 'c' 后，按 Home | cursor 移到开头 |
| End | 输入 "abc"，cursor 在 'a' 前，按 End | cursor 移到末尾 |
| Enter 空输入 | input="" 按 Enter | 无效果 |
| Enter 非空输入 | input="hello" 按 Enter | 发送消息，input 清空 |
| Ctrl+Q | 按 Ctrl+Q | 触发 Action::Quit |
| Ctrl+S | 按 Ctrl+S | 触发 Action::SwitchView(Config) |
| Esc 流式中 | 正在流式输出时按 Esc | 触发 Action::CancelStreaming |

### 10.3 状态转换

| 测试项 | Action | 预期状态变化 |
|--------|--------|--------------|
| SendMessage | 输入 "hello" 后 Enter | is_streaming=true，启动后台任务 |
| SwitchView(Config) | Ctrl+S | view_state 变为 ConfigViewState::SelectProvider |
| SwitchView(Chat) | Config 中按 ESC | view_state 恢复为 ChatViewState |
| GoBack | Config 中 ESC 或退格 | 返回 Chat，保留 provider/model |
| CancelStreaming | Esc | is_streaming=false，清除流式状态 |
| SelectProvider | Config 中选择 1-5 | current_provider 更新 |
| SaveApiKey | Config 中输入 API key | provider config 持久化 |
| SaveModel | Config 中选择模型 | provider config 持久化 |
| SaveBaseUrl | Config 中输入 URL | provider config 持久化 |
| FetchModels | Config 中按 F | 异步获取模型列表，ConfigViewState 变为 FetchingModels |

### 10.4 ConfigView 渲染

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| Provider 列表 | 显示 6 个 provider | OpenAI, Anthropic, DeepSeek, Ollama, MiniMax, Custom |
| 当前 provider 标记 | 当前是 OpenAI | OpenAI 前面有 `>` 标记 |
| 输入字段显示 | 选择编辑 API key | 显示输入框，光标闪烁 |
| 保存提示 | 编辑 API key 时 | 显示 "Enter to save, Esc to cancel" |
| 模型选择列表 | FetchModels 完成后 | 显示从 provider 获取的模型列表 |
| 快捷键提示 | 显示键盘快捷键 | 1-5 选择 provider，F 获取模型，C 自定义 |

### 10.5 Debug Panel

| 测试项 | 测试场景 | 预期结果 |
|--------|---------|----------|
| 启用方式 | 启动时 `--debug` | 创建日志文件 `~/.fastcode/ragent_{timestamp}.log` |
| 调试消息写入 | log() 调用 | 消息带时间戳写入文件 |
| 调试消息显示 | add_debug() | 最多 100 条，FIFO，超出后移除最旧 |
| 滚动导航 | DebugView 中按 Up/Down | 滚动调试面板 |
| 返回聊天 | DebugView 中按 ESC | 返回 ChatView |
| 自动滚动 | 流式输出时 | 调试面板自动滚动到最新消息 |

---

## 十一、测试场景分类汇总

### 11.1 按测试类型

| 类型 | 数量 | 说明 |
|------|------|------|
| 单元测试 | ~60 | 单个方法/组件功能验证 |
| 集成测试 | ~15 | 多组件协作测试 |
| E2E 测试 | ~10 | 完整用户场景测试 |

### 11.2 按优先级

| 优先级 | 测试项 | 说明 |
|--------|--------|------|
| P0 (必须) | ~30 | 核心路径，Agent run loop、工具执行、Provider chat |
| P1 (重要) | ~35 | 错误处理、边界情况 |
| P2 (可选) | ~20 | TUI 渲染、调试功能 |

### 11.3 按环境需求

| 环境 | 测试项 | 说明 |
|------|--------|------|
| Mock | ~40 | 不需要外部依赖，使用 Mock Provider/Tool |
| Real API | ~20 | 需要真实 API key，测试 OpenAI/Anthropic 等 |
| Local | ~10 | 需要 Ollama 等本地服务 |
| 无外部依赖 | ~15 | Session、Hooks、Skill 等纯内部模块 |

---

## 十二、自动化测试脚本对应

上述测试项将作为 YAML 测试脚本的编写依据：

```
scripts/
├── agent/
│   ├── basic_chat.yaml        # 2.4 对话操作
│   ├── tool_execution.yaml    # 2.3 工具操作
│   ├── agent_loop.yaml        # 2.5 Agent 循环
│   └── task_handling.yaml     # 2.6 任务操作
├── provider/
│   ├── openai_chat.yaml       # 4.2-4.4 OpenAI
│   ├── anthropic_chat.yaml    # 4.2-4.4 Anthropic
│   ├── deepseek_chat.yaml     # 4.2-4.4 DeepSeek (含 reasoning)
│   └── ollama_local.yaml      # 4.2-4.4 Ollama
├── tools/
│   ├── bash_tool.yaml         # 5.1 BashTool
│   ├── file_tools.yaml        # 5.2-5.4 Read/Write/Edit
│   └── grep_tool.yaml         # 5.5 GrepTool
├── orchestration/
│   ├── scheduler.yaml         # 6.1 TaskScheduler
│   └── message_bus.yaml       # 6.2 MessageBus
├── mcp/
│   └── mcp_protocol.yaml      # 7.1-7.3 MCPClient
├── hooks/
│   └── hook_dispatch.yaml     # 8.1-8.3 HookRegistry
├── skill/
│   └── skill_loading.yaml     # 9.1-9.2 SkillLoader/Registry
└── tui/
    └── chat_interface.yaml     # 10.1-10.5 TUI
```

---

**文档版本**: 1.0
**创建日期**: 2026-06-04
**更新日期**: 2026-06-04