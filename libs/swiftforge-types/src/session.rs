use crate::ModelResponse;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub context_window: usize,
    pub max_tokens: Option<usize>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            context_window: 100,
            max_tokens: None,
        }
    }
}

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

    pub async fn compact<F, Fut>(&mut self, chat_fn: F) -> Result<(), SessionError>
    where
        F: Fn(Vec<Message>) -> Fut,
        Fut: std::future::Future<Output = Result<ModelResponse, anyhow::Error>>,
    {
        if self.messages.len() < 10 {
            return Ok(());
        }

        let history: String = self.messages.iter()
            .rev()
            .take(50)
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize this conversation concisely, preserving key information, decisions, and context.\n\
            Keep the summary under 500 tokens.\n\
            Conversation:\n{}",
            history
        );

        let summary_response = chat_fn(vec![
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }
    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

pub use crate::session_error::SessionError;
