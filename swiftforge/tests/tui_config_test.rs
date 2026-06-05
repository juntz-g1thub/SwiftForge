use swiftforge::tui::{AppConfig, ConfigManager, ProviderSettings};

#[test]
fn test_config_manager_default_provider() {
    let manager = ConfigManager::new();
    let provider = manager.get_provider();
    assert!(!provider.is_empty());
}

#[test]
fn test_config_manager_set_provider() {
    let mut manager = ConfigManager::new();
    manager.set_provider("anthropic");
    assert_eq!(manager.get_provider(), "anthropic");
}

#[test]
fn test_config_manager_get_model() {
    let manager = ConfigManager::new();
    let model = manager.get_model("openai");
    assert!(!model.is_empty());
}

#[test]
fn test_config_manager_set_model() {
    let mut manager = ConfigManager::new();
    manager.set_model("openai", "gpt-4o-mini".to_string());
    assert_eq!(manager.get_model("openai"), "gpt-4o-mini");
}

#[test]
fn test_config_manager_get_api_key() {
    let manager = ConfigManager::new();
    let key = manager.get_api_key("openai");
    assert!(key.is_none());
}

#[test]
fn test_config_manager_set_api_key() {
    let mut manager = ConfigManager::new();
    manager.set_api_key("openai", Some("sk-test-key".to_string()));
    assert_eq!(
        manager.get_api_key("openai"),
        Some("sk-test-key".to_string())
    );
}

#[test]
fn test_config_manager_get_base_url() {
    let manager = ConfigManager::new();
    let url = manager.get_base_url("openai");
    assert!(url.is_none());
}

#[test]
fn test_config_manager_set_base_url() {
    let mut manager = ConfigManager::new();
    manager.set_base_url("ollama", Some("http://localhost:11434".to_string()));
    assert_eq!(
        manager.get_base_url("ollama"),
        Some("http://localhost:11434".to_string())
    );
}

#[test]
fn test_config_manager_is_enabled() {
    let manager = ConfigManager::new();
    assert!(manager.is_enabled("openai"));
}

#[test]
fn test_config_manager_set_enabled() {
    let mut manager = ConfigManager::new();
    manager.set_enabled("openai", false);
    assert!(!manager.is_enabled("openai"));
}

#[test]
fn test_config_manager_is_dirty() {
    let mut manager = ConfigManager::new();
    assert!(!manager.is_dirty());

    manager.set_provider("anthropic");
    assert!(manager.is_dirty());
}

#[test]
fn test_app_config_default() {
    let config = AppConfig::default();
    assert_eq!(config.current_provider, "openai");
    assert!(config.providers.contains_key("openai"));
    assert!(config.providers.contains_key("anthropic"));
    assert!(config.providers.contains_key("ollama"));
    assert!(config.providers.contains_key("deepseek"));
    assert!(config.providers.contains_key("minimax"));
}

#[test]
fn test_provider_settings() {
    let settings = ProviderSettings {
        base_url: Some("http://localhost:11434".to_string()),
        api_key: Some("sk-test".to_string()),
        model: "llama3".to_string(),
        enabled: true,
    };

    assert_eq!(settings.model, "llama3");
    assert!(settings.enabled);
    assert!(settings.api_key.is_some());
}

#[test]
fn test_config_manager_get_mcp_url() {
    let manager = ConfigManager::new();
    let url = manager.get_mcp_url();
    assert!(url.is_none());
}

#[test]
fn test_config_manager_set_mcp_url() {
    let mut manager = ConfigManager::new();
    manager.set_mcp_url(Some("http://localhost:8080".to_string()));
    assert_eq!(
        manager.get_mcp_url(),
        Some("http://localhost:8080".to_string())
    );
}

#[test]
fn test_config_manager_get_context_window() {
    let manager = ConfigManager::new();
    let window = manager.get_context_window();
    assert_eq!(window, 8000);
}

#[test]
fn test_config_manager_set_context_window() {
    let mut manager = ConfigManager::new();
    manager.set_context_window(16000);
    assert_eq!(manager.get_context_window(), 16000);
}

#[test]
fn test_config_manager_get_system_prompt() {
    let manager = ConfigManager::new();
    let prompt = manager.get_system_prompt();
    assert!(prompt.is_none());
}

#[test]
fn test_config_manager_set_system_prompt() {
    let mut manager = ConfigManager::new();
    manager.set_system_prompt(Some("You are a helpful assistant.".to_string()));
    assert_eq!(
        manager.get_system_prompt(),
        Some("You are a helpful assistant.".to_string())
    );
}
