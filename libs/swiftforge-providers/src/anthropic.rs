use async_trait::async_trait;
use swiftforge_provider_core::{LLMProvider, ToolCallingProvider, ProviderError};
use swiftforge_types::{ModelResponse, Usage, Message, ToolDefinition};
use swiftforge_provider_core::error::Result;
use anyhow::Context;
use tokio_stream::StreamExt;

pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl Clone for AnthropicProvider {
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        }
    }
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            model: "claude-3-sonnet-20240229".to_string(),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-sonnet-20240620".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ])
    }

    pub async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse> {
        let client = reqwest::Client::new();
        let tools_json: Vec<serde_json::Value> = tools.into_iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema
            })
        }).collect();

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "tools": tools_json,
            "max_tokens": 1024
        });

        let response = client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = data["content"]
            .as_array()
            .and_then(|arr| arr.iter().find(|c| c["type"] == "tool_use"))
            .map(|c| c["name"].clone());

        let usage = Usage {
            input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let mut response = ModelResponse::new(content, usage);
        if let Some(tc) = tool_calls {
            response = response.with_tool_calls(vec![tc]);
        }

        Ok(response)
    }

    pub async fn stream_chat(&self, messages: Vec<Message>, mut on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("accept", "text/event-stream")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages.iter().map(|m| {
                    serde_json::json!({ "role": m.role, "content": m.content })
                }).collect::<Vec<_>>(),
                "max_tokens": 1024,
                "stream": true
            }))
            .send()
            .await?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("data:") {
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        return Ok(());
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["content"][0]["text"].as_str() {
                            on_chunk(content.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, mut on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        let client = reqwest::Client::new();
        let tools_json: Vec<serde_json::Value> = tools.into_iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema
            })
        }).collect();

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "tools": tools_json,
            "max_tokens": 1024,
            "stream": true
        });

        let response = client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("accept", "text/event-stream")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("data:") {
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        return Ok(());
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["content"][0]["text"].as_str() {
                            on_chunk(content.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages.iter().map(|m| {
                    serde_json::json!({ "role": m.role, "content": m.content })
                }).collect::<Vec<_>>(),
                "max_tokens": 1024
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["content"][0]["text"]
            .as_str()
            .context("No content in response")?
            .to_string();

        Ok(ModelResponse::new(content, Usage {
            input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        }))
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Self::list_models(self).await
    }

    async fn stream_chat(&self, messages: Vec<Message>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        Self::stream_chat(self, messages, on_chunk).await
    }
}

#[async_trait]
impl ToolCallingProvider for AnthropicProvider {
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse> {
        Self::chat_with_tools(self, messages, tools).await
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        Self::stream_chat_with_tools(self, messages, tools, on_chunk).await
    }
}