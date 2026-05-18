use crate::core::tool::ToolRegistry;
use crate::core::{Agent as CoreAgent, AgentConfig};
use crate::orchestration::{MessageBus, TaskScheduler};
use crate::providers::LLMProvider;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Busy,
    Offline,
}

pub struct OrchestratedAgent {
    agent: CoreAgent,
    agent_id: String,
    status: AgentStatus,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    tool_registry: Option<Arc<ToolRegistry>>,
}

impl OrchestratedAgent {
    pub fn new(agent_id: String, config: AgentConfig) -> Self {
        let agent = CoreAgent::new(config);
        Self {
            agent,
            agent_id,
            status: AgentStatus::Idle,
            scheduler: None,
            message_bus: None,
            tool_registry: None,
        }
    }

    pub fn with_scheduler(mut self, scheduler: Arc<TaskScheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    pub fn with_message_bus(mut self, message_bus: Arc<MessageBus>) -> Self {
        self.message_bus = Some(message_bus);
        self
    }

    pub fn with_tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    pub fn with_provider<P: LLMProvider + 'static>(self, name: &str, provider: P) -> Self {
        Self {
            agent: self.agent.with_provider(name, provider),
            ..self
        }
    }

    pub fn build_agent(self) -> CoreAgent {
        let mut agent = self.agent;
        if let Some(s) = self.scheduler {
            agent = agent.with_scheduler(s);
        }
        if let Some(m) = self.message_bus {
            agent = agent.with_message_bus(m);
        }
        if let Some(t) = self.tool_registry {
            agent = agent.with_tool_registry(t);
        }
        agent
    }

    pub fn id(&self) -> &str {
        &self.agent_id
    }

    pub fn status(&self) -> AgentStatus {
        self.status.clone()
    }

    pub fn set_busy(&mut self) {
        self.status = AgentStatus::Busy;
    }

    pub fn set_idle(&mut self) {
        self.status = AgentStatus::Idle;
    }

    pub fn set_offline(&mut self) {
        self.status = AgentStatus::Offline;
    }
}
