# MCP Tool Unified Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 MCP 工具统一架构，将 MCP 服务器上的工具通过适配层接入 ToolRegistry，与内置工具共存。

**Architecture:** 扩展 `swiftforge-mcp` 库，添加 adapter/pool/loader 三个核心组件。工具命名使用 `{server_name}_{tool_name}` 前缀格式。初始化采用异步连接，不阻塞 TUI 启动。

**Tech Stack:** Rust, async-trait, tokio, reqwest, tracing

---

## 文件结构

```
libs/swiftforge-mcp/
├── Cargo.toml         # 更新：添加 swiftforge-types 依赖
├── src/
│   ├── lib.rs         # 修改：导出新模块
│   ├── client.rs      # 已存在
│   ├── protocol.rs    # 已存在
│   ├── adapter.rs     # 新增：McpToolAdapter
│   ├── pool.rs        # 新增：McpConnectionPool
│   └── loader.rs      # 新增：McpToolLoader

swiftforge/src/
├── lib.rs             # 修改：导出 MCP 相关类型
└── tui/
    └── app_controller.rs  # 修改：集成 MCP 异步初始化
```

---

## Task 1: 创建 adapter.rs

**Files:**
- Create: `libs/swiftforge-mcp/src/adapter.rs`

- [ ] **Step 1: 创建 adapter.rs 文件**

```rust
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

- [ ] **Step 2: 验证编译**

Run: `cargo build -p swiftforge-mcp`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add libs/swiftforge-mcp/src/adapter.rs
git commit -m "feat(mcp): add McpToolAdapter for Tool trait implementation"
```

---

## Task 2: 创建 pool.rs

**Files:**
- Create: `libs/swiftforge-mcp/src/pool.rs`

- [ ] **Step 1: 创建 pool.rs 文件**

```rust
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

- [ ] **Step 2: 验证编译**

Run: `cargo build -p swiftforge-mcp`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add libs/swiftforge-mcp/src/pool.rs
git commit -m "feat(mcp): add McpConnectionPool for multi-server management"
```

---

## Task 3: 创建 loader.rs

**Files:**
- Create: `libs/swiftforge-mcp/src/loader.rs`

- [ ] **Step 1: 创建 loader.rs 文件**

```rust
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

- [ ] **Step 2: 验证编译**

Run: `cargo build -p swiftforge-mcp`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add libs/swiftforge-mcp/src/loader.rs
git commit -m "feat(mcp): add McpToolLoader for tool registration"
```

---

## Task 4: 更新 lib.rs

**Files:**
- Modify: `libs/swiftforge-mcp/src/lib.rs`

- [ ] **Step 1: 更新 lib.rs 导出新模块**

```rust
pub mod client;
pub mod protocol;
pub mod adapter;
pub mod pool;
pub mod loader;

pub use client::MCPClient;
pub use pool::McpConnectionPool;
pub use loader::McpToolLoader;
pub use adapter::McpToolAdapter;
```

- [ ] **Step 2: 更新 Cargo.toml 添加依赖**

```toml
[package]
name = "swiftforge-mcp"
version.workspace = true
edition.workspace = true

[dependencies]
swiftforge-types = { path = "../swiftforge-types" }

async-trait = "0.1"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 3: 验证编译**

Run: `cargo build -p swiftforge-mcp`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add libs/swiftforge-mcp/src/lib.rs libs/swiftforge-mcp/Cargo.toml
git commit -m "feat(mcp): export new modules and add dependencies"
```

---

## Task 5: 集成到 AppController

**Files:**
- Modify: `swiftforge/src/tui/app_controller.rs`

- [ ] **Step 1: 添加导入**

```rust
use swiftforge_mcp::{McpConnectionPool, McpToolLoader};
```

- [ ] **Step 2: 在 AppController 结构体中添加字段**

```rust
// 在 AppController 结构体中添加
mcp_pool: Option<Arc<McpConnectionPool>>,
mcp_loader: Option<Arc<McpToolLoader>>,
```

- [ ] **Step 3: 在 new() 中添加异步初始化逻辑**

在创建 Agent 后，添加后台 MCP 连接任务：

```rust
// 在 new() 方法中，Agent 创建之后添加
let mcp_pool = Arc::new(McpConnectionPool::new());
let mcp_loader = Arc::new(McpToolLoader::new(
    Arc::clone(&mcp_pool),
    Arc::clone(&tool_registry),
));

// 添加默认 MCP 服务器（从配置读取 URL）
if let Some(mcp_url) = config.get_mcp_url() {
    if let Err(e) = mcp_pool.add_server("mcp", &mcp_url).await {
        tracing::warn!("[mcp] Failed to add server: {}", e);
    } else {
        // 启动后台连接任务
        let pool = Arc::clone(&mcp_pool);
        let loader = Arc::clone(&mcp_loader);
        let runtime = self.runtime.handle().clone();

        runtime.spawn(async move {
            tracing::info!("[mcp] Starting background connection to 'mcp'");

            if let Err(e) = pool.connect("mcp").await {
                tracing::warn!("[mcp] Failed to connect to '{}': {}", "mcp", e);
                return;
            }
            tracing::info!("[mcp] Connected to MCP server: {}", "mcp");

            if let Err(e) = pool.initialize("mcp", "ragent", env!("CARGO_PKG_VERSION")).await {
                tracing::warn!("[mcp] Failed to initialize '{}': {}", "mcp", e);
                return;
            }

            match loader.load_tools("mcp").await {
                Ok(count) => {
                    tracing::info!("[mcp] Loaded {} tools from '{}'", count, "mcp");
                }
                Err(e) => {
                    tracing::warn!("[mcp] Failed to load tools from '{}': {}", "mcp", e);
                }
            }
        });
    }
}

self.mcp_pool = Some(mcp_pool);
self.mcp_loader = Some(mcp_loader);
```

- [ ] **Step 4: 验证编译**

Run: `cargo build --bin ragent 2>&1 | tail -30`
Expected: 编译成功，无新错误

- [ ] **Step 5: 提交**

```bash
git add swiftforge/src/tui/app_controller.rs
git commit -m "feat(tui): integrate MCP async initialization"
```

---

## Task 6: 添加 MCP 配置读取（可选）

**Files:**
- Create: `swiftforge/src/config/mcp_config.rs` (如果配置模块不存在)
- Modify: `swiftforge/src/config/mod.rs`

- [ ] **Step 1: 创建 MCP 配置结构**

```rust
#[derive(Debug, Clone)]
pub struct McpConfig {
    pub enabled: bool,
    pub default_server: Option<String>,
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub url: String,
}

impl McpConfig {
    pub fn from_toml(table: &toml::Table) -> Option<Self> {
        let mcp_table = table.get("mcp")?.as_table()?;

        let enabled = mcp_table.get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let default_server = mcp_table.get("default_server")
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut servers = Vec::new();
        if let Some(server_list) = mcp_table.get("servers").and_then(|v| v.as_array()) {
            for server in server_list {
                let table = server.as_table()?;
                let name = table.get("name")?.as_str()?.to_string();
                let url = table.get("url")?.as_str()?.to_string();
                servers.push(McpServerConfig { name, url });
            }
        }

        Some(McpConfig { enabled, default_server, servers })
    }

    pub fn get_mcp_url(&self) -> Option<String> {
        self.servers.first().map(|s| s.url.clone())
    }
}
```

- [ ] **Step 2: 更新 ConfigManager 添加 get_mcp_url()**

在 `ConfigManager` 结构体中添加：

```rust
pub fn get_mcp_url(&self) -> Option<String> {
    self.mcp_config.as_ref()?.get_mcp_url()
}
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --bin ragent 2>&1 | tail -20`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add swiftforge/src/config/mcp_config.rs swiftforge/src/config/mod.rs
git commit -m "feat(config): add MCP configuration support"
```

---

## Task 7: 完整验证

- [ ] **Step 1: Workspace 编译**

Run: `cargo build --workspace 2>&1 | tail -30`
Expected: 编译成功

- [ ] **Step 2: Clippy 检查**

Run: `cargo clippy -p swiftforge-mcp 2>&1 | tail -20`
Expected: 无警告

- [ ] **Step 3: 检查日志集成**

确认日志格式正确：
- `[mcp] Starting background connection to 'mcp'`
- `[mcp] Connected to MCP server: mcp`
- `[mcp] Loaded {} tools from 'mcp'`

---

## 验收清单

- [ ] `cargo build -p swiftforge-mcp` 成功
- [ ] `cargo build --bin ragent` 成功
- [ ] MCP 连接异步进行，不阻塞 TUI 启动
- [ ] 内置工具在 MCP 连接前可用
- [ ] MCP 工具调用成功返回结果
- [ ] MCP 工具调用失败返回 `ToolResult { success: false }`
- [ ] 多服务器工具名正确添加前缀 `{server_name}_{tool_name}`
- [ ] 日志正确记录 MCP 连接和调用

---

**Plan complete and saved to `docs/specs/2026-06-01-mcp-tool-unified-implementation-plan.md`**

两个执行选项：

**1. Subagent-Driven (recommended)** - 我 dispatch 一个 subagent per task，task 间 review，快速迭代

**2. Inline Execution** - 在本 session 中使用 executing-plans 执行，带检查点

你想用哪个方式？
