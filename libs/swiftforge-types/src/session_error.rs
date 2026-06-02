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
