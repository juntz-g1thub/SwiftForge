use async_trait::async_trait;
use swiftforge_provider_core::{LLMProvider, ToolCallingProvider, ProviderError};
use swiftforge_types::{ModelResponse, Usage, Message, ToolDefinition, StreamingChunk};
use swiftforge_provider_core::error::Result;
use tokio_stream::StreamExt;

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl Clone for OpenAIProvider {
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        }
    }
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            model: "gpt-4o".to_string(),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.base_url);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError { status: status.as_u16(), message: body }.into());
        }

        let data: serde_json::Value = response.json().await.map_err(|e| ProviderError::ParseError(e.to_string()))?;
        let models = data["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    pub async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse> {
        let client = reqwest::Client::new();
        let tools_json: Vec<serde_json::Value> = tools.into_iter().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema
                }
            })
        }).collect();

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "tools": tools_json,
        });

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ProviderError::NetworkError(e.to_string()))?;

        let data: serde_json::Value = response.json().await.map_err(|e| ProviderError::ParseError(e.to_string()))?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = data["choices"][0]["message"]["tool_calls"]
            .as_array()
            .map(|arr| arr.clone());

        let usage = Usage {
            input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let mut response = ModelResponse::new(content, usage);
        if let Some(tc) = tool_calls {
            response = response.with_tool_calls(tc);
        }

        Ok(response)
    }

    pub async fn stream_chat(&self, messages: Vec<Message>, mut on_chunk: Box<dyn FnMut(StreamingChunk) + Send + Sync + 'static>) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages.iter().map(|m| {
                    serde_json::json!({ "role": m.role, "content": m.content })
                }).collect::<Vec<_>>(),
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
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            on_chunk(StreamingChunk::Content(content.to_string()));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, mut on_chunk: Box<dyn FnMut(StreamingChunk) + Send + Sync + 'static>) -> Result<()> {
        let client = reqwest::Client::new();
        let tools_json: Vec<serde_json::Value> = tools.into_iter().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema
                }
            })
        }).collect();

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "tools": tools_json,
            "stream": true,
        });

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            on_chunk(StreamingChunk::Content(content.to_string()));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages.iter().map(|m| {
                    serde_json::json!({ "role": m.role, "content": m.content })
                }).collect::<Vec<_>>()
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("No content in response".to_string()))?
            .to_string();

        Ok(ModelResponse::new(content, Usage {
            input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        }))
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Self::list_models(self).await
    }

    async fn stream_chat(&self, messages: Vec<Message>, on_chunk: Box<dyn FnMut(StreamingChunk) + Send + Sync + 'static>) -> Result<()> {
        Self::stream_chat(self, messages, on_chunk).await
    }
}

#[async_trait]
impl ToolCallingProvider for OpenAIProvider {
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse> {
        Self::chat_with_tools(self, messages, tools).await
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, on_chunk: Box<dyn FnMut(StreamingChunk) + Send + Sync + 'static>) -> Result<()> {
        Self::stream_chat_with_tools(self, messages, tools, on_chunk).await
    }
}