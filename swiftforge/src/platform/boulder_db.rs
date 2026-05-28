use crate::platform::boulder::{Boulder, BoulderPriority, BoulderStatus};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct BoulderDatabase {
    conn: Mutex<Connection>,
}

fn parse_status(s: &str) -> BoulderStatus {
    match s {
        "pending" => BoulderStatus::Pending,
        "in_progress" => BoulderStatus::InProgress,
        "completed" => BoulderStatus::Completed,
        "cancelled" => BoulderStatus::Cancelled,
        _ => BoulderStatus::Pending,
    }
}

fn parse_priority(s: &str) -> BoulderPriority {
    match s {
        "low" => BoulderPriority::Low,
        "medium" => BoulderPriority::Medium,
        "high" => BoulderPriority::High,
        _ => BoulderPriority::Medium,
    }
}

impl BoulderDatabase {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open database")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS boulders (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                tags TEXT
            )",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn save(&self, boulder: &Boulder) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO boulders (id, content, status, priority, created_at, updated_at, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                boulder.id,
                boulder.content,
                boulder.status.to_string(),
                boulder.priority.to_string(),
                boulder.created_at,
                boulder.updated_at,
                boulder.tags.join(",")
            ],
        )?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<Option<Boulder>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, status, priority, created_at, updated_at, tags FROM boulders WHERE id = ?1"
        )?;

        let result = stmt.query_row(params![id], |row| {
            let tags_str: String = row.get(6)?;
            let tags = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(',').map(|s| s.to_string()).collect()
            };

            let status_str: String = row.get(2)?;
            let priority_str: String = row.get(3)?;

            let status = parse_status(&status_str);
            let priority = parse_priority(&priority_str);

            Ok(Boulder {
                id: row.get(0)?,
                content: row.get(1)?,
                status,
                priority,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                tags,
            })
        });

        match result {
            Ok(boulder) => Ok(Some(boulder)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_all(&self) -> Result<Vec<Boulder>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content, status, priority, created_at, updated_at, tags FROM boulders ORDER BY created_at DESC"
        )?;

        let boulders = stmt.query_map([], |row| {
            let tags_str: String = row.get(6)?;
            let tags = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(',').map(|s| s.to_string()).collect()
            };

            let status_str: String = row.get(2)?;
            let priority_str: String = row.get(3)?;

            let status = parse_status(&status_str);
            let priority = parse_priority(&priority_str);

            Ok(Boulder {
                id: row.get(0)?,
                content: row.get(1)?,
                status,
                priority,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                tags,
            })
        })?;

        boulders
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM boulders WHERE id = ?1", params![id])?;
        Ok(())
    }
}
