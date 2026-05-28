use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use swiftforge_task::{TaskScheduler, Task, MessageBus, AgentMessage};
use crate::providers::{LLMProvider, ProviderRegistry, ToolCallingProvider};
use swiftforge_types::{Message, ModelResponse, Usage, ToolRegistry, ToolCall, ToolResult, ToolDefinition};

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
    providers: ProviderRegistry,
    tool_registry: Option<Arc<ToolRegistry>>,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            scheduler: None,
            message_bus: None,
            providers: ProviderRegistry::new(),
            tool_registry: None,
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

    pub fn with_provider<P: LLMProvider + 'static>(mut self, name: &str, provider: P) -> Self {
        self.providers.register(name, provider);
        self
    }

    pub fn with_tool_provider<P: ToolCallingProvider + 'static>(mut self, name: &str, provider: P) -> Self {
        self.providers.register_with_tools(name, provider);
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

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.list_providers()
    }

    pub fn list_tool_providers(&self) -> Vec<String> {
        self.providers.list_tool_providers()
    }

    pub fn provider(&self, name: &str) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(name)
    }

    pub fn default_provider_name(&self) -> Option<&str> {
        self.providers.default().map(|p| p.provider_name())
    }

    pub fn has_tool_registry(&self) -> bool {
        self.tool_registry.is_some()
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tool_registry.as_ref()
            .map(|r| r.list_tools())
            .unwrap_or_default()
    }

    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tool_registry.as_ref()
            .map(|r| r.get_definitions())
            .unwrap_or_default()
    }

    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<ToolResult> {
        let registry = self.tool_registry.as_ref()
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
        let mut calls = Vec::new();
        if let Ok(json) = serde_json::from_str::<JsonValue>(content) {
            if let Some(tool_calls) = json.get("tool_calls").and_then(|t| t.as_array()) {
                for call in tool_calls {
                    if let (Some(name), Some(args)) = (
                        call.get("name").and_then(|n| n.as_str()),
                        call.get("arguments")
                    ) {
                        let arguments: HashMap<String, JsonValue> = if let serde_json::Value::Object(map) = args {
                            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        } else if let serde_json::Value::String(s) = args {
                            serde_json::from_str(&s).unwrap_or_default()
                        } else {
                            HashMap::new()
                        };
                        calls.push(ToolCall { name: name.to_string(), arguments });
                    }
                }
            }
        }
        let re = regex::Regex::new(r#"<tool_call>\s*\{[^}]*?"name"\s*:\s*"([^"]+)"[^}]*?"arguments"\s*:\s*(\{[^}]+\})[^}]*\}</tool_call>"#).ok();
        if let Some(re) = re {
            for cap in re.captures_iter(content) {
                let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("{}");
                let arguments: HashMap<String, JsonValue> = serde_json::from_str(args_str).unwrap_or_default();
                if !name.is_empty() {
                    calls.push(ToolCall { name, arguments });
                }
            }
        }
        calls
    }

    pub fn parse_tool_calls_from_json(&self, tool_calls: &[serde_json::Value]) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        for call in tool_calls {
            // DeepSeek/OpenAI format: {"function": {"name": "...", "arguments": "..."}}
            // Also supports direct format: {"name": "...", "arguments": {...}}
            let name = call.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .or_else(|| call.get("name").and_then(|n| n.as_str()));

            let args = call.get("function")
                .and_then(|f| f.get("arguments"))
                .or_else(|| call.get("arguments"));

            if let (Some(name), Some(args)) = (name, args) {
                let arguments: HashMap<String, JsonValue> = if let serde_json::Value::Object(map) = args {
                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                } else if let serde_json::Value::String(s) = args {
                    serde_json::from_str(&s).unwrap_or_default()
                } else {
                    HashMap::new()
                };
                calls.push(ToolCall { name: name.to_string(), arguments });
            }
        }
        calls
    }

    pub async fn execute_tool_calls(&self, calls: Vec<ToolCall>) -> Result<Vec<ToolResult>> {
        let registry = self.tool_registry.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No tool registry configured"))?;
        let mut results = Vec::new();
        for call in calls {
            let result = registry.execute(call).await;
            results.push(result);
        }
        Ok(results)
    }

    pub async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        let provider = self.providers.default()
            .ok_or_else(|| anyhow::anyhow!("No provider configured"))?;
        provider.chat(messages).await
    }

    pub async fn chat_with(&self, provider_name: &str, messages: Vec<Message>) -> Result<ModelResponse> {
        let provider = self.providers.get(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider {} not found", provider_name))?;
        provider.chat(messages).await
    }

    pub async fn chat_with_tools(&self, messages: Vec<Message>, debug_log: Option<String>, debug_ui: Option<std::sync::mpsc::Sender<String>>) -> Result<ModelResponse> {
        let log = |msg: &str| {
            if let Some(ref path) = debug_log {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] PROVIDER: {}", timestamp, msg);
                let _ = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", formatted)
                    });
            }
            if let Some(ref tx) = debug_ui {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] PROVIDER: {}", timestamp, msg);
                let _ = tx.send(formatted);
            }
        };
        log(&format!("chat_with_tools called"));
        let provider = self.providers.default_tool_provider()
            .ok_or_else(|| anyhow::anyhow!("No tool-calling provider configured"))?;
        log("Got provider");
        let tools = self.get_tool_definitions();
        log(&format!("Got {} tools", tools.len()));
        provider.chat_with_tools(messages, tools).await
    }

    pub async fn chat_with_tools_provider(&self, provider_name: &str, messages: Vec<Message>) -> Result<ModelResponse> {
        let provider = self.providers.get_tool_provider(provider_name)
            .ok_or_else(|| anyhow::anyhow!("Tool provider {} not found", provider_name))?;
        let tools = self.get_tool_definitions();
        provider.chat_with_tools(messages, tools).await
    }

    pub async fn chat_with_tools_streaming<F>(&self, messages: Vec<Message>, debug_log: Option<String>, debug_ui: Option<std::sync::mpsc::Sender<String>>, mut on_chunk: F) -> Result<ModelResponse>
        where F: FnMut(String) + Send + Sync + 'static
    {
        let log = |msg: &str| {
            if let Some(ref path) = debug_log {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] PROVIDER: {}", timestamp, msg);
                let _ = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", formatted)
                    });
            }
            if let Some(ref tx) = debug_ui {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] PROVIDER: {}", timestamp, msg);
                let _ = tx.send(formatted);
            }
        };
        log(&format!("chat_with_tools_streaming called"));
        let provider = self.providers.default_tool_provider()
            .ok_or_else(|| anyhow::anyhow!("No tool-calling provider configured"))?;
        log("Got provider");
        let tools = self.get_tool_definitions();
        log(&format!("Got {} tools", tools.len()));

        let debug_ui_clone = debug_ui.clone();
        let debug_log_clone = debug_log.clone();
        let accumulated = Arc::new(std::sync::Mutex::new(String::new()));
        let accumulated_clone = accumulated.clone();
        let on_chunk = Arc::new(std::sync::Mutex::new(on_chunk));
        let on_chunk_clone = on_chunk.clone();
        let tool_calls_json = Arc::new(std::sync::Mutex::new(Vec::new()));
        let tool_calls_json_clone = tool_calls_json.clone();

        let on_chunk_wrapper = Box::new(move |chunk: String| {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&chunk) {
                if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                    if let Some(arguments) = json.get("arguments").and_then(|a| a.as_str()) {
                        let tc_json = serde_json::json!({
                            "name": name,
                            "arguments": arguments
                        });
                        tool_calls_json_clone.lock().unwrap().push(tc_json);
                        return;
                    }
                }
            }

            accumulated_clone.lock().unwrap().push_str(&chunk);

            if let Ok(mut cb) = on_chunk_clone.lock() {
                cb(chunk.clone());
            }
            if let Some(ref tx) = debug_ui_clone {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] STREAM: {}", timestamp, chunk);
                let _ = tx.send(formatted);
            }
            if let Some(ref path) = debug_log_clone {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] STREAM: {}", timestamp, chunk);
                let _ = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", formatted)
                    });
            }
        });

        provider.stream_chat_with_tools(messages, tools, on_chunk_wrapper).await?;

        let result = accumulated.lock().unwrap().clone();
        let tool_calls = tool_calls_json.lock().unwrap().clone();
        let mut response = ModelResponse::new(result, Usage { input_tokens: 0, output_tokens: 0 });
        if !tool_calls.is_empty() {
            response = response.with_tool_calls(tool_calls);
        }
        Ok(response)
    }

    pub async fn run_agent_loop(&self, initial_message: &str, max_iterations: usize, debug_log: Option<String>, debug_ui: Option<std::sync::mpsc::Sender<String>>, stream_ui: Option<std::sync::mpsc::Sender<Result<String>>>) -> Result<String> {
        let log = |msg: &str| {
            if let Some(ref path) = debug_log {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] AGENT: {}", timestamp, msg);
                let _ = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        writeln!(f, "{}", formatted)
                    });
            }
            if let Some(ref tx) = debug_ui {
                let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
                let formatted = format!("[{}] AGENT: {}", timestamp, msg);
                let _ = tx.send(formatted);
            }
        };

        let stream = |msg: &str, tx: &Option<std::sync::mpsc::Sender<String>>| {
            if let Some(ref t) = tx {
                let _ = t.send(msg.to_string());
            }
        };

        log(&format!("run_agent_loop started with: {}", initial_message));
        let mut messages = vec![
            Message {
                role: "user".to_string(),
                content: initial_message.to_string(),
            }
        ];

        let mut full_response = String::new();
        let mut tool_summary = Vec::new();

        for i in 0..max_iterations {
            log(&format!("Agent loop iteration {}", i + 1));

let stream_ui_clone = stream_ui.clone();
            let on_chunk = move |chunk: String| {
                if let Some(ref t) = stream_ui_clone {
                    let _ = t.send(Ok(chunk));
                }
            };

            let response = self.chat_with_tools_streaming(messages.clone(), debug_log.clone(), debug_ui.clone(), on_chunk).await?;

            log(&format!("Got response, content len: {}, tool_calls: {:?}",
                response.content.len(),
                response.tool_calls.as_ref().map(|tc| tc.len())
            ));

            if !response.content.is_empty() {
                full_response.push_str(&response.content);
            }

            let tool_calls = if let Some(ref tc) = response.tool_calls {
                let calls = self.parse_tool_calls_from_json(tc);
                log(&format!("Parsed {} tool_calls from response.tool_calls JSON", calls.len()));
                calls
            } else {
                log("No tool_calls in response JSON, parsing from content...");
                let calls = self.parse_tool_calls(&response.content);
                log(&format!("Parsed {} tool_calls from content", calls.len()));
                if calls.is_empty() && !response.content.is_empty() {
                    log(&format!("Content preview: {}", response.content.chars().take(200).collect::<String>()));
                }
                calls
            };

            if tool_calls.is_empty() {
                if full_response.is_empty() {
                    full_response.push_str("Tool execution completed.");
                }
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: response.content,
                });
                break;
            }

            messages.push(Message {
                role: "assistant".to_string(),
                content: response.content.clone(),
            });

            let results = self.execute_tool_calls(tool_calls).await?;

            for result in results {
                let tool_result_text = match result.output {
                    Some(output) => {
                        tool_summary.push(format!("✓ Tool: {}", output.chars().take(50).collect::<String>()));
                        output
                    }
                    None => {
                        let err = result.error.clone().unwrap_or_default();
                        tool_summary.push(format!("✗ Error: {}", err));
                        err
                    }
                };

                messages.push(Message {
                    role: "user".to_string(),
                    content: tool_result_text.clone(),
                });

                if let Some(ref tx) = stream_ui {
                    let _ = tx.send(Ok(format!("[TOOL_RESULT] {}", tool_result_text)));
                }

                log(&format!("Tool executed: {}", tool_result_text.chars().take(100).collect::<String>()));
            }
        }

        if full_response.is_empty() && !tool_summary.is_empty() {
            full_response = format!("Executed {} tools:\n{}", tool_summary.len(), tool_summary.join("\n"));
        } else if !tool_summary.is_empty() && !full_response.is_empty() {
            full_response = format!("{}\n\nExecuted {} tools:\n{}", full_response, tool_summary.len(), tool_summary.join("\n"));
        }

        Ok(full_response)
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let provider = self.providers.default()
            .ok_or_else(|| anyhow::anyhow!("No provider configured"))?;
        provider.list_models().await
    }

    pub async fn process_task(&self) -> Result<Option<Task>> {
        let scheduler = self.scheduler.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No scheduler configured"))?;
        let Some(task) = scheduler.get_next_task().await else {
            return Ok(None);
        };
        tracing::info!("Agent {} processing task: {}", self.name(), task.description);
        Ok(Some(task))
    }

    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        let scheduler = self.scheduler.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No scheduler configured"))?;
        scheduler.complete_task(task_id).await;
        Ok(())
    }

    pub async fn fail_task(&self, task_id: &str) -> Result<()> {
        let scheduler = self.scheduler.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No scheduler configured"))?;
        scheduler.fail_task(task_id).await;
        Ok(())
    }

    pub async fn send_message(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        let message_bus = self.message_bus.as_ref()
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
        let message_bus = self.message_bus.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No message bus configured"))?;
        message_bus.broadcast(self.name(), subject, body).await
    }

    pub fn is_connected(&self) -> bool {
        self.scheduler.is_some() && self.message_bus.is_some()
    }

    pub fn has_provider(&self) -> bool {
        !self.providers.list_providers().is_empty()
    }

    pub fn has_tool_provider(&self) -> bool {
        !self.providers.list_tool_providers().is_empty()
    }
}
