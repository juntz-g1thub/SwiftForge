use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::boulder_db::BoulderDatabase;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BoulderStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl std::fmt::Display for BoulderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoulderStatus::Pending => write!(f, "pending"),
            BoulderStatus::InProgress => write!(f, "in_progress"),
            BoulderStatus::Completed => write!(f, "completed"),
            BoulderStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for BoulderStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(BoulderStatus::Pending),
            "in_progress" => Ok(BoulderStatus::InProgress),
            "completed" => Ok(BoulderStatus::Completed),
            "cancelled" => Ok(BoulderStatus::Cancelled),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BoulderPriority {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for BoulderPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoulderPriority::Low => write!(f, "low"),
            BoulderPriority::Medium => write!(f, "medium"),
            BoulderPriority::High => write!(f, "high"),
        }
    }
}

impl std::str::FromStr for BoulderPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(BoulderPriority::Low),
            "medium" => Ok(BoulderPriority::Medium),
            "high" => Ok(BoulderPriority::High),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Boulder {
    pub id: String,
    pub content: String,
    pub status: BoulderStatus,
    pub priority: BoulderPriority,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

pub struct BoulderStore {
    db: BoulderDatabase,
}

impl BoulderStore {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("boulders.db");
        let db = BoulderDatabase::new(&db_path)?;
        Ok(Self { db })
    }

    pub fn save(&self, boulder: &Boulder) -> Result<()> {
        self.db.save(boulder)
    }

    pub fn load(&self, id: &str) -> Result<Option<Boulder>> {
        self.db.load(id)
    }

    pub fn list(&self) -> Result<Vec<Boulder>> {
        self.db.list_all()
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.delete(id)
    }

    pub fn create(
        &self,
        content: String,
        priority: BoulderPriority,
        tags: Vec<String>,
    ) -> Result<Boulder> {
        let now = chrono_lite_now();
        let id = uuid_v4();
        let boulder = Boulder {
            id,
            content,
            status: BoulderStatus::Pending,
            priority,
            created_at: now.clone(),
            updated_at: now,
            tags,
        };
        self.save(&boulder)?;
        Ok(boulder)
    }

    pub fn update_status(&self, id: &str, status: BoulderStatus) -> Result<()> {
        if let Some(mut boulder) = self.load(id)? {
            boulder.status = status;
            boulder.updated_at = chrono_lite_now();
            self.save(&boulder)?;
        }
        Ok(())
    }
}

fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

fn uuid_v4() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    use std::time::SystemTime;
    let rs = RandomState::new();
    let mut hasher = rs.build_hasher();
    SystemTime::now().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
