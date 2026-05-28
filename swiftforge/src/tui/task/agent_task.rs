use crate::tui::task::events::{AgentTaskState, FailedState, RunningState, TaskEvent, TaskType};
use uuid::Uuid;

#[derive(Debug)]
pub struct AgentTask {
    pub task_id: Uuid,
    pub task_type: TaskType,
    pub state: AgentTaskState,
}

impl AgentTask {
    pub fn new(task_id: Uuid, task_type: TaskType) -> Self {
        Self {
            task_id,
            task_type,
            state: AgentTaskState::Pending,
        }
    }

    pub fn transition(&mut self, new_state: AgentTaskState) {
        self.state = new_state;
    }

    pub fn can_retry(&self) -> bool {
        self.task_type.max_retries() > 0
    }

    pub fn max_retries(&self) -> u8 {
        self.task_type.max_retries()
    }
}

#[derive(Debug, Clone)]
pub struct AgentTaskHandle {
    pub task_id: Uuid,
    pub task_type: TaskType,
}

impl AgentTaskHandle {
    pub fn new(task_id: Uuid, task_type: TaskType) -> Self {
        Self { task_id, task_type }
    }

    pub fn is_terminal(&self) -> bool {
        false
    }
}
