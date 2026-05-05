use crate::core::{Agent, AgentConfig};

pub struct OrchestratedAgent {
    agent: Agent,
    agent_id: String,
    status: AgentStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Busy,
    Offline,
}

impl OrchestratedAgent {
    pub fn new(agent_id: String, config: AgentConfig) -> Self {
        Self {
            agent: Agent::new(config),
            agent_id,
            status: AgentStatus::Idle,
        }
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
