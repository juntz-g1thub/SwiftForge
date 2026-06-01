# MCP 工具统一架构设计

> 文档版本: 0.9
> 生成日期: 2026-05-23
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **已废弃 - 被 2026-06-01-mcp-tool-unified-architecture-design.md 替代**

---

## 概述

**所属架构**: [平台架构与接口规范](../architecture/2026-05-23-platform-architecture.md)

**功能系统**: Tool System (工具系统) — MCP 工具统一架构

**设计目标**: 将 MCP 工具通过适配层统一接入 ToolRegistry，与内置工具共存，形成统一的工具调用入口。

---

## 一、设计背景

### 1.1 问题描述

当前工具系统存在两套并行实现：

| 工具类型 | 来源 | 调用方式 | 问题 |
|---------|------|---------|------|
| 内置工具 (5个) | 编译进二进制 | 直接Rust函数调用 | 维护成本高，扩展性差 |
| MCP工具 | MCP服务器 | HTTP JSON-RPC | 未集成，无法使用 |

**当前架构**:
```
内置工具 (5个) ─────────────────────┐
                                   │
MCP Client ─────────────────────────┼──▶ ToolRegistry
(存在但未集成)                      │
                                   │
                                   ▼
                            Agent.run_agent_loop()
```

### 1.2 设计决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 架构 | 适配层模式 | 最小实现，专注适配而非自建MCP服务器 |
| 服务器数量 | 单server起步 | 保留多服务器扩展性 |
| 连接管理 | 连接池架构 | 为多server管理预留 |
| 连接时机 | 启动时连接 | 简单，日志状态可追踪 |
| 观测方式 | Log模块统一记录 | 配合Debug Panel移除 |

---

## 二、架构设计

### 2.1 整体架构

参见 [平台架构与接口规范 - 第四章](../architecture/2026-05-23-platform-architecture.md#四mcp-工具统一架构)

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

### 2.2 核心组件

#### McpToolAdapter

将 MCP 服务器上的工具适配为本地 `Tool` trait：

```rust
// src/integration/mcp/adapter.rs

pub struct McpToolAdapter {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    mcp_client: Arc<MCPClient>,
}

impl McpToolAdapter {
    pub fn new(
        mcp_client: Arc<MCPClient>,
        tool_def: protocol::Tool,
    ) -> Self {
        Self {
            name: tool_def.name,
            description: tool_def.description,
            input_schema: tool_def.input_schema,
            mcp_client,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }
    fn input_schema(&self) -> serde_json::Value { self.input_schema.clone() }

    async fn execute(&self, call: ToolCall) -> ToolResult {
        let arguments = serde_json::to_value(&call.arguments)
            .unwrap_or(serde_json::Value::Null);

        match self.mcp_client.call_tool(&self.name, arguments).await {
            Ok(content_blocks) => {
                // Vec<ContentBlock> → ToolResult
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

#### McpConnectionPool

管理多个 MCP 客户端连接（为多服务器扩展预留）：

```rust
// src/integration/mcp/pool.rs

pub struct McpConnectionPool {
    clients: HashMap<String, Arc<MCPClient>>,
    default_server: Option<String>,
}

impl McpConnectionPool {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            default_server: None,
        }
    }

    pub fn add_server(&mut self, name: &str, url: &str) -> Result<()> {
        let client = Arc::new(MCPClient::new(url.to_string()));
        self.clients.insert(name.to_string(), client);

        if self.default_server.is_none() {
            self.default_server = Some(name.to_string());
        }
        Ok(())
    }

    pub async fn connect(&self, name: &str) -> Result<()> {
        let client = self.clients.get(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        client.connect().await
    }

    pub async fn initialize(&self, name: &str, client_name: &str, version: &str) -> Result<()> {
        let client = self.clients.get(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        client.initialize(client_name, version).await
    }

    pub fn client(&self, name: &str) -> Option<&Arc<MCPClient>> {
        self.clients.get(name)
    }

    pub fn default_client(&self) -> Option<&Arc<MCPClient>> {
        self.default_server
            .as_ref()
            .and_then(|n| self.clients.get(n))
    }
}
```

#### McpToolLoader

从 MCP 服务器加载工具并注册到 ToolRegistry：

```rust
// src/integration/mcp/loader.rs

pub struct McpToolLoader {
    pool: Arc<Mutex<McpConnectionPool>>,
    registry: Arc<ToolRegistry>,
}

impl McpToolLoader {
    pub fn new(pool: Arc<Mutex<McpConnectionPool>>, registry: Arc<ToolRegistry>) -> Self {
        Self { pool, registry }
    }

    pub async fn load_tools(&self, server_name: &str) -> Result<usize> {
        let client = {
            let pool = self.pool.lock().unwrap();
            pool.client(server_name).cloned()
                .ok_or_else(|| anyhow!("Server '{}' not found", server_name))?
        };

        let tools = client.list_tools().await?;
        let count = tools.len();

        for tool_def in tools {
            let adapter = McpToolAdapter::new(client.clone(), tool_def);
            self.registry.register(adapter);
        }

        Ok(count)
    }
}
```

---

## 三、初始化流程

### 3.1 启动时连接

```
AppController::new()
    │
    ├── 创建 McpConnectionPool
    │
    ├── pool.add_server("default", "http://localhost:8080")
    │
    ├── pool.connect("default")
    │
    ├── pool.initialize("default", "ragent", "0.1.0")
    │
    ├── McpToolLoader::new(pool, registry)
    │
    ├── loader.load_tools("default") → Vec<McpToolAdapter>
    │
    └── registry.get_definitions() → 传给 Agent
```

### 3.2 工具调用流程

```
Agent::run_agent_loop()
    │
    ▼
ToolRegistry.execute(ToolCall { name: "read_file", arguments: {...} })
    │
    ├── 内置工具: 直接执行 → ToolResult
    │
    └── MCP工具:
            │
            ▼
        McpToolAdapter::execute()
            │
            ├── mcp_client.call_tool(name, arguments)
            │       │
            │       └── HTTP POST /tools/call
            │           { name: "read_file", arguments: {...} }
            │
            ├── Vec<ContentBlock> ← HTTP响应
            │
            └── convert → ToolResult { success, output, error }
```

---

## 四、错误处理

| 场景 | 处理方式 |
|------|---------|
| MCP 服务器不可达 | 返回 `ToolResult { success: false, error: Some("连接失败") }` |
| 工具不存在 | `ToolRegistry::execute` 返回错误 |
| 调用超时 | MCPClient 内部 30s 超时，返回错误 |
| 结果格式错误 | 适配层捕获解析错误，返回 `ToolResult { success: false }` |
| 服务器返回错误 | 转换 JSON-RPC error 为 ToolResult |

---

## 五、日志集成

所有 MCP 操作通过 `src/log/` 模块记录（配合 debug panel 移除）：

```rust
// MCP 连接日志
info!("[mcp]", "Connected to MCP server: {}", server_url);
info!("[mcp]", "Loaded {} tools from '{}'", count, server_name);

// MCP 调用日志
debug!("[mcp]", "Calling tool '{}' with args: {:?}", tool_name, args);
debug!("[mcp]", "Tool '{}' returned: {}", tool_name, result.output);

// 错误日志
error!("[mcp]", "MCP call failed: {}", e);
warn!("[mcp]", "Connection lost, reconnecting...");
```

---

## 六、扩展性设计

### 6.1 多服务器支持

```rust
// 未来扩展: 多服务器
pool.add_server("filesystem", "http://localhost:8080");
pool.add_server("git", "http://localhost:8081");
pool.add_server("search", "https://api.search-mcp.com");

// 工具名前缀区分来源
"fs_read_file"      // filesystem 服务器
"git_commit"        // git 服务器
"web_search"        // search 服务器
```

### 6.2 工具前缀机制

```rust
// McpToolLoader 加载时添加前缀
let tool_name = format!("{}_{}", server_name, tool_def.name);
let adapter = McpToolAdapter::new(client.clone(), tool_def, tool_name);
registry.register(adapter);
```

---

## 七、重构任务清单

| 任务 | 描述 | 优先级 |
|------|------|--------|
| 1. 创建 `src/integration/mcp/adapter.rs` | McpToolAdapter 实现 | 高 |
| 2. 创建 `src/integration/mcp/pool.rs` | McpConnectionPool 实现 | 高 |
| 3. 创建 `src/integration/mcp/loader.rs` | McpToolLoader 实现 | 高 |
| 4. 创建 `src/integration/mcp/mod.rs` | 模块导出 | 中 |
| 5. 修改 `AppController::new()` | MCP 初始化逻辑 | 中 |
| 6. 集成 Log 模块 | MCP 日志集成 | 低 |
| 7. 移除内置工具 (可选) | 全部替换为 MCP | 低 |

---

## 八、验证清单

- [ ] `cargo build` 编译通过
- [ ] MCP 客户端成功连接服务器
- [ ] `tools/list` 返回正确工具列表
- [ ] `tools/call` 成功调用并返回结果
- [ ] ToolRegistry 能同时管理内置工具和 MCP 工具
- [ ] 日志正确记录 MCP 连接和调用
- [ ] 错误处理正确返回 ToolResult

---

## 九、备选方案

### 方案B: 纯 MCP（无内置工具）

完全移除内置工具，全部通过 MCP 服务器暴露：

```
优点:
- 架构更统一
- 减少维护负担
- 工具定义集中管理

缺点:
- 内置工具（如 bash）需要通过 MCP 服务器暴露
- 需要额外的基础设施来运行 MCP 服务器
- 离线场景无法使用
```

### 方案C: 分层设计

内置工具为基础，MCP 作为高级扩展：

```
Level 1: 内置工具 (离线可用)
Level 2: MCP 工具 (需要 MCP 服务器)
```

用户可选择启用 MCP 或仅使用内置工具。

---

*文档状态: 待审批*