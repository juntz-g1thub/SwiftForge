# Session Management Implementation Plan (T9)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate Session management into Agent, replacing `Vec<Message>` with `SessionManager`, implementing SQLite persistence and LLM-based summarization (compact).

**Architecture:** SessionManager holds multiple Sessions (HashMap), each Session has messages VecDeque. Agent uses SessionManager to get/update messages. SQLite-backed persistence. Compact triggered after each interaction when threshold exceeded.

**Tech Stack:** Rust, SQLite (rusqlite), existing Agent/Provider infrastructure

---

## File Structure

```
libs/swiftforge-types/src/
├── session.rs              # Extend Session struct (add id, name, token_count, etc.)
└── session_error.rs        # Create SessionError type

libs/swiftforge-task/src/
└── session_db.rs          # Create SessionDatabase (SQLite)

swiftforge/src/core/
├── session_manager.rs      # Create SessionManager
└── session_manager_tests.rs # Tests

swiftforge/src/core/agent.rs  # Modify to use SessionManager
```

---

## Task 1: Extend Session struct

**Files:**
- Modify: `libs/swiftforge-types/src/session.rs`

- [ ] **Step 1: Read current Session struct**

Run: `cat libs/swiftforge-types/src/session.rs`
Verify: Current fields are `messages: VecDeque<Message>`, `context_window: usize`

- [ ] **Step 2: Add new fields to Session**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub messages: VecDeque<Message>,
    pub context_window: usize,
    pub max_tokens: Option<usize>,
    pub token_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl Session {
    pub fn new(id: String, name: String, context_window: usize) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            messages: VecDeque::new(),
            context_window,
            max_tokens: None,
            token_count: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push_back(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn messages(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }

    pub fn needs_compaction(&self) -> bool {
        let threshold = (self.context_window as f32 * 0.8) as usize;
        self.token_count > threshold
    }

    pub fn estimate_token_count(&self) -> usize {
        self.token_count
    }
}
```

- [ ] **Step 3: Update lib.rs exports**

```rust
pub use session::{Message, Session, SessionConfig, SessionError};
```

- [ ] **Step 4: Build to verify**

Run: `cargo build -p swiftforge-types 2>&1 | tail -10`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add libs/swiftforge-types/src/session.rs libs/swiftforge-types/src/lib.rs
git commit -m "feat(types): extend Session struct with id, name, token_count, timestamps"
```

---

## Task 2: Create SessionError type

**Files:**
- Create: `libs/swiftforge-types/src/session_error.rs`

- [ ] **Step 1: Create SessionError**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session save failed: {0}")]
    SaveFailed(String),

    #[error("Session load failed: {0}")]
    LoadFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("No active session")]
    NoActiveSession,

    #[error("Compact failed: {0}")]
    CompactFailed(String),
}

impl From<rusqlite::Error> for SessionError {
    fn from(err: rusqlite::Error) -> Self {
        SessionError::DatabaseError(err.to_string())
    }
}
```

- [ ] **Step 2: Update session.rs to re-export**

Add to `session.rs`:
```rust
pub use session_error::SessionError;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p swiftforge-types 2>&1 | tail -10`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-types/src/session_error.rs libs/swiftforge-types/src/session.rs libs/swiftforge-types/src/lib.rs
git commit -m "feat(types): add SessionError type"
```

---

## Task 3: Create SessionDatabase (SQLite)

**Files:**
- Create: `libs/swiftforge-task/src/session_db.rs`
- Create: `libs/swiftforge-task/src/lib.rs` (if needed)

- [ ] **Step 1: Create SessionDatabase**

```rust
use rusqlite::{Connection, params};
use std::path::PathBuf;
use swiftforge_types::{Session, SessionConfig, SessionError};

pub struct SessionDatabase {
    conn: Connection,
}

impl SessionDatabase {
    pub fn new(path: &PathBuf) -> Result<Self, SessionError> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                messages_json TEXT NOT NULL,
                context_window INTEGER NOT NULL,
                max_tokens INTEGER,
                token_count INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn save(&self, session: &Session) -> Result<(), SessionError> {
        let messages_json = serde_json::to_string(&session.messages)
            .map_err(|e| SessionError::SaveFailed(e.to_string()))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO sessions
             (id, name, messages_json, context_window, max_tokens, token_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.id,
                session.name,
                messages_json,
                session.context_window,
                session.max_tokens,
                session.token_count,
                session.created_at,
                session.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn load(&self, session_id: &str) -> Result<Option<Session>, SessionError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, messages_json, context_window, max_tokens, token_count, created_at, updated_at
             FROM sessions WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![session_id])?;

        if let Some(row) = rows.next()? {
            let messages_json: String = row.get(2)?;
            let messages: VecDeque<swiftforge_types::Message> =
                serde_json::from_str(&messages_json)
                    .map_err(|e| SessionError::LoadFailed(e.to_string()))?;

            Ok(Some(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                messages,
                context_window: row.get(3)?,
                max_tokens: row.get(4)?,
                token_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn load_all(&self) -> Result<Vec<Session>, SessionError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, messages_json, context_window, max_tokens, token_count, created_at, updated_at
             FROM sessions ORDER BY updated_at DESC"
        )?;

        let mut sessions = Vec::new();
        let mut rows = stmt.raw_iter();

        while let Some(row) = rows.next()? {
            let messages_json: String = row.get(2)?;
            let messages: VecDeque<swiftforge_types::Message> =
                serde_json::from_str(&messages_json)
                    .map_err(|e| SessionError::LoadFailed(e.to_string()))?;

            sessions.push(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                messages,
                context_window: row.get(3)?,
                max_tokens: row.get(4)?,
                token_count: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            });
        }

        Ok(sessions)
    }

    pub fn delete(&self, session_id: &str) -> Result<(), SessionError> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }
}
```

- [ ] **Step 2: Add rusqlite dependency**

Modify: `libs/swiftforge-task/Cargo.toml`
```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p swiftforge-task 2>&1 | tail -15`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-task/src/session_db.rs libs/swiftforge-task/Cargo.toml
git commit -m "feat(task): add SessionDatabase with SQLite persistence"
```

---

## Task 4: Create SessionManager

**Files:**
- Create: `swiftforge/src/core/session_manager.rs`

- [ ] **Step 1: Create SessionManager**

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use swiftforge_types::{Session, SessionError};
use crate::session_db::SessionDatabase;

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    current_session_id: Arc<RwLock<Option<String>>>,
    db: Arc<SessionDatabase>,
    data_dir: PathBuf,
}

impl SessionManager {
    pub fn new(data_dir: PathBuf) -> Result<Self, SessionError> {
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("sessions.db");
        let db = SessionDatabase::new(&db_path)?;

        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            current_session_id: Arc::new(RwLock::new(None)),
            db: Arc::new(db),
            data_dir,
        })
    }

    pub async fn create_session(&self, name: &str, context_window: usize) -> Result<Session, SessionError> {
        let id = Uuid::new_v4().to_string();
        let session = Session::new(id.clone(), name.to_string(), context_window);

        self.db.save(&session)?;
        self.sessions.write().await.insert(id.clone(), session.clone());
        *self.current_session_id.write().await = Some(id);

        Ok(session)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        if let Some(session) = self.sessions.read().await.get(session_id) {
            return Some(session.clone());
        }
        self.db.load(session_id).ok().flatten()
    }

    pub async fn get_current_session(&self) -> Option<Session> {
        let session_id = self.current_session_id.read().await.clone();
        if let Some(id) = session_id {
            self.get_session(&id).await
        } else {
            None
        }
    }

    pub async fn switch_to(&self, session_id: &str) -> Result<(), SessionError> {
        if self.get_session(session_id).await.is_none() {
            return Err(SessionError::NotFound(session_id.to_string()));
        }
        *self.current_session_id.write().await = Some(session_id.to_string());
        Ok(())
    }

    pub async fn update_session(&self, session: &Session) -> Result<(), SessionError> {
        self.db.save(session)?;
        self.sessions.write().await.insert(session.id.clone(), session.clone());
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        self.db.delete(session_id)?;
        self.sessions.write().await.remove(session_id);
        if *self.current_session_id.read().await == Some(session_id.to_string()) {
            *self.current_session_id.write().await = None;
        }
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<(String, String)> {
        if let Ok(sessions) = self.db.load_all() {
            sessions.into_iter().map(|s| (s.id.clone(), s.name.clone())).collect()
        } else {
            Vec::new()
        }
    }
}
```

- [ ] **Step 2: Add uuid and tokio dependency to swiftforge/Cargo.toml**

```toml
uuid = { version = "1.0", features = ["v4"] }
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p swiftforge --lib 2>&1 | tail -15`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add swiftforge/src/core/session_manager.rs swiftforge/Cargo.toml
git commit -m "feat(core): add SessionManager for multi-session support"
```

---

## Task 5: Implement Session.compact() summarization

**Files:**
- Modify: `libs/swiftforge-types/src/session.rs`

- [ ] **Step 1: Add compact method to Session**

Add to `impl Session`:

```rust
pub async fn compact(&mut self, llm_provider: &dyn LLMProvider) -> Result<(), SessionError> {
    if self.messages.len() < 10 {
        return Ok(());
    }

    let history = self.messages.iter()
        .rev()
        .take(50)
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .join("\n");

    let prompt = format!(
        "Summarize this conversation concisely, preserving key information, decisions, and context.\n\
        Keep the summary under 500 tokens.\n\
        Conversation:\n{}",
        history
    );

    let summary_response = llm_provider.chat(vec![
        Message { role: "user".to_string(), content: prompt }
    ]).await
    .map_err(|e| SessionError::CompactFailed(e.to_string()))?;

    let recent_messages: Vec<Message> = self.messages.iter().rev().take(5).cloned().collect();

    self.messages.clear();
    self.messages.push_back(Message {
        role: "system".to_string(),
        content: format!(
            "[Previous conversation summarized: {}]\n\n[Last {} messages preserved]",
            summary_response.content.trim(),
            recent_messages.len()
        ),
    });

    for msg in recent_messages.into_iter().rev() {
        self.messages.push_back(msg);
    }

    self.token_count = self.estimate_token_count();

    Ok(())
}
```

- [ ] **Step 2: Add LLMProvider import**

```rust
use swiftforge_provider_core::LLMProvider;
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p swiftforge-types 2>&1 | tail -15`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add libs/swiftforge-types/src/session.rs
git commit -m "feat(types): implement Session.compact() summarization"
```

---

## Task 6: Modify Agent.run_agent_loop to use SessionManager

**Files:**
- Modify: `swiftforge/src/core/agent.rs`

- [ ] **Step 1: Read current run_agent_loop**

Run: `grep -n "pub async fn run_agent_loop" swiftforge/src/core/agent.rs`
Note the line number and read the method

- [ ] **Step 2: Modify run_agent_loop signature**

Change from:
```rust
pub async fn run_agent_loop(&self, initial_message: &str, max_iterations: usize, stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>) -> Result<String>
```

To:
```rust
pub async fn run_agent_loop(&self, session: Arc<RwLock<Session>>, initial_message: &str, max_iterations: usize, stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>) -> Result<String>
```

- [ ] **Step 3: Update message handling**

Replace `messages: Vec<Message>` with:
```rust
let messages = session.read().await.messages();
```

And replace `messages.push(...)` with:
```rust
session.write().await.add_message(role, content);
```

- [ ] **Step 4: Add compact call after tool execution**

After `for result in results` loop, add:
```rust
let session = session.read().await;
if session.needs_compaction() {
    drop(session);
    let mut session = session.write().await;
    if let Err(e) = session.compact(self.llm_provider.as_ref()).await {
        warn!("[session]", "Compact failed: {}", e);
    }
}
```

- [ ] **Step 5: Build to verify**

Run: `cargo build -p swiftforge --lib 2>&1 | tail -20`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add swiftforge/src/core/agent.rs
git commit -m "feat(agent): integrate SessionManager into run_agent_loop"
```

---

## Task 7: Update AppController to create/use SessionManager

**Files:**
- Modify: `swiftforge/src/tui/app_controller.rs`

- [ ] **Step 1: Add SessionManager to AppController**

Add field:
```rust
session_manager: Option<Arc<SessionManager>>,
```

- [ ] **Step 2: Initialize in new()**

```rust
let session_manager = if let Some(data_dir) = config.get_session_data_dir() {
    Some(Arc::new(SessionManager::new(data_dir)?))
} else {
    None
};
```

- [ ] **Step 3: Pass session to run_agent_loop**

```rust
if let Some(ref sm) = self.session_manager {
    if let Some(session) = sm.get_current_session().await {
        let session = Arc::new(tokio::sync::RwLock::new(session));
        let result = self.context.agent.run_agent_loop(
            session,
            input,
            10,
            tx.clone()
        ).await;
    }
}
```

- [ ] **Step 4: Build to verify**

Run: `cargo build --bin ragent 2>&1 | tail -20`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add swiftforge/src/tui/app_controller.rs
git commit -m "feat(tui): integrate SessionManager into AppController"
```

---

## Verification Checklist

- [ ] `cargo build --workspace` succeeds
- [ ] Session creates with UUID and name
- [ ] Session persists to SQLite on save
- [ ] Session loads from SQLite
- [ ] Compact triggers when token_count > 80% threshold
- [ ] Agent loop uses session messages
- [ ] Messages accumulate in session after each turn

---

**Plan complete and saved to `docs/specs/2026-06-02-session-management-implementation-plan.md`**

两个执行选项：

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

你想用哪个方式？
