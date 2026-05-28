use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, Context, anyhow};
use tokio::sync::RwLock;
use reqwest::Client;
use serde_json::Value as JsonValue;
use super::protocol::{
    JsonRpcRequest, JsonRpcResponse,
    InitializeParams, InitializeResult, ClientCapabilities, ClientInfo, ServerCapabilities,
    ToolsListResult, Tool,
    ToolCallParams, ToolCallResult, ContentBlock,
};

pub struct MCPClient {
    server_url: String,
    client: Client,
    connected: Arc<RwLock<bool>>,
    initialized: Arc<RwLock<bool>>,
    server_capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    request_id: Arc<RwLock<u32>>,
}

impl MCPClient {
    pub fn new(server_url: String) -> Self {
        let client = Client::builder().timeout(Duration::from_secs(30)).build().expect("Failed to create HTTP client");
        Self {
            server_url,
            client,
            connected: Arc::new(RwLock::new(false)),
            initialized: Arc::new(RwLock::new(false)),
            server_capabilities: Arc::new(RwLock::new(None)),
            request_id: Arc::new(RwLock::new(1)),
        }
    }
    async fn next_id(&self) -> u32 {
        let mut id = self.request_id.write().await;
        let current = *id;
        *id = current + 1;
        current
    }
    async fn send_request(&self, method: &str, params: Option<JsonValue>) -> Result<JsonValue> {
        let id = self.next_id().await;
        let request = JsonRpcRequest::new(id, method);
        let request = if let Some(p) = params { request.with_params(p) } else { request };
        let response = self.client.post(&self.server_url).json(&request).send().await.context("Failed to send HTTP request")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("MCP server returned error: {} - {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown"));
        }
        let rpc_response: JsonRpcResponse = response.json().await.context("Failed to parse JSON-RPC response")?;
        if rpc_response.jsonrpc != "2.0" {
            anyhow::bail!("Invalid JSON-RPC version: {}", rpc_response.jsonrpc);
        }
        if rpc_response.id != id {
            anyhow::bail!("Response ID mismatch: expected {}, got {}", id, rpc_response.id);
        }
        if let Some(error) = rpc_response.error {
            anyhow::bail!("JSON-RPC error (code={}): {}", error.code, error.message);
        }
        rpc_response.result.ok_or_else(|| anyhow!("No result in JSON-RPC response"))
    }
    pub async fn connect(&self) -> Result<()> {
        if !self.is_connected().await {
            self.client.get(&format!("{}/health", self.server_url)).timeout(Duration::from_secs(5)).send().await.context("MCP server health check failed")?;
        }
        *self.connected.write().await = true;
        Ok(())
    }
    pub async fn disconnect(&self) -> Result<()> {
        *self.connected.write().await = false;
        *self.initialized.write().await = false;
        *self.server_capabilities.write().await = None;
        Ok(())
    }
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
    pub async fn initialize(&self, client_name: &str, client_version: &str) -> Result<ServerCapabilities> {
        if !self.is_connected().await {
            anyhow::bail!("Not connected to MCP server");
        }
        if self.is_initialized().await {
            return self.get_capabilities().await;
        }
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo { name: client_name.to_string(), version: client_version.to_string() },
        };
        let result = self.send_request("initialize", Some(serde_json::to_value(params)?)).await?;
        let init_result: InitializeResult = serde_json::from_value(result).context("Failed to parse initialize result")?;
        *self.server_capabilities.write().await = Some(init_result.capabilities.clone());
        *self.initialized.write().await = true;
        let _ = self.send_request("initialized", None).await;
        Ok(init_result.capabilities)
    }
    pub async fn get_capabilities(&self) -> Result<ServerCapabilities> {
        self.server_capabilities.read().await.clone().ok_or_else(|| anyhow!("Not initialized"))
    }
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        if !self.is_connected().await {
            anyhow::bail!("Not connected to MCP server");
        }
        if !self.is_initialized().await {
            anyhow::bail!("Not initialized - call initialize() first");
        }
        let result = self.send_request("tools/list", None).await?;
        let tools_result: ToolsListResult = serde_json::from_value(result).context("Failed to parse tools/list result")?;
        Ok(tools_result.tools)
    }
    pub async fn call_tool(&self, name: &str, arguments: JsonValue) -> Result<Vec<ContentBlock>> {
        if !self.is_connected().await {
            anyhow::bail!("Not connected to MCP server");
        }
        if !self.is_initialized().await {
            anyhow::bail!("Not initialized - call initialize() first");
        }
        let params = ToolCallParams { name: name.to_string(), arguments };
        let result = self.send_request("tools/call", Some(serde_json::to_value(params)?)).await?;
        let call_result: ToolCallResult = serde_json::from_value(result).context("Failed to parse tools/call result")?;
        Ok(call_result.content)
    }
}