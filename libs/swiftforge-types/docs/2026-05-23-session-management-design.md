# Session 管理架构设计

> 文档版本: 1.0
> 生成日期: 2026-05-23
> 分支: feature/tui-refactor
> Worktree: `.worktrees/feat-tui-refactor/`
> 状态: **初稿 - 待审批**

---

## 概述

**所属架构**: [平台架构与接口规范](../architecture/2026-05-23-platform-architecture.md)

**功能系统**: Session Management (会话管理)

**设计目标**: 实现独立的 Session 管理服务，支持多 Session 切换、Context Window 摘要压缩、自动持久化。

---

## 一、设计背景

### 1.1 问题描述

当前 Session 系统存在以下问题：

| 问题 | 说明 | 严重度 |
|------|------|--------|
| Session 未使用 | Agent 直接持有 `Vec<Message>`，Session 定义但未使用 | 中 |
| 无 Context Window 管理 | 消息无限增长，没有截断/摘要机制 | 中 |
| 无 Session 持久化 | Session 不保存，重启后对话历史丢失 | 中 |
| 无多 Session 支持 | 只能单 Session，无法切换对话 | 低 |

### 1.2 设计决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| Session 存储 | SessionManager 独立服务 | Agent 外部管理，解耦 |
| 摘要触发 | 每次交互后检查 + 软性阈值 | 准确 + 平滑 |
| 摘要执行 | 使用当前 Agent 的 LLM | 简单，保持一致性 |
| 持久化格式 | SQLite 数据库 | 支持索引，查询方便 |
| 生命周期 | 混合模式 | 自动保存，显式删除 |
| 多Session切换 | SessionManager.switch_to() | API 清晰，解耦 |

---

## 二、架构设计

### 2.1 整体架构

参见 [平台架构与接口规范 - 第三章](../architecture/2026-05-23-platform-architecture.md#三核心类型定义)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           AppController                                       │
│  ┌────────────────────┐                                                      │
│  │  SessionManager    │ ← 独立服务，TUI和Agent都通过它操作Session          │
│  │  (Arc<SessionManager>)                                                     │
│  └────────────────────┘                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    ▼                 ▼                 ▼
            ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
            │   TUI        │  │   Agent       │  │   Storage    │
            │ (切换Session)│  │ (使用Session) │  │   (SQLite)   │
            └──────────────┘  └──────────────┘  └──────────────┘
                                      │
                                      ▼
                            ┌──────────────────┐
                            │   当前 Session    │
                            │   • messages     │
                            │   • context_window│
                            │   • token_count  │
                            └──────────────────┘
```

### 2.2 核心组件

#### SessionManager

```rust
// src/core/session_manager.rs

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,  // session_id → Session
    current_session_id: Arc<RwLock<Option<String>>>,
    db: SessionDatabase,  // SQLite
    config: SessionConfig,
}

pub struct SessionConfig {
    pub data_dir: PathBuf,
    pub compact_threshold: f32,  // 触发摘要的阈值 (默认 0.8 = 80%)
    pub default_context_window: usize,
}

impl SessionManager {
    /// 创建新的 SessionManager
    pub fn new(data_dir: PathBuf) -> Result<Self>;

    // Session CRUD
    pub fn create_session(&self, name: &str) -> Result<Session>;
    pub fn get_session(&self, session_id: &str) -> Option<Session>;
    pub fn delete_session(&self, session_id: &str) -> Result<()>;
    pub fn list_sessions(&self) -> Vec<SessionMeta>;

    // 切换
    pub fn switch_to(&self, session_id: &str) -> Result<()>;
    pub fn current_session(&self) -> Option<Session>;
    pub fn current_session_mut(&self) -> Option<SessionGuard>;

    // 保存
    pub fn save_current(&self) -> Result<()>;
    pub async fn auto_save(&self) -> Result<()>;
}
```

#### Session

```rust
// src/core/session.rs

pub struct Session {
    pub id: String,
    pub name: String,
    pub messages: VecDeque<Message>,
    pub context_window: usize,        // 模型限制
    pub max_tokens: Option<usize>,    // 最大 token 数
    pub token_count: usize,           // 当前累计 token
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl Session {
    // 消息管理
    pub fn add_message(&mut self, role: &str, content: &str);
    pub fn add_message_with_tokens(&mut self, role: &str, content: &str, token_count: usize);
    pub fn messages(&self) -> Vec<Message>;

    // 上下文窗口管理
    pub fn check_and_compact(&mut self, agent: &Agent) -> Result<()>;
    pub fn needs_compaction(&self) -> bool;
    pub fn estimate_token_count(&self) -> usize;

    // 摘要
    pub async fn compact(&mut self, agent: &Agent) -> Result<()>;
}
```

#### SessionGuard

```rust
// 提供可变访问当前 Session 的 RAII _guard
pub struct SessionGuard<'a> {
    manager: &'a SessionManager,
    session: Option<Session>,
    modified: bool,
}

impl<'a> SessionGuard<'a> {
    pub fn session(&self) -> &Session;
    pub fn session_mut(&mut self) -> &mut Session;

    // Drop 时自动保存
    fn mark_modified(&mut self);
}

impl Drop for SessionGuard<'_> {
    fn drop(&mut self) {
        if self.modified {
            // 自动保存
        }
    }
}
```

#### SessionDatabase (SQLite)

```rust
// src/core/session_db.rs

pub struct SessionDatabase {
    conn: Connection,
}

// 表结构:
// sessions (
//   id TEXT PRIMARY KEY,
//   name TEXT NOT NULL,
//   messages_json TEXT NOT NULL,
//   context_window INTEGER NOT NULL,
//   max_tokens INTEGER,
//   token_count INTEGER NOT NULL,
//   created_at TEXT NOT NULL,
//   updated_at TEXT NOT NULL
// )

impl SessionDatabase {
    pub fn new(path: PathBuf) -> Result<Self>;

    pub fn save(&self, session: &Session) -> Result<()>;
    pub fn load(&self, session_id: &str) -> Result<Option<Session>>;
    pub fn load_all(&self) -> Result<Vec<Session>>;
    pub fn delete(&self, session_id: &str) -> Result<()>;
}
```

---

## 三、执行流程

### 3.1 Agent 使用 Session

```rust
impl Agent {
    pub async fn run_agent_loop(&self, session_manager: Arc<SessionManager>, ...) {
        let mut session_guard = session_manager.current_session_mut()
            .ok_or_else(|| anyhow::anyhow!("No active session"))?;

        let session = session_guard.session();

        // 1. 添加用户消息
        session_guard.session_mut().add_message("user", initial_message);

        // 2. 获取消息并调用 provider
        let messages = session.messages();
        let response = self.chat_with_tools_streaming(messages.clone(), ...).await?;

        // 3. 添加 assistant 消息
        session_guard.session_mut().add_message("assistant", response.content);

        // 4. 更新 token 计数
        session_guard.session_mut().add_token_count(response.usage.total_tokens);

        // 5. 检查是否需要摘要压缩
        if session_guard.session_mut().needs_compaction() {
            session_guard.session_mut().compact(self).await?;
        }

        // 6. Drop SessionGuard 时自动保存
    }
}
```

### 3.2 TUI 切换 Session

```rust
// TUI 中的 Session 切换
async fn switch_session(session_manager: Arc<SessionManager>, session_id: String) -> Result<()> {
    // 1. 列出所有 Session
    let sessions = session_manager.list_sessions();

    // 2. 显示选择列表 (UI)

    // 3. 用户选择后切换
    session_manager.switch_to(&session_id)?;

    Ok(())
}
```

### 3.3 摘要压缩流程

```rust
impl Session {
    pub async fn compact(&mut self, agent: &Agent) -> Result<()> {
        // 1. 如果消息太少，不需要压缩
        if self.messages.len() < 10 {
            return Ok(());
        }

        // 2. 构建摘要 prompt
        let history = self.messages.iter()
            .rev()  // 从最近的消息开始
            .take(50)  // 保留最近 50 条
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .join("\n");

        let prompt = format!(
            "Summarize this conversation concisely, preserving key information, decisions, and context.\n\
            Keep the summary under 500 tokens.\n\n\
            Conversation:\n{}",
            history
        );

        // 3. 调用 LLM 摘要
        let summary_response = agent.chat(vec![
            Message { role: "user", content: prompt }
        ]).await?;

        // 4. 替换历史消息为摘要
        let summary_text = summary_response.content.trim();

        // 保留最后 5 条消息（最近的上下文）
        let recent_messages: Vec<_> = self.messages.iter().rev().take(5).cloned().collect();

        self.messages.clear();
        self.messages.push_back(Message {
            role: "system".to_string(),
            content: format!(
                "[Previous conversation summarized: {}]\n\n[Last {} messages preserved]",
                summary_text,
                recent_messages.len()
            ),
        });

        // 5. 添加最近的消息
        for msg in recent_messages.into_iter().rev() {
            self.messages.push_back(msg);
        }

        // 6. 重置 token 计数
        self.token_count = self.estimate_token_count();

        info!("[session]", "Session {} compacted, {} messages -> 1 summary + {} recent",
              self.id, self.messages.len() - recent_messages.len() - 1, recent_messages.len());

        Ok(())
    }
}
```

---

## 四、错误处理

| 场景 | 处理方式 |
|------|---------|
| Session 不存在 | 返回 `SessionError::NotFound` |
| 数据库写入失败 | 回退到内存，重试保存 |
| 摘要失败 | 记录日志，保留原消息，不阻塞主流程 |
| 无 Provider | 跳过摘要（无法调用 LLM） |

---

## 五、日志集成

所有 Session 操作通过 `src/log/` 模块记录：

```rust
// Session 创建
info!("[session]", "Session created: {} '{}'", session.id, session.name);

// Session 切换
info!("[session]", "Switched to session: {} '{}'", session.id, session.name);

// Session 保存
debug!("[session]", "Session saved: {} ({} messages)", session.id, session.messages.len());

// 摘要压缩
info!("[session]", "Session {} compacted from {} messages", session.id, before_len);

// 错误
error!("[session]", "Session save failed: {}", e);
```

---

## 六、文件路径

| 文件 | 说明 |
|------|------|
| `src/core/session_manager.rs` | SessionManager 实现 |
| `src/core/session.rs` | Session 定义和实现 |
| `src/core/session_db.rs` | SQLite 持久化 |
| `src/core/session_error.rs` | 错误类型定义 |

---

## 七、重构任务清单

| 任务 | 描述 | 优先级 |
|------|------|--------|
| 1 | 创建 `SessionError` 类型 | 中 |
| 2 | 扩展 `Session` 结构体 | 高 |
| 3 | 创建 `SessionDatabase` | 高 |
| 4 | 创建 `SessionManager` | 高 |
| 5 | 实现 `SessionManager.switch_to()` | 高 |
| 6 | 实现 `Session.compact()` | 高 |
| 7 | 修改 `Agent.run_agent_loop()` 使用 SessionManager | 高 |
| 8 | TUI Session 切换 UI | 中 |

---

## 八、验证清单

- [ ] `cargo build` 编译通过
- [ ] Session 创建/保存/加载正常
- [ ] 多 Session 切换正常
- [ ] 摘要压缩正常触发
- [ ] 自动保存正常
- [ ] 日志正确记录

---

## 九、备选方案

### 方案 B: 分层存储

将热数据放内存，冷数据放 SQLite：

```
SessionManager
    ├── hot_sessions: Arc<RwLock<HashMap<String, Session>>>  // 内存
    └── cold_storage: SessionDatabase  // SQLite
```

**优点**: 切换快
**缺点**: 复杂

### 方案 C: 无摘要压缩

仅使用滑动窗口，不调用 LLM 摘要：

```
if messages.len() > 100 {
    messages.truncate(50);  // 简单截断
}
```

**优点**: 简单
**缺点**: 丢失上下文

---

*文档状态: 初稿 - 待审批*