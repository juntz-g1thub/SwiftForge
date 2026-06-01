# MCP 工具统一架构设计

> 文档版本: 1.0
> 生成日期: 2026-06-01
> 分支: feat/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **已实现**

---

## 概述

**功能系统**: Tool System (工具系统) — MCP 工具统一架构

**设计目标**: 将 MCP 工具通过适配层统一接入 ToolRegistry，与内置工具共存，形成统一的工具调用入口。

**决策总结**:
- 位置：扩展现有 `swiftforge-mcp` 库
- 命名：多服务器 + 前缀 `{server_name}_{tool_name}`
- 错误处理：快速失败（返回 ToolResult error）
- 初始化：启动时异步连接，不阻塞 TUI

---

## 一、架构设计

### 1.1 整体架构

```
swiftforge-mcp 库结构：
src/
├── client.rs      # 已存在：MCPClient 单连接
├── protocol.rs    # 已存在：JSON-RPC 类型定义
├── adapter.rs     # 新增：McpToolAdapter
├── pool.rs        # 新增：McpConnectionPool
├── loader.rs      # 新增：McpToolLoader
└── lib.rs         # 更新：导出新模块
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         AppController                                    │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ ToolRegistry (统一工具入口)                                       │  │
│  │  ┌─────────────────┐     ┌─────────────────────────────────────┐ │  │
│  │  │  Built-in Tools  │     │     MCP Tools (via Adapter)         │ │  │
│  │  │  • BashTool     │     │  McpToolAdapter ← MCPClient          │ │  │
│  │  │  • ReadTool     │     │       └─► tools/list                 │ │  │
│  │  │  • WriteTool    │     │       └─► tools/call                 │ │  │
│  │  │  • EditTool     │     │                                      │ │  │
│  │  │  • GrepTool     │     │  Vec<McpToolAdapter>                 │ │  │
│  │  └─────────────────┘     └─────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 工具命名

**格式**: `{server_name}_{tool_name}`

**示例**:
- 服务器名 `mcp`，工具名 `read_file` → 注册为 `mcp_read_file`
- 服务器名 `fs`，工具名 `write` → 注册为 `fs_write`

**理由**:
1. 支持同时连接多台 MCP 服务器
2. 工具名前缀可以区分来源，便于调试
3. 避免不同服务器的同名工具冲突

---

## 二、核心组件设计

### 2.1 McpToolAdapter

将 MCP 服务器上的工具适配为本地 `Tool` trait：

```rust
// src/adapter.rs

use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use swiftforge_types::{Tool, ToolCall, ToolResult};
use crate::client::MCPClient;

pub struct McpToolAdapter {
    full_name: String,          // 注册名，如 "mcp_read_file"
    mcp_name: String,           // 原始名，如 "read_file"
    description: String,
    input_schema: JsonValue,
    mcp_client: Arc<MCPClient>,
}

impl McpToolAdapter {
    pub fn new(
        server_name: &str,
        mcp_client: Arc<MCPClient>,
        name: String,
        description: String,
        input_schema: JsonValue,
    ) -> Self {
        let full_name = format!("{}_{}", server_name, name);
        Self {
            full_name,
            mcp_name: name,
            description,
            input_schema,
            mcp_client,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    async fn execute(&self, call: ToolCall) -> ToolResult {
        let arguments = serde_json::to_value(&call.arguments)
            .unwrap_or(JsonValue::Null);

        match self.mcp_client.call_tool(&self.mcp_name, arguments).await {
            Ok(content_blocks) => {
                let output = content_blocks
                    .iter()
                    .filter_map(|cb| cb.text.clone())
                    .collect::<Vec<_>>()
                    .join("\n");

                ToolResult {
                    success: true,
                    output: Some(output),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }
}
```

### 2.2 McpConnectionPool

管理多个 MCP 客户端连接：

```rust
// src/pool.rs

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use crate::client::MCPClient;

#[derive(Clone)]
pub struct McpConnectionPool {
    clients: Arc<RwLock<HashMap<String, Arc<MCPClient>>>>,
    default_server: Arc<RwLock<Option<String>>>,
}

impl McpConnectionPool {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            default_server: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn add_server(&self, name: &str, url: &str) -> Result<()> {
        let client = Arc::new(MCPClient::new(url.to_string()));
        self.clients.write().await.insert(name.to_string(), client);

        let mut default = self.default_server.write().await;
        if default.is_none() {
            *default = Some(name.to_string());
        }

        Ok(())
    }

    pub async fn connect(&self, name: &str) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        client.connect().await
    }

    pub async fn initialize(&self, name: &str, client_name: &str, version: &str) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        client.initialize(client_name, version).await
    }

    pub async fn client(&self, name: &str) -> Option<Arc<MCPClient>> {
        self.clients.read().await.get(name).cloned()
    }

    pub async fn default_client(&self) -> Option<Arc<MCPClient>> {
        let default = self.default_server.read().await;
        default.as_ref().and_then(|n| self.clients.read().await.get(n).cloned())
    }

    pub async fn list_servers(&self) -> Vec<String> {
        self.clients.read().await.keys().cloned().collect()
    }
}

impl Default for McpConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
```

### 2.3 McpToolLoader

从 MCP 服务器加载工具并注册到 ToolRegistry：

```rust
// src/loader.rs

use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::Mutex;
use swiftforge_types::ToolRegistry;
use crate::pool::McpConnectionPool;
use crate::adapter::McpToolAdapter;

pub struct McpToolLoader {
    pool: Arc<McpConnectionPool>,
    registry: Arc<Mutex<ToolRegistry>>,
}

impl McpToolLoader {
    pub fn new(pool: Arc<McpConnectionPool>, registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self { pool, registry }
    }

    pub async fn load_tools(&self, server_name: &str) -> Result<usize> {
        let client = self.pool.client(server_name).await
            .ok_or_else(|| anyhow!("Server '{}' not found", server_name))?;

        let tools = client.list_tools().await?;
        let count = tools.len();

        let mut registry = self.registry.lock().await;
        for tool_def in tools {
            let adapter = McpToolAdapter::new(
                server_name,
                client.clone(),
                tool_def.name,
                tool_def.description,
                tool_def.input_schema,
            );
            registry.register(adapter);
        }

        Ok(count)
    }

    pub async fn load_all(&self) -> Result<usize> {
        let mut total = 0;
        for server_name in self.pool.list_servers().await {
            match self.load_tools(&server_name).await {
                Ok(count) => total += count,
                Err(e) => {
                    tracing::warn!("[mcp] Failed to load tools from '{}': {}", server_name, e);
                }
            }
        }
        Ok(total)
    }
}
```

---

## 三、启动流程

### 3.1 异步初始化（推荐）

```
AppController::new()
├── 创建 ToolRegistry
│
├── 创建 McpConnectionPool
│   └── pool.add_server("mcp", "http://localhost:8080")
│
├── 创建 McpToolLoader
│
├── 启动后台连接任务（不等待）
│   ├── pool.connect("mcp")
│   ├── pool.initialize("mcp", "ragent", "0.1.0")
│   ├── loader.load_tools("mcp")
│   └── 记录日志
│
├── 立即返回，TUI 启动
└── 内置工具立即可用，MCP 工具连接后可用
```

### 3.2 错误处理

| 场景 | 处理 |
|------|------|
| MCP 服务器不可达 | 后台连接失败，仅记录日志，不阻塞启动 |
| 连接超时 | 同上 |
| 工具加载失败 | 记录警告，继续运行 |

---

## 四、工具调用流程

```
Agent::run_agent_loop()
    │
    ▼
ToolRegistry.execute(ToolCall { name: "mcp_read_file", arguments: {...} })
    │
    ├── 内置工具: 直接执行 → ToolResult
    │
    └── MCP工具:
            │
            ▼
        McpToolAdapter::execute()
            │
            ├── mcp_client.call_tool("read_file", arguments)
            │       │
            │       └── HTTP POST /tools/call
            │
            ├── 成功: ToolResult { success: true, output: ... }
            │
            └── 失败: ToolResult { success: false, error: ... }
```

---

## 五、配置文件格式

```toml
# swiftforge.toml
[mcp]
enabled = true
default_server = "mcp"

[[mcp.servers]]
name = "mcp"
url = "http://localhost:8080"
```

---

## 六、日志规范

| 事件 | 级别 | 格式 |
|------|------|------|
| MCP 后台连接启动 | INFO | `[mcp] Starting background connection to '{}'` |
| MCP 连接成功 | INFO | `[mcp] Connected to MCP server: {}` |
| MCP 初始化成功 | INFO | `[mcp] Initialized MCP server '{}' with {} tools` |
| MCP 连接失败 | WARN | `[mcp] Failed to connect to '{}': {}}` |
| 工具加载 | INFO | `[mcp] Loaded {} tools from '{}'` |
| 工具调用 | DEBUG | `[mcp] Calling tool '{}'` |
| 工具调用成功 | DEBUG | `[mcp] Tool '{}' returned: {}` |
| 工具调用失败 | WARN | `[mcp] Tool '{}' failed: {}` |

---

## 七、扩展性设计

### 7.1 多服务器支持

```toml
[[mcp.servers]]
name = "filesystem"
url = "http://localhost:8080"

[[mcp.servers]]
name = "git"
url = "http://localhost:8081"

[[mcp.servers]]
name = "search"
url = "https://api.search-mcp.com"
```

工具将注册为 `filesystem_read_file`、`git_commit`、`search_web` 等。

### 7.2 禁用前缀（单服务器场景）

如只需单服务器且不想用前缀，可在配置中设置 `prefix = false`：

```toml
[[mcp.servers]]
name = "mcp"
url = "http://localhost:8080"
prefix = false  # 工具名直接使用 "read_file" 而非 "mcp_read_file"
```

---

## 八、重构任务清单

| 任务 | 文件 | 优先级 |
|------|------|--------|
| 1. 创建 adapter.rs | `libs/swiftforge-mcp/src/adapter.rs` | 高 |
| 2. 创建 pool.rs | `libs/swiftforge-mcp/src/pool.rs` | 高 |
| 3. 创建 loader.rs | `libs/swiftforge-mcp/src/loader.rs` | 高 |
| 4. 更新 lib.rs | `libs/swiftforge-mcp/src/lib.rs` | 中 |
| 5. 更新 Cargo.toml | `libs/swiftforge-mcp/Cargo.toml` | 中 |
| 6. 创建 MCP 配置解析 | `swiftforge/src/config/` | 中 |
| 7. 集成到 AppController | `swiftforge/src/tui/app_controller.rs` | 高 |
| 8. 添加日志集成 | 全部 | 低 |

---

## 九、验收标准

- [ ] `cargo build -p swiftforge-mcp` 成功
- [ ] `cargo build --bin ragent` 成功
- [ ] MCP 连接异步进行，不阻塞 TUI 启动
- [ ] 内置工具在 MCP 连接前可用
- [ ] MCP 工具调用成功返回结果
- [ ] MCP 工具调用失败返回 `ToolResult { success: false }`
- [ ] 多服务器工具名正确添加前缀
- [ ] 日志正确记录 MCP 连接和调用

---

## 十、备选方案说明

### 为什么不用方案 1（同步连接）？

同步连接会阻塞 TUI 启动，影响用户体验。异步连接允许：
1. 用户立即开始使用内置工具
2. MCP 服务器可在后台慢慢连接
3. 即使 MCP 不可用，核心功能不受影响

### 为什么不用方案 3（按需连接）？

按需连接会导致首次 MCP 工具调用时等待连接 + 初始化，增加延迟。启动时异步连接：
1. 用户感知到连接进度（通过日志）
2. 连接成功后工具立即可用
3. 错误可以提前发现

---

*文档状态: 已批准*
