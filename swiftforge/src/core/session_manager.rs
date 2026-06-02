use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use swiftforge_types::{Session, SessionError};
use crate::core::session_db::SessionDatabase;

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    current_session_id: Arc<RwLock<Option<String>>>,
    db: Arc<SessionDatabase>,
    data_dir: PathBuf,
}

impl SessionManager {
    pub fn new(data_dir: PathBuf) -> Result<Self, SessionError> {
        std::fs::create_dir_all(&data_dir).map_err(|e| SessionError::SaveFailed(e.to_string()))?;
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
        let mut current_id = self.current_session_id.write().await;
        if *current_id == Some(session_id.to_string()) {
            *current_id = None;
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