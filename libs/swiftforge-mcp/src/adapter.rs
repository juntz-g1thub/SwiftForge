use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use swiftforge_types::{Tool, ToolCall, ToolResult};
use crate::client::MCPClient;

pub struct McpToolAdapter {
    full_name: String,
    mcp_name: String,
    description: String,
    input_schema: JsonValue,
    mcp_client: Arc<MCPClient>,
}

impl McpToolAdapter {
    pub fn new(
        server_name: &str,
        mcp_client: Arc<MCPClient>,
        name: String,
        description: String,
        input_schema: JsonValue,
    ) -> Self {
        let full_name = format!("{}_{}", server_name, name);
        Self {
            full_name,
            mcp_name: name,
            description,
            input_schema,
            mcp_client,
        }
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    async fn execute(&self, call: ToolCall) -> ToolResult {
        let arguments = serde_json::to_value(&call.arguments)
            .unwrap_or(JsonValue::Null);

        match self.mcp_client.call_tool(&self.mcp_name, arguments).await {
            Ok(content_blocks) => {
                let output = content_blocks
                    .iter()
                    .filter_map(|cb| cb.text.clone())
                    .collect::<Vec<_>>()
                    .join("\n");

                ToolResult {
                    success: true,
                    output: Some(output),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: None,
                error: Some(e.to_string()),
            },
        }
    }
}