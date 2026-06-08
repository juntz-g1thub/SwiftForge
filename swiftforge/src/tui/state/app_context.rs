use std::sync::{mpsc, Arc, Mutex};

use crate::core::{Agent, ToolRegistry};
use crate::tui::config::ConfigManager;

#[derive(Clone)]
pub struct AppContext {
    pub agent: Arc<Agent>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub tool_registry: Arc<ToolRegistry>,
}

impl AppContext {
    pub fn new(agent: Agent, config: ConfigManager, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            agent: Arc::new(agent),
            config: Arc::new(Mutex::new(config)),
            tool_registry,
        }
    }
}

pub struct UIState {
    pub streaming_text: Arc<Mutex<Option<String>>>,
    pub response_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<String, anyhow::Error>>>>>,
    pub agent_command_tx: Arc<Mutex<Option<mpsc::Sender<AgentCommand>>>>,
    pub finalized_message: Arc<Mutex<Option<(String, String)>>>,
    pub finalized_reasoning: Arc<Mutex<Option<String>>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            streaming_text: Arc::new(Mutex::new(None)),
            response_receiver: Arc::new(Mutex::new(None)),
            agent_command_tx: Arc::new(Mutex::new(None)),
            finalized_message: Arc::new(Mutex::new(None)),
            finalized_reasoning: Arc::new(Mutex::new(None)),
        }
    }

    pub fn append_streaming(&self, chunk: &str) {
        if let Ok(mut streaming) = self.streaming_text.lock() {
            if let Some(ref mut text) = *streaming {
                text.push_str(chunk);
            } else {
                *streaming = Some(chunk.to_string());
            }
        }
    }

    pub fn clear_streaming(&self) {
        if let Ok(mut streaming) = self.streaming_text.lock() {
            *streaming = None;
        }
    }
}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

pub enum AgentCommand {
    SendMessage(String),
    CancelCurrentRequest,
}
