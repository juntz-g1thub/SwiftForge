use rust_agent_platform::integration::mcp::{ContentBlock, MCPClient, Tool};

#[tokio::test]
async fn test_mcp_client_creation() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    assert!(!client.is_connected().await);
    assert!(!client.is_initialized().await);
}

#[tokio::test]
async fn test_mcp_client_not_connected_error() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    let result = client.list_tools().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mcp_client_without_real_server() {
    let client = MCPClient::new("http://localhost:8080".to_string());
    let connect_result = client.connect().await;
    assert!(connect_result.is_err());
}
