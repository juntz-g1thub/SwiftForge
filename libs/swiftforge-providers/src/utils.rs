// Shared utilities for providers

/// Extract content from SSE data line
pub fn extract_sse_content(data: &str) -> Option<String> {
    let trimmed = data.trim();
    if trimmed.is_empty() || trimmed == "[DONE]" {
        return None;
    }
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // Try OpenAI format
        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
            return Some(content.to_string());
        }
        // Try Anthropic format
        if let Some(content) = json["content"][0]["text"].as_str() {
            return Some(content.to_string());
        }
    }
    None
}
