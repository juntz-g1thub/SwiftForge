use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use swiftforge_log::{debug, info, warn};
use swiftforge_provider_core::{DynLLMProvider, DynToolCallingProvider, ToolCallingProvider};
use swiftforge_task::{AgentMessage, MessageBus, Task, TaskScheduler};
use swiftforge_tools::ToolCallParser;
use swiftforge_types::{
    Message, ModelResponse, Session, StreamingChunk, ToolCall, ToolDefinition, ToolRegistry,
    ToolResult, Usage,
};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub model: Option<String>,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    Orchestrator,
    Executor,
    Planner,
    Advisor,
    Explorer,
    Librarian,
}

#[derive(Clone)]
pub struct Agent {
    config: AgentConfig,
    scheduler: Option<Arc<TaskScheduler>>,
    message_bus: Option<Arc<MessageBus>>,
    llm_provider: DynLLMProvider,
    tool_provider: Option<DynToolCallingProvider>,
    tool_registry: Option<Arc<ToolRegistry>>,
    tool_parser: ToolCallParser,
    reasoning_history: Arc<std::sync::Mutex<Vec<String>>>,
}

impl Agent {
    pub fn new(config: AgentConfig, llm_provider: DynLLMProvider) -> Self {
        Self {
            config,
            scheduler: None,
            message_bus: None,
            llm_provider,
            tool_provider: None,
            tool_registry: None,
            tool_parser: ToolCallParser::new(),
            reasoning_history: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn with_scheduler(mut self, scheduler: Arc<TaskScheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    pub fn with_message_bus(mut self, message_bus: Arc<MessageBus>) -> Self {
        self.message_bus = Some(message_bus);
        self
    }

    pub fn with_tool_provider(mut self, provider: Option<DynToolCallingProvider>) -> Self {
        self.tool_provider = provider;
        self
    }

    pub fn with_tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn role(&self) -> &AgentRole {
        &self.config.role
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    pub fn provider_name(&self) -> &str {
        self.llm_provider.provider_name()
    }

    pub fn has_tool_registry(&self) -> bool {
        self.tool_registry.is_some()
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tool_registry
            .as_ref()
            .map(|r| r.list_tools())
            .unwrap_or_default()
    }

    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tool_registry
            .as_ref()
            .map(|r| r.get_definitions())
            .unwrap_or_default()
    }

    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<ToolResult> {
        let registry = self
            .tool_registry
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No tool registry configured"))?;
        let call = ToolCall {
            name: name.to_string(),
            arguments: if let serde_json::Value::Object(map) = arguments {
                map.into_iter().map(|(k, v)| (k, v)).collect()
            } else {
                HashMap::new()
            },
        };
        Ok(registry.execute(call).await)
    }

    pub fn parse_tool_calls(&self, content: &str) -> Vec<ToolCall> {
        self.tool_parser.parse(content)
    }

    pub fn parse_tool_calls_from_json(&self, tool_calls: &[JsonValue]) -> Vec<ToolCall> {
        self.tool_parser.parse_from_json(tool_calls)
    }

    pub async fn execute_tool_calls(&self, calls: Vec<ToolCall>) -> Result<Vec<ToolResult>> {
        let registry = self
            .tool_registry
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No tool registry configured"))?;

        let futures = calls.into_iter().map(|call| registry.execute(call));
        let results = futures::future::join_all(futures).await;

        Ok(results)
    }

    pub async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        self.llm_provider
            .chat(messages)
            .await
            .map_err(|e: swiftforge_provider_core::ProviderError| anyhow::anyhow!("{:?}", e))
    }

    pub async fn chat_with_tools(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        debug!("[provider]", "chat_with_tools called");
        let provider = self
            .tool_provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No tool-calling provider configured"))?;
        debug!("[provider]", "Got provider");
        let tools = self.get_tool_definitions();
        debug!("[provider]", "Got {} tools", tools.len());
        provider
            .chat_with_tools(messages, tools)
            .await
            .map_err(|e: swiftforge_provider_core::ProviderError| anyhow::anyhow!("{:?}", e))
    }

    pub async fn chat_with_tools_streaming<F>(
        &self,
        messages: Vec<Message>,
        on_chunk: F,
    ) -> Result<ModelResponse>
    where
        F: FnMut(StreamingChunk) + Send + Sync + 'static,
    {
        debug!("[provider]", "chat_with_tools_streaming called");
        let provider = self
            .tool_provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No tool-calling provider configured"))?;
        debug!("[provider]", "Got provider");
        let tools = self.get_tool_definitions();
        debug!("[provider]", "Got {} tools", tools.len());

        let accumulated_content = Arc::new(std::sync::Mutex::new(String::new()));
        let accumulated_content_clone = accumulated_content.clone();
        let accumulated_reasoning = Arc::new(std::sync::Mutex::new(String::new()));
        let accumulated_reasoning_clone = accumulated_reasoning.clone();
        let on_chunk = Arc::new(std::sync::Mutex::new(on_chunk));
        let on_chunk_clone = on_chunk.clone();
        let tool_calls_json = Arc::new(std::sync::Mutex::new(Vec::new()));
        let tool_calls_json_clone = tool_calls_json.clone();

        let on_chunk_wrapper = Box::new(move |chunk: StreamingChunk| match chunk {
            StreamingChunk::Reasoning(text) => {
                accumulated_reasoning_clone.lock().unwrap().push_str(&text);
                if let Ok(mut cb) = on_chunk_clone.lock() {
                    cb(StreamingChunk::Reasoning(text));
                }
            }
            StreamingChunk::Content(text) => {
                accumulated_content_clone.lock().unwrap().push_str(&text);
                if let Ok(mut cb) = on_chunk_clone.lock() {
                    cb(StreamingChunk::Content(text));
                }
            }
            StreamingChunk::ToolCall { name, arguments } => {
                let tc_json = serde_json::json!({
                    "name": name,
                    "arguments": arguments
                });
                tool_calls_json_clone.lock().unwrap().push(tc_json);
            }
        });

        provider
            .stream_chat_with_tools(messages, tools, on_chunk_wrapper)
            .await
            .map_err(|e: swiftforge_provider_core::ProviderError| anyhow::anyhow!("{:?}", e))?;

        let content = accumulated_content.lock().unwrap().clone();
        let reasoning = accumulated_reasoning.lock().unwrap().clone();
        let tool_calls = tool_calls_json.lock().unwrap().clone();

        if !reasoning.is_empty() {
            self.reasoning_history
                .lock()
                .unwrap()
                .push(reasoning.clone());
        }

        let mut response = ModelResponse::new(
            content,
            Usage {
                input_tokens: 0,
                output_tokens: 0,
            },
        );
        if !tool_calls.is_empty() {
            response = response.with_tool_calls(tool_calls);
        }
        if !reasoning.is_empty() {
            response = response.with_reasoning_content(reasoning);
        }
        Ok(response)
    }

    pub async fn run_agent_loop(
        &self,
        session: Arc<RwLock<Session>>,
        initial_message: &str,
        max_iterations: usize,
        stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>,
    ) -> Result<String> {
        info!(
            "[agent]",
            "run_agent_loop started with: {}", initial_message
        );
        session.write().await.add_message("user", initial_message);

        let mut full_response = String::new();
        let mut tool_summary = Vec::new();

        for i in 0..max_iterations {
            info!("[agent]", "Agent loop iteration {}", i + 1);

            let messages = session.read().await.messages();

            let stream_ui_clone = stream_ui.clone();
            let on_chunk = move |chunk: StreamingChunk| {
                if let Some(ref t) = stream_ui_clone {
                    let text = match chunk {
                        StreamingChunk::Reasoning(s) => s,
                        StreamingChunk::Content(s) => s,
                        StreamingChunk::ToolCall { name, arguments } => {
                            format!("[Tool: {}] {}", name, arguments)
                        }
                    };
                    let _ = t.send(Ok(text));
                }
            };

            let response = self
                .chat_with_tools_streaming(messages.clone(), on_chunk)
                .await?;

            info!(
                "[agent]",
                "Got response, content len: {}, tool_calls: {:?}",
                response.content.len(),
                response.tool_calls.as_ref().map(|tc| tc.len())
            );

            if !response.content.is_empty() {
                full_response.push_str(&response.content);
            }

            let tool_calls = if let Some(ref tc) = response.tool_calls {
                let calls = self.parse_tool_calls_from_json(tc);
                info!(
                    "[agent]",
                    "Parsed {} tool_calls from response.tool_calls JSON",
                    calls.len()
                );
                calls
            } else {
                info!(
                    "[agent]",
                    "No tool_calls in response JSON, parsing from content..."
                );
                let calls = self.parse_tool_calls(&response.content);
                info!("[agent]", "Parsed {} tool_calls from content", calls.len());
                if calls.is_empty() && !response.content.is_empty() {
                    info!(
                        "[agent]",
                        "Content preview: {}",
                        response.content.chars().take(200).collect::<String>()
                    );
                }
                calls
            };

            if tool_calls.is_empty() {
                if full_response.is_empty() {
                    full_response.push_str("Tool execution completed.");
                }
                session
                    .write()
                    .await
                    .add_message("assistant", &response.content);
                break;
            }

            session
                .write()
                .await
                .add_message("assistant", &response.content);

            let results = self.execute_tool_calls(tool_calls).await?;

            for result in results {
                let tool_result_text = match result.output {
                    Some(output) => {
                        tool_summary.push(format!(
                            "✓ Tool: {}",
                            output.chars().take(50).collect::<String>()
                        ));
                        output
                    }
                    None => {
                        let err = result.error.clone().unwrap_or_default();
                        tool_summary.push(format!("✗ Error: {}", err));
                        err
                    }
                };

                session.write().await.add_message("user", &tool_result_text);

                if let Some(ref tx) = stream_ui {
                    let _ = tx.send(Ok(format!("[TOOL_RESULT] {}", tool_result_text)));
                }

                info!(
                    "[agent]",
                    "Tool executed: {}",
                    tool_result_text.chars().take(100).collect::<String>()
                );
            }

            let mut session_guard = session.write().await;
            if session_guard.needs_compaction() {
                if let Err(e) = session_guard.compact(|msgs| async move {
                    self.llm_provider.chat(msgs).await.map_err(|e| anyhow::anyhow!("{:?}", e))
                }).await {
                    warn!("[session]", "Compact failed: {}", e);
                }
            }
            drop(session_guard);
        }

        if full_response.is_empty() && !tool_summary.is_empty() {
            full_response = format!(
                "Executed {} tools:\n{}",
                tool_summary.len(),
                tool_summary.join("\n")
            );
        } else if !tool_summary.is_empty() && !full_response.is_empty() {
            full_response = format!(
                "{}\n\nExecuted {} tools:\n{}",
                full_response,
                tool_summary.len(),
                tool_summary.join("\n")
            );
        }

        Ok(full_response)
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        self.llm_provider
            .list_models()
            .await
            .map_err(|e: swiftforge_provider_core::ProviderError| anyhow::anyhow!("{:?}", e))
    }

    pub async fn process_task(&self) -> Result<Option<Task>> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No scheduler configured"))?;
        let Some(task) = scheduler.get_next_task().await else {
            return Ok(None);
        };
        info!(
            "[agent]",
            "Agent {} processing task: {}",
            self.name(),
            task.description
        );
        Ok(Some(task))
    }

    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No scheduler configured"))?;
        scheduler.complete_task(task_id).await;
        Ok(())
    }

    pub async fn fail_task(&self, task_id: &str) -> Result<()> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No scheduler configured"))?;
        scheduler.fail_task(task_id).await;
        Ok(())
    }

    pub async fn send_message(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        let message_bus = self
            .message_bus
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No message bus configured"))?;
        let message = AgentMessage {
            from: self.name().to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        };
        message_bus.send(message).await
    }

    pub async fn broadcast(&self, subject: &str, body: &str) -> Result<()> {
        let message_bus = self
            .message_bus
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No message bus configured"))?;
        message_bus.broadcast(self.name(), subject, body).await
    }

    pub fn is_connected(&self) -> bool {
        self.scheduler.is_some() && self.message_bus.is_some()
    }

    pub fn has_provider(&self) -> bool {
        true
    }

    pub fn has_tool_provider(&self) -> bool {
        self.tool_provider.is_some()
    }
}
