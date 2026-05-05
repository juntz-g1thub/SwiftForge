use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPMessage {
    Initialize {
        protocol_version: String,
        capabilities: Capabilities,
    },
    Initialized {
        capabilities: Capabilities,
    },
    Ping,
    Pong,
    ResourcesList,
    ResourcesListResponse {
        resources: Vec<Resource>,
    },
    ToolsList,
    ToolsListResponse {
        tools: Vec<Tool>,
    },
    ToolsCall {
        name: String,
        arguments: serde_json::Value,
    },
    ToolsCallResponse {
        content: Vec<ContentBlock>,
    },
    PromptsList,
    PromptsListResponse {
        prompts: Vec<Prompt>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub resources: bool,
    pub tools: bool,
    pub prompts: bool,
    pub sampling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    pub r#type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub mime_type: Option<String>,
}
