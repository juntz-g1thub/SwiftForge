use swiftforge::tui::task::coordinator::TaskCoordinator;
use swiftforge::tui::task::events::{AgentTaskState, CoordinatorState, TaskType};

#[test]
fn test_task_coordinator_creation() {
    let coordinator = TaskCoordinator::new();
    assert!(coordinator.is_idle());
    assert!(coordinator.active_task_id().is_none());
    assert_eq!(coordinator.pending_count(), 0);
}

#[test]
fn test_task_coordinator_enqueue_user_message() {
    let mut coordinator = TaskCoordinator::new();
    let task_id = coordinator.enqueue(TaskType::UserMessage { priority: 80 });

    assert!(!coordinator.is_idle());
    assert_eq!(coordinator.active_task_id(), Some(task_id));
}

#[test]
fn test_task_coordinator_enqueue_system_task() {
    let mut coordinator = TaskCoordinator::new();
    let task_id = coordinator.enqueue(TaskType::SystemTask { priority: 50 });

    assert!(!coordinator.is_idle());
    assert_eq!(coordinator.active_task_id(), Some(task_id));
}

#[test]
fn test_task_coordinator_multiple_tasks() {
    let mut coordinator = TaskCoordinator::new();

    coordinator.enqueue(TaskType::UserMessage { priority: 80 });
    coordinator.enqueue(TaskType::SystemTask { priority: 60 });
    coordinator.enqueue(TaskType::BackgroundFetch { priority: 30 });

    assert!(!coordinator.is_idle());
    assert!(matches!(
        coordinator.state(),
        CoordinatorState::Managing { .. }
    ));
}

#[test]
fn test_task_type_priority() {
    let high = TaskType::UserMessage { priority: 100 };
    let medium = TaskType::SystemTask { priority: 50 };
    let low = TaskType::BackgroundFetch { priority: 10 };

    assert_eq!(high.priority(), 100);
    assert_eq!(medium.priority(), 50);
    assert_eq!(low.priority(), 10);
}

#[test]
fn test_task_type_max_retries() {
    let user = TaskType::UserMessage { priority: 80 };
    let system = TaskType::SystemTask { priority: 50 };
    let background = TaskType::BackgroundFetch { priority: 10 };

    assert_eq!(user.max_retries(), 3);
    assert_eq!(system.max_retries(), 2);
    assert_eq!(background.max_retries(), 0);
}

#[test]
fn test_agent_task_state_is_terminal() {
    assert!(!AgentTaskState::Pending.is_terminal());
    assert!(
        !AgentTaskState::Running(swiftforge::tui::task::events::RunningState::Initializing)
            .is_terminal()
    );
    assert!(AgentTaskState::Completed.is_terminal());
    assert!(
        AgentTaskState::Failed(swiftforge::tui::task::events::FailedState::Cancelled).is_terminal()
    );
    assert!(AgentTaskState::Cancelled.is_terminal());
    assert!(!AgentTaskState::Suspended.is_terminal());
}

#[test]
fn test_failed_state_is_retriable() {
    let retriable = swiftforge::tui::task::events::FailedState::NetworkError {
        message: "test".to_string(),
        retry_count: 2,
    };
    assert!(retriable.is_retriable());

    let not_retriable = swiftforge::tui::task::events::FailedState::Cancelled;
    assert!(!not_retriable.is_retriable());
}

#[test]
fn test_failed_state_with_decrement() {
    let error = swiftforge::tui::task::events::FailedState::NetworkError {
        message: "test".to_string(),
        retry_count: 2,
    };
    let decremented = error.with_decrement();

    if let swiftforge::tui::task::events::FailedState::NetworkError { retry_count, .. } =
        decremented
    {
        assert_eq!(retry_count, 1);
    } else {
        panic!("Expected NetworkError");
    }
}

#[test]
fn test_coordinator_state_idle() {
    let coordinator = TaskCoordinator::new();
    assert!(matches!(coordinator.state(), CoordinatorState::Idle));
}

#[test]
fn test_coordinator_state_managing() {
    let mut coordinator = TaskCoordinator::new();
    coordinator.enqueue(TaskType::UserMessage { priority: 80 });

    match coordinator.state() {
        CoordinatorState::Managing {
            active_task_id,
            pending_queue,
        } => {
            assert!(active_task_id.is_some());
            assert!(pending_queue.is_empty());
        }
        _ => panic!("Expected Managing state"),
    }
}

#[test]
fn test_task_type_default() {
    let task_type = TaskType::default();
    assert_eq!(task_type.priority(), 50);
    assert_eq!(task_type.max_retries(), 2);
}
