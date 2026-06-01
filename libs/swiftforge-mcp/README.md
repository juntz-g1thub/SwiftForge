# swiftforge-mcp

MCP (Model Context Protocol) 客户端库，通过适配层统一接入 ToolRegistry。

## 文档

| 文档 | 说明 |
|------|------|
| `docs/2026-05-23-mcp-tool-unified-design.md` | MCP 工具统一架构设计（旧版） |
| `docs/2026-06-01-mcp-tool-unified-architecture-design.md` | MCP 工具统一架构设计（已批准） |
| `docs/specs/2026-06-01-mcp-tool-unified-implementation-plan.md` | 详细实现计划 |

## 概述

- **适配层模式**：将 MCP 服务器上的工具适配为本地 `Tool` trait
- **连接池架构**：为多服务器管理预留
- **启动时连接**：简单，日志状态可追踪

## 核心组件

- `McpToolAdapter`：将 MCP 工具适配为本地 Tool
- `McpConnectionPool`：管理多个 MCP 客户端连接
- `McpToolLoader`：从 MCP 服务器加载工具并注册到 ToolRegistry
