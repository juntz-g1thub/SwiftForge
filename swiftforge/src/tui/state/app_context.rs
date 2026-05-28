use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use crate::core::{Agent, ToolRegistry};
use crate::tui::config::ConfigManager;

#[derive(Clone)]
pub struct AppContext {
    pub agent: Arc<Agent>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub tool_registry: Arc<ToolRegistry>,
    pub debug_log_path: Option<PathBuf>,
}

impl AppContext {
    pub fn new(
        agent: Agent,
        config: ConfigManager,
        tool_registry: Arc<ToolRegistry>,
        show_debug: bool,
    ) -> Self {
        let debug_log_path = if show_debug {
            let log_dir = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".fastcode");
            std::fs::create_dir_all(&log_dir).ok();
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let log_path = log_dir.join(format!("ragent_{}.log", timestamp));
            std::fs::write(&log_path, "").ok();
            Some(log_path)
        } else {
            None
        };

        Self {
            agent: Arc::new(agent),
            config: Arc::new(Mutex::new(config)),
            tool_registry,
            debug_log_path,
        }
    }
}

pub struct UIState {
    pub streaming_text: Arc<Mutex<Option<String>>>,
    pub debug_messages: Arc<Mutex<Vec<String>>>,
    pub response_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<String, anyhow::Error>>>>>,
    pub agent_command_tx: Arc<Mutex<Option<mpsc::Sender<AgentCommand>>>>,
    pub finalized_message: Arc<Mutex<Option<(String, String)>>>,
    pub debug_tx: Arc<Mutex<Option<mpsc::Sender<String>>>>,
}

impl UIState {
    pub fn new() -> Self {
        Self {
            streaming_text: Arc::new(Mutex::new(None)),
            debug_messages: Arc::new(Mutex::new(Vec::new())),
            response_receiver: Arc::new(Mutex::new(None)),
            agent_command_tx: Arc::new(Mutex::new(None)),
            finalized_message: Arc::new(Mutex::new(None)),
            debug_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn add_debug(&self, msg: String) {
        if let Ok(mut messages) = self.debug_messages.lock() {
            messages.push(msg);
            if messages.len() > 100 {
                messages.remove(0);
            }
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
