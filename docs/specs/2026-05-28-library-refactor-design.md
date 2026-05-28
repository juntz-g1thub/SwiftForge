# 静态库拆分重构实施计划

> 版本: 0.1.0
> 日期: 2026-05-28
> 状态: 设计中

---

## 一、目标

将 `rust-agent-platform` 拆分为一个 Cargo workspace，包含一个主应用和多个独立静态库：

| 库 | 职责 |
|----|------|
| `rust-agent-types` | 通用类型定义（Message, Tool trait, Provider trait 等） |
| `rust-orchestration` | 任务调度（TaskScheduler）和消息总线（MessageBus） |
| `rust-tools` | 内置工具集（BashTool, ReadTool, WriteTool, EditTool, GrepTool） |
| `rust-mcp-client` | MCP JSON-RPC 客户端 |
| `rust-agent-hooks` | 52 钩子事件系统 |
| `rust-skill-loader` | SKILL.md 技能加载器 |
| `rust-agent-platform` | 主应用（TUI + Agent + Providers） |

---

## 二、Workspace 结构

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "rust-agent-types",
    "rust-orchestration",
    "rust-tools",
    "rust-mcp-client",
    "rust-agent-hooks",
    "rust-skill-loader",
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
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

---

## 三、库详细设计

### 3.1 rust-agent-types

**路径**: `libs/rust-agent-types/`

**Cargo.toml**:
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

**源代码结构**:
```
src/
├── lib.rs           # 导出所有类型
├── message.rs      # Message, Session, SessionConfig
├── tool.rs         # Tool trait, ToolCall, ToolResult, ToolDefinition, ToolRegistry
├── provider.rs     # Provider trait, ModelResponse, Usage
└── session.rs       # Session, SessionConfig
```

**导出**:
```rust
pub use message::{Message, Session, SessionConfig};
pub use tool::{Tool, ToolCall, ToolResult, ToolDefinition, ToolRegistry, ToolRegistryExt};
pub use provider::{Provider, ProviderConfig, ModelResponse, Usage};
pub use session::{Session, SessionConfig};
```

---

### 3.2 rust-orchestration

**路径**: `libs/rust-orchestration/`

**Cargo.toml**:
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

**源代码结构**:
```
src/
├── lib.rs
├── scheduler.rs    # TaskScheduler, Task, TaskPriority, TaskStatus
└── message_bus.rs  # MessageBus, AgentMessage, MessageHandler
```

**关键设计**:
- `AgentMessage::from`, `AgentMessage::to` 使用 `String` 类型（可通过泛型抽象但不迫切）

---

### 3.3 rust-tools

**路径**: `libs/rust-tools/`

**Cargo.toml**:
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

**源代码结构**:
```
src/
├── lib.rs
├── bash.rs   # BashTool
├── read.rs   # ReadTool
├── write.rs  # WriteTool
├── edit.rs   # EditTool
└── grep.rs   # GrepTool
```

---

### 3.4 rust-mcp-client

**路径**: `libs/rust-mcp-client/`

**Cargo.toml**:
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

**源代码结构**:
```
src/
├── lib.rs
├── protocol.rs  # JsonRpcRequest, JsonRpcResponse, ServerCapabilities, Tool
└── client.rs   # MCPClient
```

---

### 3.5 rust-agent-hooks

**路径**: `libs/rust-agent-hooks/`

**Cargo.toml**:
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

**源代码结构**:
```
src/
├── lib.rs
├── types.rs    # HookEvent, HookContext
└── registry.rs # HookRegistry, HookFn
```

**注意**: 需要抽象掉原 `platform/hooks/types.rs` 中对主应用类型的引用

---

### 3.6 rust-skill-loader

**路径**: `libs/rust-skill-loader/`

**Cargo.toml**:
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

**源代码结构**:
```
src/
├── lib.rs
├── loader.rs   # Skill, SkillLoader, SkillScope
└── registry.rs # SkillRegistry
```

**注意**: `IntentCategory` 枚举应保留在主应用中，`SkillLoader` 依赖改为字符串路径

---

### 3.7 rust-agent-platform（主应用）

**重构后 Cargo.toml**:
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
rust-agent-types = { path = "../rust-agent-types" }
rust-orchestration = { path = "../rust-orchestration" }
rust-tools = { path = "../rust-tools" }
rust-mcp-client = { path = "../rust-mcp-client" }
rust-agent-hooks = { path = "../rust-agent-hooks" }
rust-skill-loader = { path = "../rust-skill-loader" }

# 原有依赖（移除已提取库的间接依赖）
tokio.workspace = true
anyhow.workspace = true
# ... 其他依赖保留
```

**保留的 src/ 结构**:
```
src/
├── lib.rs
├── main.rs
├── core/           # 精简：仅 Agent, AgentConfig, AgentRole
├── providers/      # OpenAI, Anthropic, Ollama, DeepSeek, MiniMax, Custom
├── tui/            # 完整 TUI 层
└── platform/
    ├── mod.rs
    ├── intent_gate.rs
    ├── category.rs
    ├── boulder.rs
    └── boulder_db.rs
```

**移除**:
- `orchestration/` → 已迁移到 `rust-orchestration`
- `tools/` → 已迁移到 `rust-tools`
- `integration/mcp/` → 已迁移到 `rust-mcp-client`
- `platform/hooks/` → 已迁移到 `rust-agent-hooks`
- `platform/skill/` → 已迁移到 `rust-skill-loader`

---

## 四、实施步骤

### Step 1: 创建 rust-agent-types

1. 创建 `libs/rust-agent-types/` 目录
2. 编写 `Cargo.toml`
3. 从 `rust-agent-platform/src/core/` 迁移类型定义：
   - `message.rs` → `Message`, `Session`, `SessionConfig`
   - `tool.rs` → `Tool`, `ToolCall`, `ToolResult`, `ToolDefinition`, `ToolRegistry`
   - `provider.rs` → `Provider`, `ProviderConfig`, `ModelResponse`, `Usage`
4. 更新主应用 Cargo workspace 配置

### Step 2: 创建 rust-orchestration

1. 创建 `libs/rust-orchestration/`
2. 从 `rust-agent-platform/src/orchestration/` 迁移代码
3. 更新 `Cargo.toml`，添加对 `rust-agent-types` 的依赖
4. 修复跨库依赖错误

### Step 3: 创建 rust-tools

1. 创建 `libs/rust-tools/`
2. 从 `rust-agent-platform/src/tools/` 迁移代码
3. 更新导入，依赖 `rust-agent-types`
4. 验证编译

### Step 4: 创建 rust-mcp-client

1. 创建 `libs/rust-mcp-client/`
2. 从 `rust-agent-platform/src/integration/mcp/` 迁移代码
3. 更新导入，依赖 `rust-agent-types`
4. 验证编译

### Step 5: 创建 rust-agent-hooks

1. 创建 `libs/rust-agent-hooks/`
2. 从 `rust-agent-platform/src/platform/hooks/` 迁移代码
3. 抽象化 `HookContext` 中对应用类型的引用
4. 验证编译

### Step 6: 创建 rust-skill-loader

1. 创建 `libs/rust-skill-loader/`
2. 从 `rust-agent-platform/src/platform/skill/` 迁移代码
3. `IntentCategory` 改为字符串或泛型处理
4. 验证编译

### Step 7: 重构 rust-agent-platform 主应用

1. 更新 `rust-agent-platform/Cargo.toml`，添加新库依赖
2. 精简 `core/`：仅保留 Agent, AgentConfig, AgentRole
3. 删除已迁移的模块目录
4. 更新所有导入路径
5. 运行 `cargo build --bin ragent` 验证

### Step 8: Workspace 配置

1. 更新 workspace 根 `Cargo.toml`，添加所有成员
2. 更新 workspace 依赖版本
3. 运行 `cargo build` 验证整个 workspace

---

## 五、风险与注意事项

1. **循环依赖风险**: `rust-orchestration` 不应依赖主应用，主应用依赖 orchestration
2. **Trait 对象传递**: 各库通过 `Arc<dyn Trait>` 交互，确保 trait 是 `Send + Sync`
3. **IntentCategory 耦合**: `rust-skill-loader` 原依赖 `platform/category.rs`，重构后需解耦
4. **测试策略**: 每创建一个库，应有对应的单元测试验证

---

## 六、验收标准

1. `cargo build --bin ragent` 成功编译
2. `cargo test` 所有测试通过
3. 无循环依赖警告
4. 每个库可独立发布到 crates.io（如需）
5. 主应用体积显著减小