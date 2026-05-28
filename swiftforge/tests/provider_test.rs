use rust_agent_platform::providers::{
    AnthropicProvider, LLMProvider, OllamaProvider, OpenAIProvider,
};

#[test]
fn test_openai_provider_creation() {
    let provider = OpenAIProvider::new("sk-test".to_string(), None);
    assert_eq!(provider.provider_name(), "openai");
}

#[test]
fn test_anthropic_provider_creation() {
    let provider = AnthropicProvider::new("sk-ant-test".to_string(), None);
    assert_eq!(provider.provider_name(), "anthropic");
}

#[test]
fn test_ollama_provider_creation() {
    let provider = OllamaProvider::new(None, None);
    assert_eq!(provider.provider_name(), "ollama");
}
