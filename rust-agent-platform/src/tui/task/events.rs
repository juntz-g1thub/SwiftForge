use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum TaskType {
    UserMessage { priority: u8 },
    SystemTask { priority: u8 },
    BackgroundFetch { priority: u8 },
}

impl TaskType {
    pub fn priority(&self) -> u8 {
        match self {
            TaskType::UserMessage { priority } => *priority,
            TaskType::SystemTask { priority } => *priority,
            TaskType::BackgroundFetch { priority } => *priority,
        }
    }

    pub fn max_retries(&self) -> u8 {
        match self {
            TaskType::UserMessage { .. } => 3,
            TaskType::SystemTask { .. } => 2,
            TaskType::BackgroundFetch { .. } => 0,
        }
    }
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::SystemTask { priority: 50 }
    }
}

#[derive(Debug, Clone)]
pub enum FailedState {
    NetworkError {
        message: String,
        retry_count: u8,
    },
    ProviderError {
        message: String,
        provider: String,
        retry_count: u8,
    },
    Timeout {
        waited_secs: u64,
        limit_secs: u64,
    },
    Cancelled,
}

impl FailedState {
    pub fn is_retriable(&self) -> bool {
        match self {
            FailedState::NetworkError { retry_count, .. } => *retry_count > 0,
            FailedState::ProviderError { retry_count, .. } => *retry_count > 0,
            FailedState::Timeout { .. } => true,
            FailedState::Cancelled => false,
        }
    }

    pub fn with_decrement(&self) -> Self {
        match self {
            FailedState::NetworkError {
                message,
                retry_count,
            } => FailedState::NetworkError {
                message: message.clone(),
                retry_count: *retry_count - 1,
            },
            FailedState::ProviderError {
                message,
                provider,
                retry_count,
            } => FailedState::ProviderError {
                message: message.clone(),
                provider: provider.clone(),
                retry_count: *retry_count - 1,
            },
            other => other.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunningState {
    Initializing,
    WaitingProvider,
    StreamingChunks,
    ProcessingToolCall,
}

#[derive(Debug, Clone)]
pub enum AgentTaskState {
    Pending,
    Running(RunningState),
    Completed,
    Failed(FailedState),
    Suspended,
    Cancelled,
}

impl AgentTaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentTaskState::Completed | AgentTaskState::Failed(_) | AgentTaskState::Cancelled
        )
    }
}

#[derive(Debug, Clone)]
pub enum CoordinatorState {
    Idle,
    Managing {
        active_task_id: Uuid,
        pending_queue: Vec<QueuedTask>,
    },
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: Uuid,
    pub task_type: TaskType,
    pub state: AgentTaskState,
}

#[derive(Debug, Clone)]
pub enum CoordinatorEvent {
    TaskEnqueued {
        task_id: Uuid,
        task_type: TaskType,
    },
    TaskStarted {
        task_id: Uuid,
    },
    TaskCompleted {
        task_id: Uuid,
    },
    TaskCancelled {
        task_id: Uuid,
    },
    Preempted {
        high_priority: Uuid,
        low_priority: Uuid,
    },
    TaskSuspended {
        task_id: Uuid,
    },
    TaskResumed {
        task_id: Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum TaskEvent {
    Spawned,
    Chunk { content: String },
    ToolCall { name: String, input: String },
    ToolResult { name: String, result: String },
    Completed { content: String },
    Failed { state: FailedState },
    Suspended,
    Resumed,
}
