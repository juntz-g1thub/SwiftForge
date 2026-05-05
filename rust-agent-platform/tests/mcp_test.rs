use rust_agent_platform::integration::mcp::{MCPClient, Resource, Tool, ContentBlock};

#[tokio::test]
async fn test_mcp_client_creation() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    assert!(!client.is_connected().await);
}

#[tokio::test]
async fn test_mcp_client_connect() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    client.connect().await.unwrap();
    assert!(client.is_connected().await);
}

#[tokio::test]
async fn test_mcp_client_disconnect() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    client.connect().await.unwrap();
    client.disconnect().await.unwrap();
    assert!(!client.is_connected().await);
}

#[tokio::test]
async fn test_mcp_list_resources() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    client.connect().await.unwrap();
    let resources = client.list_resources().await.unwrap();
    assert!(resources.is_empty());
}

#[tokio::test]
async fn test_mcp_list_tools() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    client.connect().await.unwrap();
    let tools = client.list_tools().await.unwrap();
    assert!(tools.is_empty());
}

#[tokio::test]
async fn test_mcp_call_tool() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    client.connect().await.unwrap();
    let result = client.call_tool("test_tool", serde_json::json!({})).await.unwrap();
    assert!(!result.is_empty());
}

#[tokio::test]
async fn test_mcp_not_connected_error() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    let result = client.list_resources().await;
    assert!(result.is_err());
}