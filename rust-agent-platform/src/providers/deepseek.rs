use async_trait::async_trait;
use crate::providers::{LLMProvider, ToolCallingProvider};
use crate::core::{ModelResponse, Usage, Message, ToolDefinition};
use anyhow::{Result, Context};
use tokio_stream::StreamExt;

pub struct DeepSeekProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.deepseek.com".to_string()),
            model: model.unwrap_or_else(|| "deepseek-chat".to_string()),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, body);
        }

        let data: serde_json::Value = response.json().await?;
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
            .await?;

let data: serde_json::Value = response.json().await?;

        let data_str = serde_json::to_string_pretty(&data).unwrap_or_default();
        if let Ok(log_path) = std::env::var("DEBUG_LOG_PATH") {
            if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&log_path) {
                use std::io::Write;
                let _ = writeln!(file, "=== DEEPSEEK RESPONSE ===");
                let _ = writeln!(file, "{}", data_str);
            }
        }

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

    pub async fn stream_chat(&self, messages: Vec<Message>, mut on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        let client = reqwest::Client::new();
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "stream": true,
            "thinking": {
                "type": "enabled"
            },
            "reasoning_effort": "high"
        });

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let mut stream = response.bytes_stream();
        let mut is_thinking = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line.starts_with("data:") {
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        return Ok(());
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(reasoning) = json["choices"][0]["delta"]["reasoning_content"].as_str() {
                            if !reasoning.is_empty() {
                                if !is_thinking {
                                    on_chunk("[thinking] ".to_string());
                                    is_thinking = true;
                                }
                                on_chunk(reasoning.to_string());
                            }
                        }
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                if is_thinking {
                                    on_chunk("\n\n".to_string());
                                }
                                is_thinking = false;
                                on_chunk(content.to_string());
                            }
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
            "thinking": {
                "type": "enabled"
            },
            "reasoning_effort": "low"
        });

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let mut stream = response.bytes_stream();
        let mut is_thinking = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line.starts_with("data:") {
                    let data = line.trim_start_matches("data:").trim();
                    if data == "[DONE]" {
                        return Ok(());
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(reasoning) = json["choices"][0]["delta"]["reasoning_content"].as_str() {
                            if !reasoning.is_empty() {
                                if !is_thinking {
                                    on_chunk("<thinking>\n".to_string());
                                    is_thinking = true;
                                }
                                on_chunk(reasoning.to_string());
                            }
                        }
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                if is_thinking {
                                    on_chunk("\n</thinking>\n<content>\n".to_string());
                                    is_thinking = false;
                                } else {
                                    on_chunk("<content>\n".to_string());
                                }
                                on_chunk(content.to_string());
                                on_chunk("\n</content>\n".to_string());
                            }
                        }
                        if let Some(tool_calls) = json["choices"][0]["delta"]["tool_calls"].as_array() {
                            for tool_call in tool_calls {
                                if let Some(func) = tool_call.get("function") {
                                    let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                    if !name.is_empty() {
                                        if is_thinking {
                                            on_chunk("\n</thinking>\n<tool>\n".to_string());
                                            is_thinking = false;
                                        } else {
                                            on_chunk("<tool>\n".to_string());
                                        }
                                        on_chunk(name.to_string());
                                        on_chunk("\n</tool>\n".to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl LLMProvider for DeepSeekProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<ModelResponse> {
        let client = reqwest::Client::new();
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({ "role": m.role, "content": m.content })
            }).collect::<Vec<_>>(),
            "thinking": {
                "type": "enabled"
            },
            "reasoning_effort": "high"
        });

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .context("No content in response")?
            .to_string();

        Ok(ModelResponse::new(content, Usage {
            input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        }))
    }

    fn provider_name(&self) -> &str {
        "deepseek"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        Self::list_models(self).await
    }

    async fn stream_chat(&self, messages: Vec<Message>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        Self::stream_chat(self, messages, on_chunk).await
    }
}

#[async_trait]
impl ToolCallingProvider for DeepSeekProvider {
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<ModelResponse> {
        Self::chat_with_tools(self, messages, tools).await
    }

    fn provider_name(&self) -> &str {
        "deepseek"
    }

    async fn stream_chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, on_chunk: Box<dyn FnMut(String) + Send + Sync + 'static>) -> Result<()> {
        Self::stream_chat_with_tools(self, messages, tools, on_chunk).await
    }
}
