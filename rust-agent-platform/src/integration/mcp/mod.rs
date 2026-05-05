mod protocol;
mod client;

pub use protocol::{MCPMessage, Capabilities, Resource, Tool, Prompt, ContentBlock};
pub use client::MCPClient;