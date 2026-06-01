use std::sync::Arc;
use anyhow::{Result, anyhow};
use std::sync::Mutex;
use swiftforge_types::ToolRegistry;
use crate::pool::McpConnectionPool;
use crate::adapter::McpToolAdapter;

pub struct McpToolLoader {
    pool: Arc<McpConnectionPool>,
    registry: Arc<Mutex<ToolRegistry>>,
}

impl McpToolLoader {
    pub fn new(pool: Arc<McpConnectionPool>, registry: Arc<ToolRegistry>) -> Self {
        Self { pool, registry: Arc::new(Mutex::new(registry.as_ref().clone())) }
    }

    pub async fn load_tools(&self, server_name: &str) -> Result<usize> {
        let client = self.pool.client(server_name).await
            .ok_or_else(|| anyhow!("Server '{}' not found", server_name))?;

        let tools = client.list_tools().await?;
        let count = tools.len();

        let mut registry = self.registry.lock().unwrap();
        for tool_def in tools {
            let adapter = McpToolAdapter::new(
                server_name,
                client.clone(),
                tool_def.name,
                tool_def.description,
                tool_def.input_schema,
            );
            registry.register(adapter);
        }

        Ok(count)
    }

    pub async fn load_all(&self) -> Result<usize> {
        let mut total = 0;
        for server_name in self.pool.list_servers().await {
            match self.load_tools(&server_name).await {
                Ok(count) => total += count,
                Err(e) => {
                    tracing::warn!("[mcp] Failed to load tools from '{}': {}", server_name, e);
                }
            }
        }
        Ok(total)
    }
}
