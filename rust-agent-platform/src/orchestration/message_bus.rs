use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub trait MessageHandler: Send + Sync {
    fn handle(&self, message: AgentMessage) -> Result<()>;
}

pub struct MessageBus {
    handlers: Arc<RwLock<HashMap<String, Vec<Arc<dyn MessageHandler>>>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, agent_id: &str, handler: Arc<dyn MessageHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers
            .entry(agent_id.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
    }

    pub async fn unsubscribe(&self, agent_id: &str) {
        let mut handlers = self.handlers.write().await;
        handlers.remove(agent_id);
    }

    pub async fn send(&self, message: AgentMessage) -> Result<()> {
        let handlers = self.handlers.read().await;
        if let Some(agent_handlers) = handlers.get(&message.to) {
            for handler in agent_handlers {
                handler.handle(message.clone())?;
            }
        }
        Ok(())
    }

    pub async fn broadcast(&self, from: &str, subject: &str, body: &str) -> Result<()> {
        let handlers = self.handlers.read().await;
        for (agent_id, agent_handlers) in handlers.iter() {
            if agent_id != from {
                for handler in agent_handlers {
                    handler.handle(AgentMessage {
                        from: from.to_string(),
                        to: agent_id.clone(),
                        subject: subject.to_string(),
                        body: body.to_string(),
                    })?;
                }
            }
        }
        Ok(())
    }
}