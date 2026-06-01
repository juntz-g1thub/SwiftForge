# swiftforge-types

核心类型库，所有其他库的基础。

## 文档

| 文档 | 说明 |
|------|------|
| `docs/2026-05-23-session-management-design.md` | Session 管理架构设计 |

## 概述

包含以下核心类型：

- **Message**: 消息结构
- **Session**: 会话管理
- **Tool, ToolCall, ToolResult, ToolDefinition**: 工具系统
- **Provider**: LLM Provider 接口和配置

## 模块

- `message`: 消息类型定义
- `tool`: 工具类型和 ToolRegistry
- `provider`: Provider 接口
- `session`: 会话管理
