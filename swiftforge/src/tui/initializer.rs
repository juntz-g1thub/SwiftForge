use anyhow::Result;
use std::sync::Arc;
use swiftforge_log::{info, warn};
use swiftforge_mcp::{McpConnectionPool, McpToolLoader};
use swiftforge_providers::{
    AnthropicProvider, CustomProvider, DeepSeekProvider, MiniMaxProvider, OllamaProvider,
    OpenAIProvider,
};
use swiftforge_provider_core::ProviderRegistry;
use swiftforge_tools::{BashTool, EditTool, GrepTool, ReadTool, WriteTool};
use swiftforge_types::ToolRegistry;
use tokio::runtime::Handle;

use crate::core::{Agent, AgentConfig, AgentRole};
use crate::tui::config::ConfigManager;

pub struct InitializedComponents {
    pub agent: Agent,
    pub tool_registry: Arc<ToolRegistry>,
    pub mcp_pool: Arc<McpConnectionPool>,
    pub mcp_loader: Arc<McpToolLoader>,
    pub provider_name: String,
    pub model_name: String,
}

pub fn initialize_components(runtime_handle: &Handle) -> Result<InitializedComponents> {
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(BashTool::new());
    tool_registry.register(ReadTool::new());
    tool_registry.register(WriteTool::new());
    tool_registry.register(EditTool::new());
    tool_registry.register(GrepTool::new());
    let tool_registry = Arc::new(tool_registry);

    let config = ConfigManager::new();
    let provider_name = config.get_provider().to_string();
    let model_name = config.get_model(&provider_name).to_string();
    let api_key = config.get_api_key(&provider_name).unwrap_or_default();
    let base_url = config.get_base_url(&provider_name);

    let _registry = ProviderRegistry::new();
    let llm_provider: swiftforge_provider_core::DynLLMProvider;
    let tool_provider: Option<swiftforge_provider_core::DynToolCallingProvider>;

    match provider_name.as_str() {
        "openai" => {
            let p = OpenAIProvider::new(api_key, base_url);
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
        "anthropic" => {
            let p = AnthropicProvider::new(api_key, base_url);
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
        "deepseek" => {
            let p = DeepSeekProvider::new(api_key, base_url, Some(model_name.clone()));
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
        "ollama" => {
            let p = OllamaProvider::new(base_url, Some(model_name.clone()));
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
        "minimax" => {
            let p = MiniMaxProvider::new(api_key, base_url, Some(model_name.clone()));
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
        "custom" => {
            let p = CustomProvider::new(
                "custom".to_string(),
                api_key,
                base_url.unwrap_or_default(),
                model_name.clone(),
            );
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
        _ => {
            let p = DeepSeekProvider::new(api_key, base_url, Some(model_name.clone()));
            llm_provider = Arc::new(p.clone());
            tool_provider = Some(Arc::new(p));
        }
    };

    let agent_config = AgentConfig {
        name: "tui-agent".to_string(),
        role: AgentRole::Executor,
        model: Some(model_name.clone()),
        temperature: 0.7,
    };
    let agent = Agent::new(agent_config, llm_provider)
        .with_tool_provider(tool_provider)
        .with_tool_registry(Arc::clone(&tool_registry));

    let mcp_pool = Arc::new(McpConnectionPool::new());
    let mcp_loader = Arc::new(McpToolLoader::new(
        Arc::clone(&mcp_pool),
        Arc::clone(&tool_registry),
    ));

    let runtime_handle = runtime_handle.clone();
    let config = ConfigManager::new();
    if let Some(mcp_url) = config.get_mcp_url() {
        let pool = Arc::clone(&mcp_pool);
        let loader = Arc::clone(&mcp_loader);

        runtime_handle.spawn(async move {
            if let Err(e) = pool.add_server("mcp", &mcp_url).await {
                warn!("[mcp]", "Failed to add server: {}", e);
                return;
            }

            info!("[mcp]", "Starting background connection to 'mcp'");

            if let Err(e) = pool.connect("mcp").await {
                warn!("[mcp]", "Failed to connect to 'mcp': {}", e);
                return;
            }
            info!("[mcp]", "Connected to MCP server: mcp");

            if let Err(e) = pool
                .initialize("mcp", "ragent", env!("CARGO_PKG_VERSION"))
                .await
            {
                warn!("[mcp]", "Failed to initialize 'mcp': {}", e);
                return;
            }

            match loader.load_tools("mcp").await {
                Ok(count) => {
                    info!("[mcp]", "Loaded {} tools from 'mcp'", count);
                }
                Err(e) => {
                    warn!("[mcp]", "Failed to load tools from 'mcp': {}", e);
                }
            }
        });
    }

    Ok(InitializedComponents {
        agent,
        tool_registry,
        mcp_pool,
        mcp_loader,
        provider_name,
        model_name,
    })
}