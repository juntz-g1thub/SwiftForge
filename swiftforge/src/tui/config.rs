use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub base_url: Option<String>, // 第1位 - URL
    pub api_key: Option<String>,  // 第2位 - API密钥
    pub model: String,            // 第3位 - 模型名
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub current_provider: String,
    pub providers: std::collections::HashMap<String, ProviderSettings>,
    pub system_prompt: Option<String>,
    pub context_window: usize,
    pub mcp_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderSettings {
                base_url: None,
                api_key: None,
                model: "gpt-4o".to_string(),
                enabled: true,
            },
        );
        providers.insert(
            "anthropic".to_string(),
            ProviderSettings {
                base_url: None,
                api_key: None,
                model: "claude-3-5-sonnet".to_string(),
                enabled: true,
            },
        );
        providers.insert(
            "ollama".to_string(),
            ProviderSettings {
                base_url: Some("http://localhost:11434".to_string()),
                api_key: None,
                model: "llama3".to_string(),
                enabled: true,
            },
        );
        providers.insert(
            "deepseek".to_string(),
            ProviderSettings {
                base_url: None,
                api_key: None,
                model: "deepseek-v4-flash".to_string(),
                enabled: true,
            },
        );
        providers.insert(
            "minimax".to_string(),
            ProviderSettings {
                base_url: None,
                api_key: None,
                model: "MiniMax-01".to_string(),
                enabled: true,
            },
        );

        Self {
            current_provider: "openai".to_string(),
            providers,
            system_prompt: None,
            context_window: 8000,
            mcp_url: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("FastCode")
            .join("config.json")
    }
}

#[derive(Clone)]
pub struct ConfigManager {
    config: AppConfig,
    dirty: bool,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: AppConfig::load(),
            dirty: false,
        }
    }

    pub fn get_provider(&self) -> &str {
        &self.config.current_provider
    }

    pub fn set_provider(&mut self, provider: &str) {
        self.config.current_provider = provider.to_string();
        self.dirty = true;
    }

    // 第1位 - base_url
    pub fn get_base_url(&self, provider: &str) -> Option<String> {
        self.config
            .providers
            .get(provider)
            .and_then(|p| p.base_url.clone())
    }

    pub fn set_base_url(&mut self, provider: &str, url: Option<String>) {
        if let Some(cfg) = self.config.providers.get_mut(provider) {
            cfg.base_url = url;
            self.dirty = true;
        }
    }

    // 第2位 - api_key
    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        self.config
            .providers
            .get(provider)
            .and_then(|p| p.api_key.clone())
    }

    pub fn set_api_key(&mut self, provider: &str, api_key: Option<String>) {
        if let Some(cfg) = self.config.providers.get_mut(provider) {
            cfg.api_key = api_key;
            self.dirty = true;
        }
    }

    // 第3位 - model
    pub fn get_model(&self, provider: &str) -> String {
        self.config
            .providers
            .get(provider)
            .map(|p| p.model.clone())
            .unwrap_or_else(|| "gpt-4o".to_string())
    }

    pub fn set_model(&mut self, provider: &str, model: String) {
        if let Some(cfg) = self.config.providers.get_mut(provider) {
            cfg.model = model;
            self.dirty = true;
        }
    }

    // 第4位 - enabled
    pub fn is_enabled(&self, provider: &str) -> bool {
        self.config
            .providers
            .get(provider)
            .map(|p| p.enabled)
            .unwrap_or(false)
    }

    pub fn set_enabled(&mut self, provider: &str, enabled: bool) {
        if let Some(cfg) = self.config.providers.get_mut(provider) {
            cfg.enabled = enabled;
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config.save()?;
        self.dirty = false;
        Ok(())
    }

    pub fn load(&mut self) {
        self.config = AppConfig::load();
        self.dirty = false;
    }

    pub fn get_full_config(&self) -> &AppConfig {
        &self.config
    }

    // System prompt
    pub fn get_system_prompt(&self) -> Option<String> {
        self.config.system_prompt.clone()
    }

    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.config.system_prompt = prompt;
        self.dirty = true;
    }

    // Context window
    pub fn get_context_window(&self) -> usize {
        self.config.context_window
    }

    pub fn set_context_window(&mut self, window: usize) {
        self.config.context_window = window;
        self.dirty = true;
    }

    pub fn get_mcp_url(&self) -> Option<String> {
        self.config.mcp_url.clone()
    }

    pub fn set_mcp_url(&mut self, url: Option<String>) {
        self.config.mcp_url = url;
        self.dirty = true;
    }
}

pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub models: &'static str,
}

impl ProviderInfo {
    pub fn list_all() -> Vec<ProviderInfo> {
        vec![
            ProviderInfo {
                id: "openai",
                name: "OpenAI",
                models: "GPT-4o, GPT-4o-mini, GPT-4-Turbo",
            },
            ProviderInfo {
                id: "anthropic",
                name: "Anthropic",
                models: "Claude-3.5-Sonnet, Claude-3-Opus",
            },
            ProviderInfo {
                id: "ollama",
                name: "Ollama (Local)",
                models: "Llama3, CodeLlama, Mistral",
            },
            ProviderInfo {
                id: "deepseek",
                name: "DeepSeek",
                models: "DeepSeek-V4-Pro, DeepSeek-V4-Flash",
            },
            ProviderInfo {
                id: "minimax",
                name: "MiniMax",
                models: "MiniMax-01",
            },
            ProviderInfo {
                id: "custom",
                name: "Custom API",
                models: "User-defined",
            },
        ]
    }
}
