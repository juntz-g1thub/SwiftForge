# 静态库拆分重构详细实施计划

> 版本: 0.1.0
> 日期: 2026-05-28
> 状态: ✅ 已完成（2026-05-28）

---

## 阶段一：基础设施搭建

### Phase 1.1: 创建 Workspace 结构

**目标**: 创建完整的 workspace 配置

**步骤**:

1. **备份现有 Cargo.toml**
   ```bash
   cp rust-agent-platform/Cargo.toml rust-agent-platform/Cargo.toml.bak
   ```

2. **创建 libs/ 目录结构**
   ```bash
   mkdir -p libs/rust-agent-types/src
   mkdir -p libs/rust-orchestration/src
   mkdir -p libs/rust-tools/src
   mkdir -p libs/rust-mcp-client/src
   mkdir -p libs/rust-agent-hooks/src
   mkdir -p libs/rust-skill-loader/src
   ```

3. **创建 Workspace 根 Cargo.toml** (`Cargo.toml`)
   ```toml
   [workspace]
   members = [
       "libs/rust-agent-types",
       "libs/rust-orchestration",
       "libs/rust-tools",
       "libs/rust-mcp-client",
       "libs/rust-agent-hooks",
       "libs/rust-skill-loader",
       "rust-agent-platform",
   ]
   resolver = "2"

   [workspace.package]
   version = "0.1.0"
   edition = "2021"
   authors = ["Developer <dev@example.com>"]
   license = "MIT OR Apache-2.0"

   [workspace.dependencies]
   tokio = { version = "1", features = ["full"] }
   tokio-stream = "0.1"
   futures = "0.3"
   tracing = "0.1"
   tracing-subscriber = "0.3"
   anyhow = "1.0"
   thiserror = "1.0"
   async-trait = "0.1"
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   rusqlite = { version = "0.31", features = ["bundled"] }
   ```

**Checkpoint**: `cargo metadata --format-version 1` 成功解析 workspace

---

### Phase 1.2: 创建 rust-agent-types

**目标**: 创建通用类型库，所有其他库的基础

**步骤**:

1. **创建 `libs/rust-agent-types/Cargo.toml`**
   ```toml
   [package]
   name = "rust-agent-types"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   async-trait.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   ```

2. **创建 `libs/rust-agent-types/src/lib.rs`**
   ```rust
   pub mod message;
   pub mod tool;
   pub mod provider;
   pub mod session;

   pub use message::{Message, Session, SessionConfig};
   pub use tool::{Tool, ToolCall, ToolResult, ToolDefinition, ToolRegistry};
   pub use provider::{Provider, ProviderConfig, ModelResponse, Usage};
   pub use session::{Session as ChatSession, SessionConfig as ChatSessionConfig};
   ```

3. **创建 `libs/rust-agent-types/src/message.rs`** (从 `rust-agent-platform/src/core/session.rs` 迁移)
   ```rust
   use serde::{Deserialize, Serialize};

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct SessionConfig {
       pub context_window: usize,
       pub max_tokens: Option<usize>,
   }

   impl Default for SessionConfig {
       fn default() -> Self {
           Self {
               context_window: 100,
               max_tokens: None,
           }
       }
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Message {
       pub role: String,
       pub content: String,
   }

   // ... 其余内容从 session.rs 迁移
   ```

4. **创建 `libs/rust-agent-types/src/tool.rs`** (从 `rust-agent-platform/src/core/tool.rs` 迁移)
   - 迁移 `Tool`, `ToolCall`, `ToolResult`, `ToolDefinition`, `ToolRegistry`
   - 保留 `async_trait` 依赖

5. **创建 `libs/rust-agent-types/src/provider.rs`** (从 `rust-agent-platform/src/core/provider.rs` 迁移)
   - 迁移 `Provider`, `ProviderConfig`, `ModelResponse`, `Usage`
   - 保留 `async_trait` 依赖

6. **创建 `libs/rust-agent-types/src/session.rs`** (从 `rust-agent-platform/src/core/session.rs` 迁移)
   - 迁移 `Session` 结构体及其方法

**Checkpoint**: `cd libs/rust-agent-types && cargo build` 成功

**Rollback**: 删除 `libs/rust-agent-types/` 目录，恢复 Cargo.toml

---

### Phase 1.3: 创建 rust-orchestration

**目标**: 创建任务调度和消息总线库

**步骤**:

1. **创建 `libs/rust-orchestration/Cargo.toml`**
   ```toml
   [package]
   name = "rust-orchestration"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   rust-agent-types = { path = "../rust-agent-types" }
   tokio.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   ```

2. **创建 `libs/rust-orchestration/src/lib.rs`**
   ```rust
   pub mod scheduler;
   pub mod message_bus;

   pub use scheduler::{TaskScheduler, Task, TaskPriority, TaskStatus};
   pub use message_bus::{MessageBus, AgentMessage, MessageHandler};
   ```

3. **迁移 `libs/rust-orchestration/src/scheduler.rs`**
   - 从 `rust-agent-platform/src/orchestration/scheduler.rs` 迁移
   - 更新 `use crate::...` → 使用 `rust_agent_types::`

4. **迁移 `libs/rust-orchestration/src/message_bus.rs`**
   - 从 `rust-agent-platform/src/orchestration/message_bus.rs` 迁移

**Checkpoint**: `cd libs/rust-orchestration && cargo build` 成功

---

## 阶段二：工具和集成库

### Phase 2.1: 创建 rust-tools

**目标**: 创建内置工具库

**步骤**:

1. **创建 `libs/rust-tools/Cargo.toml`**
   ```toml
   [package]
   name = "rust-tools"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   rust-agent-types = { path = "../rust-agent-types" }
   async-trait.workspace = true
   serde_json.workspace = true
   regex = "1"
   ```

2. **创建 `libs/rust-tools/src/lib.rs`**
   ```rust
   pub mod bash;
   pub mod read;
   pub mod write;
   pub mod edit;
   pub mod grep;

   pub use bash::BashTool;
   pub use read::ReadTool;
   pub use write::WriteTool;
   pub use edit::EditTool;
   pub use grep::GrepTool;
   ```

3. **迁移各工具文件**
   - `bash.rs` → `libs/rust-tools/src/bash.rs`
   - `read.rs` → `libs/rust-tools/src/read.rs`
   - `write.rs` → `libs/rust-tools/src/write.rs`
   - `edit.rs` → `libs/rust-tools/src/edit.rs`
   - `grep.rs` → `libs/rust-tools/src/grep.rs`

4. **更新导入**: `use crate::core::Tool` → `use rust_agent_types::Tool`

**Checkpoint**: `cd libs/rust-tools && cargo build` 成功

---

### Phase 2.2: 创建 rust-mcp-client

**目标**: 创建 MCP 客户端库

**步骤**:

1. **创建 `libs/rust-mcp-client/Cargo.toml`**
   ```toml
   [package]
   name = "rust-mcp-client"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   rust-agent-types = { path = "../rust-agent-types" }
   reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
   tokio.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   anyhow.workspace = true
   ```

2. **创建 `libs/rust-mcp-client/src/lib.rs`**
   ```rust
   pub mod protocol;
   pub mod client;

   pub use protocol::*;
   pub use client::MCPClient;
   ```

3. **迁移协议文件**
   - `protocol.rs` → `libs/rust-mcp-client/src/protocol.rs`
   - `client.rs` → `libs/rust-mcp-client/src/client.rs`

4. **更新导入**: 移除对 `crate::core` 的依赖，改用 `rust_agent_types::Message` 等

**Checkpoint**: `cd libs/rust-mcp-client && cargo build` 成功

---

## 阶段三：平台特性库

### Phase 3.1: 创建 rust-agent-hooks

**目标**: 创建事件钩子库

**前置**: 需要先分析 `platform/hooks/types.rs` 中的 `HookContext` 依赖

**步骤**:

1. **分析 HookContext 依赖**
   - 读取 `rust-agent-platform/src/platform/hooks/types.rs`
   - 确定 `HookContext` 是否包含应用特定类型
   - 如有，抽象化为泛型或移除

2. **创建 `libs/rust-agent-hooks/Cargo.toml`**
   ```toml
   [package]
   name = "rust-agent-hooks"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   rust-agent-types = { path = "../rust-agent-types" }
   tokio.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   ```

3. **创建 `libs/rust-agent-hooks/src/lib.rs`**
   ```rust
   pub mod types;
   pub mod registry;

   pub use types::{HookEvent, HookContext};
   pub use registry::{HookRegistry, HookFn};
   ```

4. **迁移 `types.rs` 和 `registry.rs`**
   - 抽象化 `HookContext` 中的应用特定类型

**Checkpoint**: `cd libs/rust-agent-hooks && cargo build` 成功

---

### Phase 3.2: 创建 rust-skill-loader

**目标**: 创建技能加载器库

**前置**: `IntentCategory` 需解耦

**步骤**:

1. **分析 SkillLoader 依赖**
   - 读取 `rust-agent-platform/src/platform/skill/loader.rs`
   - 确定 `SkillLoader` 如何使用 `IntentCategory`
   - 设计字符串路径或泛型方案

2. **创建 `libs/rust-skill-loader/Cargo.toml`**
   ```toml
   [package]
   name = "rust-skill-loader"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   rust-agent-types = { path = "../rust-agent-types" }
   tokio.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   anyhow.workspace = true
   dirs = "5"
   ```

3. **创建 `libs/rust-skill-loader/src/lib.rs`**
   ```rust
   pub mod loader;
   pub mod registry;

   pub use loader::{Skill, SkillLoader, SkillScope};
   pub use registry::SkillRegistry;
   ```

4. **迁移 `loader.rs` 和 `registry.rs`**
   - `IntentCategory` 改为字符串路径或配置注入

**Checkpoint**: `cd libs/rust-skill-loader && cargo build` 成功

---

## 阶段四：主应用重构

### Phase 4.1: 更新 rust-agent-platform Cargo.toml

**目标**: 配置主应用依赖新库

**步骤**:

1. **更新 `rust-agent-platform/Cargo.toml`**
   ```toml
   [package]
   name = "rust-agent-platform"
   version.workspace = true
   edition.workspace = true

   [lib]
   name = "rust_agent_platform"
   path = "src/lib.rs"

   [[bin]]
   name = "ragent"
   path = "src/main.rs"

   [dependencies]
   # 新增库依赖
   rust-agent-types = { path = "../libs/rust-agent-types" }
   rust-orchestration = { path = "../libs/rust-orchestration" }
   rust-tools = { path = "../libs/rust-tools" }
   rust-mcp-client = { path = "../libs/rust-mcp-client" }
   rust-agent-hooks = { path = "../libs/rust-agent-hooks" }
   rust-skill-loader = { path = "../libs/rust-skill-loader" }

   # 保留原有依赖（已移除已迁移库的间接依赖）
   tokio.workspace = true
   tokio-stream.workspace = true
   futures.workspace = true
   tracing.workspace = true
   tracing-subscriber.workspace = true
   anyhow.workspace = true
   thiserror.workspace = true
   async-trait.workspace = true
   serde.workspace = true
   serde_json.workspace = true
   rusqlite.workspace = true

   ratatui = "0.26"
   crossterm = "0.27"
   pulldown-cmark = "0.10"
   reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
   dirs = "5"
   regex = "1"
   clap = { version = "4", features = ["derive"] }
   chrono = "0.4"
   ```

---

### Phase 4.2: 重构 core/ 模块

**目标**: 精简 core/ 目录，仅保留 Agent 相关

**步骤**:

1. **分析 `core/mod.rs`**
   - 读取 `rust-agent-platform/src/core/mod.rs`
   - 确定哪些类型已迁移到 `rust-agent-types`

2. **重构 `rust-agent-platform/src/core/mod.rs`**
   ```rust
   mod agent;
   // 移除 tool, session, provider - 已迁移

   pub use agent::{Agent, AgentConfig, AgentRole};
   // 移除其他 pub use - 已迁移到 rust-agent-types
   ```

3. **重构 `rust-agent-platform/src/core/agent.rs`**
   - 更新导入: `crate::core::Tool` → `rust_agent_types::Tool`
   - 更新导入: `crate::providers` → 保持不变（仍在主应用中）
   - 更新导入: `crate::orchestration` → `rust_orchestration`
   - 更新导入: `crate::core::session::Message` → `rust_agent_types::Message`

4. **删除已迁移文件**
   ```bash
   rm rust-agent-platform/src/core/tool.rs
   rm rust-agent-platform/src/core/session.rs
   rm rust-agent-platform/src/core/provider.rs
   ```

---

### Phase 4.3: 删除已迁移的模块目录

**目标**: 清理主应用中的重复代码

**步骤**:

1. **删除 orchestration/ 目录**
   ```bash
   rm -rf rust-agent-platform/src/orchestration/
   ```

2. **删除 tools/ 目录**
   ```bash
   rm -rf rust-agent-platform/src/tools/
   ```

3. **删除 integration/mcp/ 目录**
   ```bash
   rm -rf rust-agent-platform/src/integration/
   ```

4. **删除 platform/hooks/ 目录**
   ```bash
   rm -rf rust-agent-platform/src/platform/hooks/
   ```

5. **删除 platform/skill/ 目录**
   ```bash
   rm -rf rust-agent-platform/src/platform/skill/
   ```

---

### Phase 4.4: 更新 platform/mod.rs

**目标**: 更新 platform 模块声明

**步骤**:

1. **编辑 `rust-agent-platform/src/platform/mod.rs`**
   ```rust
   mod intent_gate;
   mod category;
   pub mod hooks;     // 改为引用新库
   pub mod skill;      // 改为引用新库
   pub mod boulder;
   pub mod boulder_db;

   pub use intent_gate::IntentGate;
   pub use category::IntentCategory;

   // 重新导出 hooks 和 skill 从新库
   pub use rust_agent_hooks::{HookRegistry, HookEvent, HookContext, HookFn};
   pub use rust_skill_loader::{SkillLoader, Skill, SkillScope, SkillRegistry};
   ```

---

### Phase 4.5: 更新 lib.rs

**目标**: 更新库导出

**步骤**:

1. **编辑 `rust-agent-platform/src/lib.rs`**
   ```rust
   pub mod core;
   pub mod providers;
   pub mod tui;
   pub mod platform;
   // pub mod integration;  // 已删除
   // pub mod orchestration; // 已删除
   // pub mod tools;         // 已删除

   pub use core::{Agent, AgentConfig, AgentRole};
   pub use rust_agent_types::{Message, Tool, ToolResult, Session, Provider};
   ```

---

### Phase 4.6: 更新 tui/app_controller.rs

**目标**: 更新 TUI 控制器的导入

**步骤**:

1. **分析 `tui/app_controller.rs` 的导入**
   - 识别所有 `crate::core::`, `crate::tools::`, `crate::orchestration::` 引用
   - 更新为新的库路径

2. **典型更新模式**:
   ```rust
   // 旧
   use crate::core::{Tool, ToolRegistry};
   use crate::tools::{BashTool, ReadTool, WriteTool, EditTool, GrepTool};
   use crate::orchestration::{TaskScheduler, MessageBus};

   // 新
   use rust_agent_types::{Tool, ToolRegistry};
   use rust_tools::{BashTool, ReadTool, WriteTool, EditTool, GrepTool};
   use rust_orchestration::{TaskScheduler, MessageBus};
   ```

---

## 阶段五：验证和测试

### Phase 5.1: Workspace 级编译验证

**步骤**:

1. **运行 workspace 编译**
   ```bash
   cargo build --workspace
   ```

2. **修复所有编译错误**
   - 导入路径错误
   - 缺失依赖
   - API 不匹配

**Checkpoint**: `cargo build --workspace` 无错误

---

### Phase 5.2: 二进制编译验证

**步骤**:

1. **运行主二进制编译**
   ```bash
   cargo build --bin ragent
   ```

2. **修复所有链接错误**

**Checkpoint**: `cargo build --bin ragent` 成功

---

### Phase 5.3: 运行测试

**步骤**:

1. **运行所有测试**
   ```bash
   cargo test --workspace
   ```

2. **修复测试失败**

**Checkpoint**: `cargo test --workspace` 全部通过

---

### Phase 5.4: Clippy 检查

**步骤**:

1. **运行 clippy**
   ```bash
   cargo clippy --workspace -- -D warnings
   ```

2. **修复警告**

**Checkpoint**: 无 clippy 警告

---

## 阶段六：最终验证

### Phase 6.1: 验收标准检查

- [ ] `cargo build --bin ragent` 成功编译
- [ ] `cargo test --workspace` 所有测试通过
- [ ] 无循环依赖警告 (`cargo tree --workspace -d` 检查)
- [ ] 每个库可独立编译 (`cargo build -p <package-name>`)
- [ ] 主应用体积显著减小

### Phase 6.2: 提交变更

**步骤**:

1. **Git add 所有变更**
   ```bash
   git add -A
   ```

2. **创建 commit**
   ```bash
   git commit -m "refactor: extract libraries into workspace structure

   - rust-agent-types: core types (Message, Tool, Provider, etc.)
   - rust-orchestration: TaskScheduler, MessageBus
   - rust-tools: BashTool, ReadTool, WriteTool, EditTool, GrepTool
   - rust-mcp-client: MCP JSON-RPC client
   - rust-agent-hooks: event hook system
   - rust-skill-loader: SKILL.md loader

   BREAKING CHANGE: several internal types moved to separate crates"
   ```

---

## 风险缓解策略

### 风险 1: 循环依赖
**检测**: `cargo tree --workspace -d`
**缓解**: 确保依赖方向为 单向：应用 → 库 → rust-agent-types

### 风险 2: API 不兼容
**检测**: 每个 Phase 后运行 `cargo build`
**缓解**: 小步前进，每步验证

### 风险 3: IntentCategory 耦合
**检测**: 编译 `rust-skill-loader` 时检查
**缓解**: 使用字符串路径替代枚举

### 风险 4: Trait 对象传递
**检测**: `cargo build` 时 dyn Trait 相关错误
**缓解**: 确保所有 trait 是 `Send + Sync`

### 风险 5: HookContext 应用类型耦合 (新增)
**问题**: `HookContext` 可能包含应用特定类型（如 AgentId, SessionId）
**缓解**: 定义最小化 `HookContext`，仅包含：
```rust
pub struct HookContext {
    pub timestamp: std::time::Instant,
    pub session_id: Option<String>,
    // 无应用特定类型
}
```
主应用如需更丰富上下文，可自行包装。

### 风险 6: ToolRegistry 所有权 (新增)
**问题**: `ToolCallingProvider` 需要 tool definitions，但 tools 在 `rust-tools` 中
**缓解**: 在 `rust-tools` 中提供 helper：
```rust
pub fn default_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    r.register(BashTool::new());
    r.register(ReadTool::new());
    // ...
    r
}
```

### 风险 7: AgentMessage::from/to 类型安全 (新增)
**问题**: 使用 `String` 类型，未来扩展困难（如 agent groups, wildcards）
**缓解**: 定义 `AgentId` 类型或使用泛型：
```rust
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);
```

---

## 附加测试策略

### 测试基础设施缺失 (高优先级)

需要为 `rust-agent-types` 添加测试工具：

```rust
// 在 rust-agent-types/tests/ 或 src/test_utils.rs
pub mod test_utils {
    pub fn create_mock_registry() -> ToolRegistry { ... }
    pub fn create_mock_provider() -> Arc<dyn LLMProvider> { ... }
    pub fn create_mock_message(content: &str) -> Message { ... }
}
```

### 测试覆盖要求

| 库 | 单元测试 | 集成测试 | 文档测试 |
|----|---------|---------|---------|
| rust-agent-types | ✓ Message, Tool 等 | ✓ 跨 trait 交互 | 示例 |
| rust-orchestration | ✓ TaskScheduler | ✓ 与 Agent 交互 | - |
| rust-tools | ✓ 各 Tool | ✓ ToolRegistry | - |
| rust-mcp-client | ✓ protocol types | ✓ MCPClient | - |
| rust-agent-hooks | ✓ HookRegistry | ✓ 事件触发 | - |
| rust-skill-loader | ✓ SkillLoader | ✓ 文件加载 | - |

---

## 回滚计划

如遇重大问题无法解决：

1. **恢复到 Phase 1 之前**:
   ```bash
   git stash
   git checkout main
   ```

2. **保留备份**:
   ```bash
   # Cargo.toml.bak 已备份
   cp -r rust-agent-platform rust-agent-platform.refactor.bak
   ```

3. **评估方案**:
   - 如问题可修复，继续
   - 如问题严重，回滚并丢弃 `libs/` 目录

---

## 实施结果总结

### 实际完成结构

```
libs/
├── swiftforge-types/      ✅ Message, Session, Tool, Provider, ToolRegistry 等
├── swiftforge-task/       ✅ TaskScheduler, MessageBus, Task, AgentMessage
├── swiftforge-tools/      ✅ BashTool, ReadTool, WriteTool, EditTool, GrepTool
├── swiftforge-mcp/        ✅ MCPClient, JsonRpc protocol types
├── swiftforge-hooks/       ✅ HookRegistry, HookEvent, HookContext
└── swiftforge-skill/       ✅ SkillLoader, Skill, SkillRegistry

swiftforge/                 ✅ 主应用（TUI + Agent + Providers）
```

### 重命名说明

原计划 crate 名为 `rust-*`，实际执行为 `swiftforge-*`：
- `rust-agent-types` → `swiftforge-types`
- `rust-orchestration` → `swiftforge-task`
- `rust-tools` → `swiftforge-tools`
- `rust-mcp-client` → `swiftforge-mcp`
- `rust-agent-hooks` → `swiftforge-hooks`
- `rust-skill-loader` → `swiftforge-skill`

### 验证

```bash
cargo build --bin ragent  # ✅ Finished in 4.58s
```

### 注意事项

1. 首次在主分支执行导致工作流违规，后迁移到 worktree 重新执行
2. message_bus.rs 因编码问题需用 printf 重新写入
3. 所有导入路径需同步更新为新的 crate 名