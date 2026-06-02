use rust_agent_platform::core::AgentConfig;
use rust_agent_platform::orchestration::{
    AgentMessage, AgentStatus, MessageBus, OrchestratedAgent, Task, TaskPriority, TaskScheduler,
    TaskStatus,
};

#[tokio::test]
async fn test_task_scheduler_creation() {
    let scheduler = TaskScheduler::new();
    let pending = scheduler.list_pending().await;
    assert!(pending.is_empty());
}

#[tokio::test]
async fn test_add_task() {
    let scheduler = TaskScheduler::new();
    let task = Task {
        id: "1".to_string(),
        description: "Test task".to_string(),
        priority: TaskPriority::Normal,
        assigned_to: None,
        status: TaskStatus::Pending,
    };
    scheduler.add_task(task).await;
    let pending = scheduler.list_pending().await;
    assert_eq!(pending.len(), 1);
}

#[tokio::test]
async fn test_get_next_task() {
    let scheduler = TaskScheduler::new();
    let task = Task {
        id: "1".to_string(),
        description: "Test task".to_string(),
        priority: TaskPriority::Normal,
        assigned_to: None,
        status: TaskStatus::Pending,
    };
    scheduler.add_task(task).await;
    let next = scheduler.get_next_task().await;
    assert!(next.is_some());
    assert_eq!(next.unwrap().status, TaskStatus::Running);
}

#[tokio::test]
async fn test_message_bus_creation() {
    let bus = MessageBus::new();
}

#[tokio::test]
async fn test_orchestrated_agent_status() {
    let config = AgentConfig {
        name: "test".to_string(),
        role: rust_agent_platform::core::AgentRole::Orchestrator,
        model: None,
        temperature: 0.1,
    };
    let mut agent = OrchestratedAgent::new("agent-1".to_string(), config);
    assert_eq!(agent.status(), AgentStatus::Idle);
    agent.set_busy();
    assert_eq!(agent.status(), AgentStatus::Busy);
    agent.set_idle();
    assert_eq!(agent.status(), AgentStatus::Idle);
}
