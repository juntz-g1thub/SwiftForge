use rust_agent_platform::core::{
    Agent, AgentConfig, AgentRole, 
    ToolRegistry, Tool, ToolCall, ToolResult,
    Message,
};
use rust_agent_platform::orchestration::{TaskScheduler, MessageBus};
use rust_agent_platform::providers::{LLMProvider, OpenAIProvider, ProviderRegistry, ModelResponse, Usage};
use async_trait::async_trait;
use std::sync::Arc;

struct MockProvider;

#[async_trait]
impl LLMProvider for MockProvider {
    async fn chat(&self, _messages: Vec<Message>) -> anyhow::Result<ModelResponse> {
        Ok(ModelResponse::new(
            r#"{"tool_calls":[{"name":"read_file","arguments":{"path":"/test.txt"}}]}"#.to_string(),
            Usage { input_tokens: 10, output_tokens: 20 },
        ))
    }

    fn provider_name(&self) -> &str {
        "mock"
    }

    async fn list_models(&self) -> anyhow::Result<Vec<String>> {
        Ok(vec!["mock-model".to_string()])
    }

    async fn stream_chat(&self, _messages: Vec<Message>, _on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> anyhow::Result<()> {
        Ok(())
    }
}

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echo back the input" }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Text to echo back"
                }
            },
            "required": ["input"]
        })
    }
    async fn execute(&self, call: ToolCall) -> ToolResult {
        let input = call.arguments.get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("nothing");
        ToolResult {
            success: true,
            output: Some(format!("echo: {}", input)),
            error: None,
        }
    }
}

#[tokio::test]
async fn test_agent_with_provider() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: Some("mock-model".to_string()),
        temperature: 0.1,
    };

    let agent = Agent::new(config)
        .with_provider("mock", MockProvider);

    assert!(agent.has_provider());
    assert_eq!(agent.default_provider_name(), Some("mock"));
    assert_eq!(agent.list_providers(), vec!["mock"]);

    let models = agent.list_models().await.unwrap();
    assert_eq!(models, vec!["mock-model"]);
}

#[tokio::test]
async fn test_agent_with_scheduler() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let scheduler = Arc::new(TaskScheduler::new());

    let agent = Agent::new(config)
        .with_scheduler(scheduler.clone());

    scheduler.add_task(rust_agent_platform::orchestration::Task {
        id: "task-1".to_string(),
        description: "Test task".to_string(),
        priority: rust_agent_platform::orchestration::TaskPriority::Normal,
        assigned_to: None,
        status: rust_agent_platform::orchestration::TaskStatus::Pending,
    }).await;

    let task = agent.process_task().await.unwrap();
    assert!(task.is_some());
    assert_eq!(task.unwrap().description, "Test task");
}

#[tokio::test]
async fn test_agent_with_scheduler_and_message_bus() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let scheduler = Arc::new(TaskScheduler::new());
    let message_bus = Arc::new(MessageBus::new());

    let agent = Agent::new(config)
        .with_scheduler(scheduler)
        .with_message_bus(message_bus);

    assert!(agent.is_connected());
}

#[tokio::test]
async fn test_agent_with_message_bus_only() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let message_bus = Arc::new(MessageBus::new());

    let agent = Agent::new(config)
        .with_message_bus(message_bus);

    assert!(!agent.is_connected());
}

#[tokio::test]
async fn test_agent_with_tool_registry() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let mut registry = ToolRegistry::new();
    registry.register(EchoTool);

    let agent = Agent::new(config)
        .with_tool_registry(Arc::new(registry));

    assert!(agent.has_tool_registry());
    assert_eq!(agent.list_tools(), vec!["echo"]);

    let result = agent.call_tool("echo", serde_json::json!({"input": "hello"}))
        .await
        .unwrap();
    assert!(result.success);
    assert_eq!(result.output, Some("echo: hello".to_string()));
}

#[tokio::test]
async fn test_parse_tool_calls_json_format() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let agent = Agent::new(config);

    let content = r#"{"tool_calls":[{"name":"read_file","arguments":{"path":"/test.txt"}},{"name":"bash","arguments":{"command":"ls -la"}}]}"#;
    let calls = agent.parse_tool_calls(content);

    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "read_file");
    assert_eq!(calls[0].arguments.get("path").and_then(|v| v.as_str()), Some("/test.txt"));
    assert_eq!(calls[1].name, "bash");
}

#[tokio::test]
async fn test_parse_tool_calls_xml_format() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let agent = Agent::new(config);

    let content = r#"Here is my response<tool_call>{"name":"read_file","arguments":{"path":"/test.txt"}}</tool_call>More text"#;
    let calls = agent.parse_tool_calls(content);

    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "read_file");
}

#[tokio::test]
async fn test_execute_tool_calls() {
    let config = AgentConfig {
        name: "test-agent".to_string(),
        role: AgentRole::Executor,
        model: None,
        temperature: 0.1,
    };

    let mut registry = ToolRegistry::new();
    registry.register(EchoTool);

    let agent = Agent::new(config)
        .with_tool_registry(Arc::new(registry));

    let calls = vec![
        ToolCall {
            name: "echo".to_string(),
            arguments: vec![("input".to_string(), serde_json::json!("hello"))].into_iter().collect(),
        },
        ToolCall {
            name: "echo".to_string(),
            arguments: vec![("input".to_string(), serde_json::json!("world"))].into_iter().collect(),
        },
    ];

    let results = agent.execute_tool_calls(calls).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].output, Some("echo: hello".to_string()));
    assert_eq!(results[1].output, Some("echo: world".to_string()));
}