use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::RwLock;
use crate::client::MCPClient;

#[derive(Clone)]
pub struct McpConnectionPool {
    clients: Arc<RwLock<HashMap<String, Arc<MCPClient>>>>,
    default_server: Arc<RwLock<Option<String>>>,
}

impl McpConnectionPool {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            default_server: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn add_server(&self, name: &str, url: &str) -> Result<()> {
        let client = Arc::new(MCPClient::new(url.to_string()));
        self.clients.write().await.insert(name.to_string(), client);

        let mut default = self.default_server.write().await;
        if default.is_none() {
            *default = Some(name.to_string());
        }

        Ok(())
    }

    pub async fn connect(&self, name: &str) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        client.connect().await
    }

    pub async fn initialize(&self, name: &str, client_name: &str, version: &str) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(name)
            .ok_or_else(|| anyhow!("MCP server '{}' not found", name))?;
        let _ = client.initialize(client_name, version).await?;
        Ok(())
    }

    pub async fn client(&self, name: &str) -> Option<Arc<MCPClient>> {
        self.clients.read().await.get(name).cloned()
    }

    pub async fn default_client(&self) -> Option<Arc<MCPClient>> {
        let default = self.default_server.read().await;
        let name = default.as_ref()?;
        Some(self.clients.read().await.get(name)?.clone())
    }

    pub async fn list_servers(&self) -> Vec<String> {
        self.clients.read().await.keys().cloned().collect()
    }
}

impl Default for McpConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}