use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScript {
    pub name: String,
    pub description: String,
    pub version: String,
    pub tags: Vec<String>,
    pub config: ScriptConfig,
    pub steps: Vec<TestStep>,
    #[serde(default)]
    pub verify: Option<VerifyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    pub provider: ProviderConfig,
    pub agent: AgentScriptConfig,
    #[serde(default)]
    pub environment: EnvironmentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScriptConfig {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentConfig {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default)]
    pub retry_delay_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum TestStep {
    #[serde(alias = "chat")]
    Chat {
        name: String,
        #[serde(default)]
        description: Option<String>,
        input: ChatInput,
        expect: ChatExpect,
    },
    #[serde(alias = "tool_call")]
    ToolCall {
        name: String,
        #[serde(default)]
        description: Option<String>,
        tool: String,
        arguments: HashMap<String, serde_json::Value>,
        expect: ToolExpect,
    },
    #[serde(alias = "assert")]
    Assert {
        name: String,
        #[serde(default)]
        description: Option<String>,
        conditions: Vec<String>,
    },
    #[serde(alias = "wait")]
    Wait {
        name: String,
        #[serde(default)]
        description: Option<String>,
        duration_ms: u64,
    },
    #[serde(alias = "loop")]
    Loop {
        name: String,
        #[serde(default)]
        description: Option<String>,
        times: usize,
        steps: Vec<TestStep>,
    },
    #[serde(alias = "condition")]
    Condition {
        name: String,
        #[serde(default)]
        description: Option<String>,
        condition: String,
        then: Vec<TestStep>,
        #[serde(default, rename = "else")]
        else_branch: Option<Vec<TestStep>>,
    },
    #[serde(alias = "snapshot")]
    Snapshot {
        name: String,
        #[serde(default)]
        description: Option<String>,
        snapshot_name: String,
        data: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInput {
    pub messages: Vec<MessageInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatExpect {
    #[serde(default)]
    pub response_success: bool,
    #[serde(default)]
    pub response_contains: Option<String>,
    #[serde(default)]
    pub response_matches: Option<String>,
    #[serde(default)]
    pub response_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExpect {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub output_contains: Option<String>,
    #[serde(default)]
    pub stderr_empty: Option<bool>,
    #[serde(default)]
    pub error_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyConfig {
    #[serde(default)]
    pub final_state: Option<FinalState>,
    #[serde(default)]
    pub snapshots: Option<Vec<SnapshotExpect>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalState {
    #[serde(default)]
    pub messages_count: Option<usize>,
    #[serde(default)]
    pub last_role: Option<String>,
    #[serde(default)]
    pub tools_called: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotExpect {
    pub name: String,
    pub data: HashMap<String, String>,
}

pub fn parse_script(content: &str) -> Result<TestScript> {
    let expanded = expand_env_vars(content);
    serde_yaml::from_str::<TestScript>(&expanded).map_err(|e| {
        anyhow::anyhow!(
            "YAML parse error: {} in {}",
            e,
            expanded.lines().take(5).collect::<Vec<_>>().join("\n")
        )
    })
}

fn expand_env_vars(content: &str) -> String {
    let mut result = content.to_string();
    let env_var_regex = regex::Regex::new(r"\$\{([^}:]+)(?::-[^}]*)?\}").unwrap();

    for cap in env_var_regex.captures_iter(content) {
        let full_match = cap.get(0).unwrap().as_str();
        let var_name = cap.get(1).unwrap().as_str();
        let default_value = cap.get(2).map(|m| &m.as_str()[2..]).unwrap_or("");

        let value = std::env::var(var_name).unwrap_or_else(|_| default_value.to_string());
        result = result.replace(full_match, &value);
    }

    result
}

pub fn parse_step_as_yaml(step: &TestStep) -> Result<String> {
    let json = serde_json::to_value(step)?;
    let yaml = serde_yaml::to_string(&json)?;
    Ok(yaml)
}

pub fn get_step_name(step: &TestStep) -> &str {
    match step {
        TestStep::Chat { name, .. } => name,
        TestStep::ToolCall { name, .. } => name,
        TestStep::Assert { name, .. } => name,
        TestStep::Wait { name, .. } => name,
        TestStep::Loop { name, .. } => name,
        TestStep::Condition { name, .. } => name,
        TestStep::Snapshot { name, .. } => name,
    }
}

pub fn get_step_action(step: &TestStep) -> &'static str {
    match step {
        TestStep::Chat { .. } => "chat",
        TestStep::ToolCall { .. } => "tool_call",
        TestStep::Assert { .. } => "assert",
        TestStep::Wait { .. } => "wait",
        TestStep::Loop { .. } => "loop",
        TestStep::Condition { .. } => "condition",
        TestStep::Snapshot { .. } => "snapshot",
    }
}
