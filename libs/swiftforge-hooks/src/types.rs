use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookEvent {
    OnStartup,
    OnShutdown,
    OnError(String),
    OnWarning(String),
    OnInfo(String),
    OnDebug(String),
    OnAgentCreated(String),
    OnAgentDestroyed(String),
    OnSessionStart,
    OnSessionEnd,
    OnMessageReceived(String),
    OnMessageSent(String),
    OnToolCall(String),
    OnToolResult(String, bool),
    OnProviderCall(String),
    OnProviderResponse(bool),
}

#[derive(Debug, Clone)]
pub struct HookContext {
    pub event: HookEvent,
    pub timestamp: std::time::Instant,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HookContext {
    pub fn new(event: HookEvent) -> Self {
        Self {
            event,
            timestamp: std::time::Instant::now(),
            metadata: HashMap::new(),
        }
    }
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}
