use async_trait::async_trait;
use crate::providers::LLMProvider;
use crate::core::{ModelResponse, Usage, Message};
use anyhow::{Result, Context};
use tokio_stream::StreamExt;

pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    model: String,
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

    pub async fn stream_chat<F>(&self, messages: Vec<Message>, mut on_chunk: F) -> Result<()>
    where
        F: FnMut(String) + Send + Sync,
    {
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

        Ok(ModelResponse {
            content,
            usage: Usage {
                input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            },
        })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Self::list_models(self).await
    }

    async fn stream_chat<F>(&self, messages: Vec<Message>, on_chunk: F) -> Result<()>
    where
        F: FnMut(String) + Send + Sync,
    {
        Self::stream_chat(self, messages, on_chunk).await
    }
}