mod protocol;
mod client;

pub use protocol::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError,
    InitializeParams, InitializeResult, ClientCapabilities, ClientInfo, ServerCapabilities,
    ToolsListParams, ToolsListResult, Tool,
    ToolCallParams, ToolCallResult, ContentBlock,
    Resource, Prompt, PromptArgument,
};
pub use client::MCPClient;