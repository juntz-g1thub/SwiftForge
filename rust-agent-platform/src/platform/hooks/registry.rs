use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{HookContext, HookEvent};

pub type HookFn = Arc<dyn Fn(HookContext) -> Result<(), anyhow::Error> + Send + Sync>;

pub struct HookRegistry {
    hooks: RwLock<HashMap<String, Vec<(HookFn, i32)>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, event_name: &str, priority: i32, hook: HookFn) {
        let mut hooks = self.hooks.write().await;
        hooks
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push((hook, priority));
    }

    pub async fn dispatch(&self, event: HookEvent) -> Result<(), anyhow::Error> {
        let event_name = format!("{:?}", event);
        let hooks = self.hooks.read().await;
        
        if let Some(event_hooks) = hooks.get(&event_name) {
            let context = HookContext::new(event);
            for (hook, _) in event_hooks.iter() {
                hook(context.clone())?;
            }
        }
        Ok(())
    }

    pub async fn list_hooks(&self) -> Vec<String> {
        let hooks = self.hooks.read().await;
        hooks.keys().cloned().collect()
    }
}