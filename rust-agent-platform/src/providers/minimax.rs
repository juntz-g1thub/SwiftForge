use async_trait::async_trait;
use crate::providers::LLMProvider;
use crate::core::{ModelResponse, Usage, Message};
use anyhow::{Result, Context};
use tokio_stream::StreamExt;

pub struct MiniMaxProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl MiniMaxProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.minimax.chat/v1".to_string()),
            model: model.unwrap_or_else(|| "MiniMax-01".to_string()),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "MiniMax-01".to_string(),
            "MiniMax-01-Turbo".to_string(),
            "abab6.5s-chat".to_string(),
            "abab6.5g-chat".to_string(),
        ])
    }

    pub async fn stream_chat<F>(&self, messages: Vec<Message>, mut on_chunk: F) -> Result<()>
    where
        F: FnMut(String) + Send + Sync,
    {
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
impl LLMProvider for MiniMaxProvider {
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
            .context("No content in response")?
            .to_string();

        Ok(ModelResponse {
            content,
            usage: Usage {
                input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            },
        })
    }

    fn provider_name(&self) -> &str {
        "minimax"
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