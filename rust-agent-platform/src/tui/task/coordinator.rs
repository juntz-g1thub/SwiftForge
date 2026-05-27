use crate::tui::task::events::{CoordinatorEvent, TaskType};
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Debug)]
pub struct TaskCoordinator {
    state: CoordinatorState,
    event_queue: VecDeque<CoordinatorEvent>,
}

#[derive(Debug, Clone)]
pub enum CoordinatorState {
    Idle,
    Managing {
        active_task_id: Uuid,
        pending_queue: VecDeque<QueuedTask>,
    },
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: Uuid,
    pub task_type: TaskType,
}

impl TaskCoordinator {
    pub fn new() -> Self {
        Self {
            state: CoordinatorState::Idle,
            event_queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, task_type: TaskType) -> Uuid {
        let task_id = Uuid::new_v4();
        let event = CoordinatorEvent::TaskEnqueued { task_id, task_type };
        self.event_queue.push_back(event.clone());
        self.process_event(event);
        task_id
    }

    fn process_event(&mut self, event: CoordinatorEvent) {
        match event {
            CoordinatorEvent::TaskEnqueued { task_id, task_type } => match &mut self.state {
                CoordinatorState::Idle => {
                    self.state = CoordinatorState::Managing {
                        active_task_id: task_id,
                        pending_queue: VecDeque::new(),
                    };
                }
                CoordinatorState::Managing { pending_queue, .. } => {
                    pending_queue.push_back(QueuedTask { task_id, task_type });
                }
            },
            CoordinatorEvent::TaskCompleted { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskCancelled { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            _ => {}
        }
    }

    fn transition_to_idle_or_next(&mut self) {
        match &mut self.state {
            CoordinatorState::Managing { pending_queue, .. } => {
                if let Some(next) = pending_queue.pop_front() {
                    self.state = CoordinatorState::Managing {
                        active_task_id: next.task_id,
                        pending_queue: pending_queue.clone(),
                    };
                } else {
                    self.state = CoordinatorState::Idle;
                }
            }
            _ => {}
        }
    }

    pub fn state(&self) -> &CoordinatorState {
        &self.state
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, CoordinatorState::Idle)
    }
}

impl Default for TaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
