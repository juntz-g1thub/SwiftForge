use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============================================================================
// JSON-RPC 2.0 Envelope Types
// ============================================================================

/// JSON-RPC 2.0 request envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u32,
    pub method: String,
    #[serde(default)]
    pub params: Option<JsonValue>,
}

/// JSON-RPC 2.0 response envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u32,
    #[serde(default)]
    pub result: Option<JsonValue>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<JsonValue>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self::new(-32602, msg)
    }

    pub fn internal_error(msg: &str) -> Self {
        Self::new(-32603, msg)
    }
}

impl JsonRpcRequest {
    pub fn new(id: u32, method: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: JsonValue) -> Self {
        self.params = Some(params);
        self
    }
}

// ============================================================================
// MCP Message Types (JSON-RPC method names and payloads)
// ============================================================================

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub sampling: Option<()>,
}

/// Client info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Initialize result from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {}

/// Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Tools list request (no params)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListParams;

/// Tools list result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

/// Tool call parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: JsonValue,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
}

/// Content block in tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    pub r#type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

impl ContentBlock {
    pub fn text(content: &str) -> Self {
        Self {
            r#type: "text".to_string(),
            text: Some(content.to_string()),
            data: None,
            mime_type: None,
        }
    }
}

// ============================================================================
// Legacy types (kept for compatibility)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
