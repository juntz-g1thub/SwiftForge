use rusqlite::{params, Connection};
use std::collections::VecDeque;
use std::path::PathBuf;
use swiftforge_types::{Message, Session, SessionError};

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
            let messages: VecDeque<Message> = serde_json::from_str(&messages_json)
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
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let messages_json: String = row.get(2)?;
            let messages: VecDeque<Message> = serde_json::from_str(&messages_json)
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
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }
}
