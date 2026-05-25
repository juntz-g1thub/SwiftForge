# 架构设计与重构任务跟踪

> 文档版本: 1.0
> 生成日期: 2026-05-23
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **进行中**

---

## 概述

本文档跟踪 Rust Agent Platform 的架构设计与重构任务，作为 [平台架构与接口规范](./2026-05-23-platform-architecture.md) 的补充。

---

## 一、重构任务清单

### 1.1 任务总表

| ID | 任务 | 描述 | 优先级 | 状态 | 关联文档 |
|----|------|------|--------|------|----------|
| T1 | Log 模块 | 创建 `src/log/` 统一日志系统 | 高 | 📋 规划中 | [logging-refactoring-design](../specs/2026-05-22-logging-refactoring-design.md) |
| T2 | Debug Panel 移除 | TUI DebugPanel 移除，集成 Log 模块 | 中 | 📋 规划中 | 同上 |
| T3 | McpToolAdapter | MCP 工具适配器，实现 Tool trait | 高 | 📋 规划中 | [mcp-tool-unified-design](../specs/2026-05-23-mcp-tool-unified-design.md) |
| T4 | McpConnectionPool | MCP 连接池管理 | 高 | 📋 规划中 | 同上 |
| T5 | McpToolLoader | MCP 工具加载器 | 高 | 📋 规划中 | 同上 |
| T6 | AppController MCP 初始化 | MCP 初始化逻辑集成到 AppController | 中 | 📋 规划中 | 同上 |
| T7 | 并发工具执行 | `execute_independent_tool_calls` 实现 | 中 | 📋 规划中 | - |
| T8 | DeepSeek reasoning 累积 | reasoning_history 管理和反馈 | 高 | 📋 规划中 | - |
| T9 | Session 集成 | Session 管理器与 Agent 集成 | 中 | 📋 规划中 | - |
| T10 | Orchestration 集成 | TaskScheduler/MessageBus 与 Agent 集成 | 低 | 📋 规划中 | - |
| T11 | DeepSeek 文本标签移除 | 删除 `stream_chat_with_tools` 中的 `<thinking>`, `<content>`, `<tool_call>` 文本标签，使用标准 JSON 格式 | 中 | 📋 规划中 | - |
| T12 | TUI 分栏显示重构 | 实现深度思考、工具调用、内容的三栏分显示，支持折叠/展开交互 | 高 | 📋 规划中 | [tui-message-display-design](../specs/2026-05-25-tui-message-display-design.md) |

### 1.2 任务详情

#### T1: Log 模块

**描述**: 创建 `src/log/` 模块作为统一的日志系统

**文件**:
- `src/log/mod.rs` - Log, LogLevel, FileWriter, 全局宏
- `src/log/level.rs` - LogLevel 枚举
- `src/log/writer.rs` - FileWriter 单例

**目标**:
- 多级别支持: TRACE, DEBUG, INFO, WARN, ERROR
- 仅文件输出: `~/.fastcode/ragent.log`
- 全局宏简化: `log::info!()`, `log::debug!()` 等

**状态**: 📋 规划中

---

#### T2: Debug Panel 移除

**描述**: 移除 TUI DebugPanel，集成 Log 模块

**移除内容**:
| 文件 | 移除内容 |
|------|----------|
| `main.rs` | `--debug` 参数 |
| `tui/app_context.rs` | `debug_log_path: Option<PathBuf>` |
| `tui/app_controller.rs` | `show_debug` 参数、`log()` 方法 |
| `tui/views/chat_view.rs` | `render_debug_panel()` 调用 |
| `tui/views/debug_view.rs` | 整个文件删除 |
| `tui/state/action.rs` | `ToggleDebug`, `ScrollDebugUp`, `ScrollDebugDown` |
| `tui/state/view_state.rs` | `ViewState::Debug` variant |

**状态**: 📋 规划中

---

#### T3: McpToolAdapter

**描述**: 创建 MCP 工具适配器，将 MCP 工具适配为 `Tool` trait

**文件**: `src/integration/mcp/adapter.rs`

**核心实现**:
```rust
impl Tool for McpToolAdapter {
    async fn execute(&self, call: ToolCall) -> ToolResult {
        // 调用 mcp_client.call_tool()
        // 转换 Vec<ContentBlock> → ToolResult
    }
}
```

**状态**: 📋 规划中

---

#### T4: McpConnectionPool

**描述**: 创建 MCP 连接池，管理多个 MCP 客户端

**文件**: `src/integration/mcp/pool.rs`

**核心方法**:
- `add_server(name, url)` - 添加 MCP 服务器
- `connect(name)` - 连接服务器
- `initialize(name, client_name, version)` - 初始化
- `client(name)` - 获取客户端

**状态**: 📋 规划中

---

#### T5: McpToolLoader

**描述**: 从 MCP 服务器加载工具并注册到 ToolRegistry

**文件**: `src/integration/mcp/loader.rs`

**核心方法**:
- `load_tools(server_name) -> usize` - 加载工具数量

**状态**: 📋 规划中

---

#### T6: AppController MCP 初始化

**描述**: 修改 `AppController::new()` 集成 MCP 初始化逻辑

**流程**:
```
AppController::new()
    │
    ├── 创建 McpConnectionPool
    ├── pool.add_server("default", "http://localhost:8080")
    ├── pool.connect("default")
    ├── pool.initialize("default", "ragent", "0.1.0")
    ├── McpToolLoader::new(pool, registry)
    └── loader.load_tools("default")
```

**状态**: 📋 规划中

---

#### T7: 并发工具执行

**描述**: 实现 `execute_independent_tool_calls` 支持工具并发执行

**位置**: `core/agent.rs`

**设计**:
```rust
pub async fn execute_independent_tool_calls(
    &self,
    calls: Vec<ToolCall>
) -> Vec<ToolResult> {
    // 分析依赖关系
    // 无依赖工具并发执行
    // 有依赖工具按顺序执行
}
```

**状态**: 📋 规划中

---

#### T8: DeepSeek reasoning 累积

**描述**: 实现 reasoning_history 管理和反馈机制

**位置**: `core/agent.rs`

**字段**:
```rust
reasoning_history: Arc<Mutex<Vec<String>>>
```

**方法**:
- `add_reasoning(content: String)` - 添加
- `get_reasoning_history() -> Vec<String>` - 获取
- `clear_reasoning()` - 清空
- `format_reasoning_for_next_turn() -> Option<String>` - 格式化

**状态**: 📋 规划中

---

#### T9: Session 集成

**描述**: 将 Session 管理器与 Agent 集成

**目标**:
- 统一消息历史管理
- Context window 管理（截断/摘要）
- Agent 直接使用 Session 而非 Vec<Message>

**状态**: 📋 规划中

---

#### T10: Orchestration 集成

**描述**: TaskScheduler/MessageBus 与 Agent 集成

**目标**:
- Agent 可以调度任务
- Agent 之间可以通信

**状态**: 📋 规划中

---

#### T11: DeepSeek 文本标签移除

**描述**: 删除 `stream_chat_with_tools` 中的手动文本标签，使用标准 JSON 格式

**问题**: DeepSeekProvider 当前在流式输出时手动添加以下文本标签：
- `<thinking>...</thinking>` - 推理内容
- `<content>...</content>` - 用户可见内容
- `<tool_call>...</tool_call>` - 工具调用

**原因**: 最初设计是为了让前端能够区分不同类型的输出内容

**解决方案**: 直接使用标准 JSON 格式，通过响应中的字段区分：
- `delta.reasoning_content` → 推理内容
- `delta.content` → 用户可见内容
- `delta.tool_calls` → 工具调用数组

**修改位置**: `providers/deepseek.rs` 的 `stream_chat_with_tools` 方法

**状态**: 📋 规划中

---

#### T12: TUI 分栏显示重构

**描述**: 实现深度思考、工具调用、内容的三栏分显示

**问题**: 当前 TUI 无法区分显示深度思考、工具调用、内容三类信息

**解决方案**: 实现分栏式显示，支持折叠/展开交互

**目标**:
- 深度思考区域：紫色边框，折叠/展开功能
- 工具调用区域：青色边框，独立卡片显示
- 内容区域：正常显示

**关联设计文档**: [tui-message-display-design.md](../specs/2026-05-25-tui-message-display-design.md)

**修改位置**:
- `tui/state/view_state.rs` - ChatViewState/MessageBlock 结构
- `tui/views/chat_view.rs` - render_reasoning_block/render_tool_call_block 方法

**状态**: 📋 规划中

---

## 二、模块讨论进度

### 2.1 状态汇总

| 模块 | 状态 | 结论 |
|------|------|------|
| TUI Frontend | ⚠️ 改进中 | MVC重构完成，DebugPanel由Log替代 |
| Provider 接口 | ⚠️ 部分解决 | DeepSeek解析已实现，reasoning待累积 |
| Agent Loop | ⚠️ 待改进 | 并发工具执行、reasoning累积 |
| Session 管理 | ❌ 未解决 | 需要设计context window管理方案 |
| Tool System | ⚠️ 改进中 | MCP统一架构设计中 |
| Platform | ✅ 已实现 | IntentGate/Hooks/Skill/Boulder完整 |
| Orchestration | ❌ 未解决 | TaskScheduler/MessageBus未使用 |
| MCP Client | ⚠️ 改进中 | 协议已实现，适配层设计中 |
| Log Module | 📋 规划中 | 替代DebugPanel，统一日志 |

### 2.2 详细进度

#### TUI Frontend

**当前状态**: MVC架构重构完成

**已完成**:
- ✅ AppController 中心控制器
- ✅ View trait + ChatView/ConfigView
- ✅ AppContext/UIState/ViewState 分离

**待完成**:
- 📋 DebugPanel 移除 (T2)
- 📋 Log 模块集成 (T1)
- 📋 分栏显示重构 (T12)

**结论**: MVC重构完成，DebugPanel由Log替代

---

#### Provider 接口

**当前状态**: ⚠️ 部分解决

**已完成**:
- ✅ LLMProvider/ToolCallingProvider Trait 定义
- ✅ OpenAI/Anthropic/DeepSeek/Ollama 实现
- ✅ DeepSeek `<tool_call>` 标签解析

**待完成**:
- 📋 reasoning_content 累积机制 (T8)
- 📋 DeepSeek reasoning 反馈
- 📋 DeepSeek 文本标签移除 (T11)

**结论**: DeepSeek解析已实现，reasoning待累积

---

#### Agent Loop

**当前状态**: ⚠️ 待改进

**待完成**:
- 📋 并发工具执行 (T7)
- 📋 reasoning 累积 (T8)

**结论**: 并发工具执行、reasoning累积

---

#### Session 管理

**当前状态**: ❌ 未解决

**问题**:
- Session 定义但未使用
- Agent 直接持有 Vec<Message>
- 无 context window 管理

**待完成**:
- 📋 Session 集成 (T9)

**结论**: 需要设计context window管理方案

---

#### Tool System

**当前状态**: ⚠️ 改进中

**已完成**:
- ✅ Tool trait 定义
- ✅ ToolRegistry 注册机制
- ✅ 5个内置工具

**待完成**:
- 📋 MCP 统一架构 (T3, T4, T5, T6)

**结论**: MCP统一架构设计中

---

#### Platform

**当前状态**: ✅ 已实现

**模块**:
- ✅ IntentGate - 意图分类路由
- ✅ Hooks - 52个生命周期钩子
- ✅ Skill - SKILL.md 加载注册
- ✅ Boulder - TODO 持久化

**结论**: IntentGate/Hooks/Skill/Boulder完整

---

#### Orchestration

**当前状态**: ❌ 未解决

**问题**:
- TaskScheduler 定义但未使用
- MessageBus 定义但未使用

**待完成**:
- 📋 Orchestration 集成 (T10)

**结论**: TaskScheduler/MessageBus未使用

---

#### MCP Client

**当前状态**: ⚠️ 改进中

**已完成**:
- ✅ protocol.rs - JSON-RPC 2.0 协议
- ✅ client.rs - HTTP POST 客户端

**待完成**:
- 📋 McpToolAdapter (T3)
- 📋 McpConnectionPool (T4)
- 📋 McpToolLoader (T5)

**结论**: 协议已实现，适配层设计中

---

#### Log Module

**当前状态**: 📋 规划中

**目标**:
- 替代 DebugPanel
- 统一日志系统
- 多级别支持

**待完成**:
- 📋 Log 模块实现 (T1)

**结论**: 替代DebugPanel，统一日志

---

## 三、里程碑

### M1: Log 模块上线
- T1 Log 模块创建
- T2 DebugPanel 移除

### M2: MCP 工具统一
- T3 McpToolAdapter
- T4 McpConnectionPool
- T5 McpToolLoader
- T6 AppController MCP 初始化

### M3: Agent Loop 增强
- T7 并发工具执行
- T8 DeepSeek reasoning 累积

### M4: 集成完善
- T9 Session 集成
- T10 Orchestration 集成

---

## 四、变更记录

| 日期 | 版本 | 变更内容 |
|------|------|----------|
| 2026-05-23 | 1.0 | 初始版本，任务清单和讨论进度从主架构文档分离 |

---

*文档状态: 进行中*