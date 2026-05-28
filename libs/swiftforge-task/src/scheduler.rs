use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub assigned_to: Option<String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

pub struct TaskScheduler {
    tasks: Arc<RwLock<VecDeque<Task>>>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self { tasks: Arc::new(RwLock::new(VecDeque::new())) }
    }
    pub async fn add_task(&self, task: Task) {
        let mut tasks = self.tasks.write().await;
        let pos = tasks.iter().position(|t| t.priority < task.priority).unwrap_or(tasks.len());
        tasks.insert(pos, task);
    }
    pub async fn get_next_task(&self) -> Option<Task> {
        let mut tasks = self.tasks.write().await;
        let mut task = tasks.pop_front()?;
        task.status = TaskStatus::Running;
        Some(task)
    }
    pub async fn complete_task(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        for task in tasks.iter_mut() {
            if task.id == task_id {
                task.status = TaskStatus::Completed;
                break;
            }
        }
    }
    pub async fn fail_task(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        for task in tasks.iter_mut() {
            if task.id == task_id {
                task.status = TaskStatus::Failed;
                break;
            }
        }
    }
    pub async fn list_pending(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.iter().filter(|t| t.status == TaskStatus::Pending).cloned().collect()
    }
}