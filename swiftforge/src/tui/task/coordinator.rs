use crate::tui::task::events::{CoordinatorEvent, CoordinatorState, QueuedTask, TaskType};
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Debug)]
pub struct TaskCoordinator {
    state: CoordinatorState,
    event_queue: VecDeque<CoordinatorEvent>,
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
        let priority = task_type.priority();

        let should_preempt = match &self.state {
            CoordinatorState::Managing {
                active_task_id,
                pending_queue,
            } => {
                if active_task_id.is_some() {
                    let active_priority = pending_queue
                        .back()
                        .map(|q| q.task_type.priority())
                        .unwrap_or(50);
                    priority > active_priority
                } else {
                    false
                }
            }
            _ => false,
        };

        if should_preempt {
            if let CoordinatorState::Managing {
                active_task_id,
                pending_queue,
            } = &mut self.state
            {
                if let Some(active_id) = active_task_id.take() {
                    pending_queue.push_back(QueuedTask {
                        task_id: active_id,
                        task_type: TaskType::BackgroundFetch { priority: 10 },
                        state: crate::tui::task::events::AgentTaskState::Suspended,
                    });
                }
            }
        }

        let event = CoordinatorEvent::TaskEnqueued { task_id, task_type };
        self.event_queue.push_back(event.clone());
        self.process_event(event);
        task_id
    }

    pub fn process_event(&mut self, event: CoordinatorEvent) {
        match event {
            CoordinatorEvent::TaskEnqueued { task_id, task_type } => {
                let task_type_clone = task_type.clone();
                match &mut self.state {
                    CoordinatorState::Idle => {
                        self.state = CoordinatorState::Managing {
                            active_task_id: Some(task_id),
                            pending_queue: VecDeque::new(),
                        };
                    }
                    CoordinatorState::Managing { pending_queue, .. } => {
                        let mut inserted = false;
                        for (i, qtask) in pending_queue.iter().enumerate() {
                            if qtask.task_type.priority() < task_type.priority() {
                                pending_queue.insert(
                                    i,
                                    QueuedTask {
                                        task_id,
                                        task_type: task_type_clone,
                                        state: crate::tui::task::events::AgentTaskState::Pending,
                                    },
                                );
                                inserted = true;
                                break;
                            }
                        }
                        if !inserted {
                            pending_queue.push_back(QueuedTask {
                                task_id,
                                task_type,
                                state: crate::tui::task::events::AgentTaskState::Pending,
                            });
                        }
                    }
                }
            }
            CoordinatorEvent::TaskCompleted { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskCancelled { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::Preempted {
                high_priority: _,
                low_priority: _,
            } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskSuspended { task_id: _ } => {
                self.transition_to_idle_or_next();
            }
            CoordinatorEvent::TaskResumed { task_id: _ } => {}
            _ => {}
        }
    }

    fn transition_to_idle_or_next(&mut self) {
        match &mut self.state {
            CoordinatorState::Managing {
                active_task_id,
                pending_queue,
            } => {
                *active_task_id = None;
                if let Some(next) = pending_queue.pop_front() {
                    *active_task_id = Some(next.task_id);
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

    pub fn active_task_id(&self) -> Option<Uuid> {
        match &self.state {
            CoordinatorState::Managing { active_task_id, .. } => *active_task_id,
            _ => None,
        }
    }

    pub fn pending_count(&self) -> usize {
        match &self.state {
            CoordinatorState::Managing { pending_queue, .. } => pending_queue.len(),
            _ => 0,
        }
    }
}

impl Default for TaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
