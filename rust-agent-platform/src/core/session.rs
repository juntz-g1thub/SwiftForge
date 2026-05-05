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
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct Session {
    messages: VecDeque<Message>,
    #[allow(dead_code)]
    context_window: usize,
}

impl Session {
    pub fn new(context_window: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            context_window,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push_back(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn messages(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }
}
