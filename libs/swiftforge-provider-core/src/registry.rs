use crate::error::ProviderError;
use crate::traits::{DynLLMProvider, DynToolCallingProvider, LLMProvider, ToolCallingProvider};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, DynLLMProvider>,
    tool_providers: HashMap<String, DynToolCallingProvider>,
    default_provider: Option<String>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            tool_providers: HashMap::new(),
            default_provider: None,
        }
    }

    pub fn register<P: LLMProvider + 'static>(&mut self, name: &str, provider: P) {
        self.providers.insert(name.to_string(), Arc::new(provider));
        if self.default_provider.is_none() {
            self.default_provider = Some(name.to_string());
        }
    }

    pub fn register_with_tools<P: ToolCallingProvider + 'static>(
        &mut self,
        name: &str,
        provider: P,
    ) {
        self.tool_providers
            .insert(name.to_string(), Arc::new(provider));
        if self.default_provider.is_none() {
            self.default_provider = Some(name.to_string());
        }
    }

    pub fn register_boxed(&mut self, name: &str, provider: DynLLMProvider) {
        self.providers.insert(name.to_string(), provider);
        if self.default_provider.is_none() {
            self.default_provider = Some(name.to_string());
        }
    }

    pub fn get(&self, name: &str) -> Option<&DynLLMProvider> {
        self.providers.get(name)
    }

    pub fn get_tool_provider(&self, name: &str) -> Option<&DynToolCallingProvider> {
        self.tool_providers.get(name)
    }

    pub fn default(&self) -> Option<&DynLLMProvider> {
        self.default_provider
            .as_ref()
            .and_then(|n| self.providers.get(n))
    }

    pub fn default_tool_provider(&self) -> Option<&DynToolCallingProvider> {
        self.default_provider
            .as_ref()
            .and_then(|n| self.tool_providers.get(n))
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn list_tool_providers(&self) -> Vec<String> {
        self.tool_providers.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
