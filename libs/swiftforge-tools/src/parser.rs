use serde_json::Value as JsonValue;
use std::collections::HashMap;
use swiftforge_types::ToolCall;

pub struct ToolCallParser {
    re: regex::Regex,
}

impl Clone for ToolCallParser {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl ToolCallParser {
    pub fn new() -> Self {
        let re = regex::Regex::new(
            r#"<tool_call>\s*\{[^}]*?"name"\s*:\s*"([^"]+)"[^}]*?"arguments"\s*:\s*(\{[^}]+\})[^}]*\}</tool_call>"#
        ).expect("Invalid regex");
        Self { re }
    }

    /// From content, parse tool_calls (supports DeepSeek <tool_call> tags)
    pub fn parse(&self, content: &str) -> Vec<ToolCall> {
        let mut calls = Vec::new();

        // First try JSON parse
        if let Ok(json) = serde_json::from_str::<JsonValue>(content) {
            if let Some(tool_calls) = json.get("tool_calls").and_then(|t| t.as_array()) {
                for call in tool_calls {
                    if let (Some(name), Some(args)) = (
                        call.get("name").and_then(|n| n.as_str()),
                        call.get("arguments"),
                    ) {
                        let arguments = Self::parse_arguments(args);
                        calls.push(ToolCall {
                            name: name.to_string(),
                            arguments,
                        });
                    }
                }
            }
        }

        // Try DeepSeek <tool_call> tags
        for cap in self.re.captures_iter(content) {
            let name = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("{}");
            let arguments: HashMap<String, JsonValue> =
                serde_json::from_str(args_str).unwrap_or_default();
            if !name.is_empty() {
                calls.push(ToolCall { name, arguments });
            }
        }

        calls
    }

    /// From JSON array, parse (OpenAI format)
    pub fn parse_from_json(&self, tool_calls: &[JsonValue]) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        for call in tool_calls {
            let name = call
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .or_else(|| call.get("name").and_then(|n| n.as_str()));

            let args = call
                .get("function")
                .and_then(|f| f.get("arguments"))
                .or_else(|| call.get("arguments"));

            if let (Some(name), Some(args)) = (name, args) {
                let arguments = Self::parse_arguments(args);
                calls.push(ToolCall {
                    name: name.to_string(),
                    arguments,
                });
            }
        }
        calls
    }

    /// Check if content contains tool calls
    pub fn has_tool_calls(&self, content: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<JsonValue>(content) {
            if json.get("tool_calls").is_some() {
                return true;
            }
        }
        content.contains("<tool_call>")
    }

    fn parse_arguments(args: &JsonValue) -> HashMap<String, JsonValue> {
        if let JsonValue::Object(map) = args {
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else if let JsonValue::String(s) = args {
            serde_json::from_str(s).unwrap_or_default()
        } else {
            HashMap::new()
        }
    }
}

impl Default for ToolCallParser {
    fn default() -> Self {
        Self::new()
    }
}
