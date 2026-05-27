use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use anyhow::Result;
use crossterm::{event::{self, Event}, execute};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::runtime::Builder;

use crate::core::{Agent, AgentConfig, AgentRole, ToolRegistry};
use crate::providers::{OpenAIProvider, AnthropicProvider, OllamaProvider, DeepSeekProvider, MiniMaxProvider, CustomProvider};
use crate::tools::{BashTool, ReadTool, WriteTool, EditTool, GrepTool};
use crate::tui::config::ConfigManager;

use crate::tui::{AppContext, UIState, Action, ViewState, View, ChatView, ConfigView};

pub struct AppController {
    context: AppContext,
    ui_state: UIState,
    runtime: tokio::runtime::Runtime,
    current_view: Box<dyn View>,
    should_quit: bool,
    debug_rx: Option<mpsc::Receiver<String>>,
}

impl AppController {
    pub fn new(show_debug: bool) -> Result<Self> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        let mut tool_registry = ToolRegistry::new();
        tool_registry.register(BashTool::new());
        tool_registry.register(ReadTool::new());
        tool_registry.register(WriteTool::new());
        tool_registry.register(EditTool::new());
        tool_registry.register(GrepTool::new());
        let tool_registry = Arc::new(tool_registry);

        let agent_config = AgentConfig {
            name: "tui-agent".to_string(),
            role: AgentRole::Executor,
            model: None,
            temperature: 0.7,
        };
        let agent = Agent::new(agent_config)
            .with_tool_registry(Arc::clone(&tool_registry));

        let config = ConfigManager::new();
        let context = AppContext::new(agent, config, tool_registry, show_debug);
        let ui_state = UIState::new();

        let provider = context.config.lock().unwrap().get_provider().to_string();
        let model = context.config.lock().unwrap().get_model(&provider).to_string();
        let current_view: Box<dyn View> = Box::new(ChatView::new(&provider, &model));

        let debug_rx = if show_debug {
            let (tx, rx) = mpsc::channel();
            *ui_state.debug_tx.lock().unwrap() = Some(tx);
            Some(rx)
        } else {
            None
        };

        Ok(Self {
            context,
            ui_state,
            runtime,
            current_view,
            should_quit: false,
            debug_rx,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.log("Application started");

        loop {
            terminal.draw(|f| {
                self.current_view.render(f, f.size(), &self.context, &self.ui_state);
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if let Some(action) = self.current_view.handle_key(key, &self.context) {
                        self.handle_action(action)?;

                        if self.should_quit {
                            break;
                        }
                    }
                }
            }

            self.process_agent_response();
            self.process_debug_rx();
        }

        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        self.log(&format!("Action: {:?}", action));

        match action {
            Action::SendMessage(msg) => {
                if let Some(chat_view) = self.get_chat_view_mut() {
                    chat_view.state.add_message("user", &msg);
                    chat_view.state.is_streaming = true;
                }
                self.spawn_agent_task(msg);
            }
            Action::SwitchView(view_state) => {
                self.current_view.on_exit();
                self.current_view = match view_state {
                    ViewState::Chat(ctx) => {
                        Box::new(ChatView::new(&ctx.current_provider, &ctx.current_model))
                    }
                    ViewState::Config(ctx) => {
                        let mut view = ConfigView::new();
                        view.state = ctx;
                        Box::new(view)
                    }
                };
                self.current_view.on_enter();
            }
            Action::GoBack => {
                self.current_view.on_exit();
                let provider = self.context.config.lock().unwrap().get_provider().to_string();
                let model = self.context.config.lock().unwrap().get_model(&provider).to_string();
                self.current_view = Box::new(ChatView::new(&provider, &model));
                self.current_view.on_enter();
            }
            Action::CancelStreaming => {
                self.ui_state.clear_streaming();
                if let Some(chat_view) = self.get_chat_view_mut() {
                    chat_view.state.is_streaming = false;
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SelectProvider(name) => {
                self.context.config.lock().unwrap().set_provider(&name);
            }
            Action::SaveApiKey(key) => {
                let provider = self.context.config.lock().unwrap().get_provider().to_string();
                self.context.config.lock().unwrap().set_api_key(&provider, Some(key));
            }
            Action::SaveModel(model) => {
                let provider = self.context.config.lock().unwrap().get_provider().to_string();
                self.context.config.lock().unwrap().set_model(&provider, model);
            }
            Action::SaveBaseUrl(url) => {
                let provider = self.context.config.lock().unwrap().get_provider().to_string();
                self.context.config.lock().unwrap().set_base_url(&provider, Some(url));
            }
            Action::FetchModels => {
                self.spawn_fetch_models();
            }
            Action::ToggleDebug => {
            }
            _ => {}
        }

        Ok(())
    }

    fn get_chat_view_mut(&mut self) -> Option<&mut ChatView> {
        let ptr = self.current_view.as_mut() as *mut dyn View;
        if ptr.is_null() {
            return None;
        }
        let chat_ptr = ptr as *mut ChatView;
        if chat_ptr.is_null() {
            return None;
        }
        unsafe { Some(&mut *chat_ptr) }
    }

    fn spawn_agent_task(&mut self, msg: String) {
        let runtime = self.runtime.handle().clone();

        let (tx, rx) = mpsc::channel();
        *self.ui_state.response_receiver.lock().unwrap() = Some(rx);

        let provider_name = self.context.config.lock().unwrap().get_provider().to_string();
        let api_key = self.context.config.lock().unwrap().get_api_key(&provider_name);
        let base_url = self.context.config.lock().unwrap().get_base_url(&provider_name);
        let model = self.context.config.lock().unwrap().get_model(&provider_name).to_string();
        let agent = self.context.agent.clone();
        let debug_path = self.context.debug_log_path.clone();

        let streaming_text = Arc::clone(&self.ui_state.streaming_text);
        let debug_messages = Arc::clone(&self.ui_state.debug_messages);
        let finalized_message = Arc::clone(&self.ui_state.finalized_message);
        let debug_tx = self.ui_state.debug_tx.clone();

        runtime.spawn(async move {
            let model_opt = Some(model.clone());

            let final_agent: Arc<Agent> = match provider_name.as_str() {
                "openai" => {
                    let p = OpenAIProvider::new(api_key.unwrap_or_default(), base_url);
                    Arc::new(Agent::clone(&agent).with_tool_provider("openai", p))
                }
                "anthropic" => {
                    let p = AnthropicProvider::new(api_key.unwrap_or_default(), base_url);
                    Arc::new(Agent::clone(&agent).with_tool_provider("anthropic", p))
                }
                "ollama" => {
                    let p = OllamaProvider::new(base_url, model_opt);
                    Arc::new(Agent::clone(&agent).with_tool_provider("ollama", p))
                }
                "deepseek" => {
                    let p = DeepSeekProvider::new(api_key.unwrap_or_default(), base_url, model_opt);
                    Arc::new(Agent::clone(&agent).with_tool_provider("deepseek", p))
                }
                "minimax" => {
                    let p = MiniMaxProvider::new(api_key.unwrap_or_default(), base_url, model_opt);
                    Arc::new(Agent::clone(&agent).with_tool_provider("minimax", p))
                }
                "custom" => {
                    let p = CustomProvider::new(
                        "custom".to_string(),
                        api_key.unwrap_or_default(),
                        base_url.unwrap_or_default(),
                        model_opt.unwrap_or_default(),
                    );
                    Arc::new(Agent::clone(&agent).with_tool_provider("custom", p))
                }
                _ => agent,
            };

            {
                if let Ok(mut msgs) = debug_messages.lock() {
                    msgs.push(format!("Starting request to {}", provider_name));
                    if msgs.len() > 100 {
                        msgs.remove(0);
                    }
                }
                if let Ok(mut streaming) = streaming_text.lock() {
                    *streaming = None;
                }
            }

            let debug_sender = debug_tx.lock().unwrap().take();
            let result = final_agent.run_agent_loop(
                &msg,
                5,
                debug_path.map(|p| p.to_string_lossy().to_string()),
                debug_sender,
                Some(tx),
            ).await;

            match result {
                Ok(response) => {
                    if let Ok(mut msgs) = debug_messages.lock() {
                        msgs.push(format!("Response length: {}", response.len()));
                        if msgs.len() > 100 {
                            msgs.remove(0);
                        }
                    }
                    if !response.is_empty() {
                        if let Ok(mut finalized) = finalized_message.lock() {
                            *finalized = Some(("assistant".to_string(), response));
                        }
                    }
                }
                Err(e) => {
                    if let Ok(mut msgs) = debug_messages.lock() {
                        msgs.push(format!("Error: {}", e));
                        if msgs.len() > 100 {
                            msgs.remove(0);
                        }
                    }
                    let partial = streaming_text.lock()
                        .map(|mut s| s.take().unwrap_or_default())
                        .unwrap_or_default();
                    if let Ok(mut finalized) = finalized_message.lock() {
                        *finalized = Some(("error".to_string(), format!("{} (partial: {})", e, partial)));
                    }
                }
            }
        });
    }

    fn spawn_fetch_models(&mut self) {
        let runtime = self.runtime.handle().clone();

        let provider_name = self.context.config.lock().unwrap().get_provider().to_string();
        let api_key = self.context.config.lock().unwrap().get_api_key(&provider_name);
        let base_url = self.context.config.lock().unwrap().get_base_url(&provider_name);
        let model = self.context.config.lock().unwrap().get_model(&provider_name).to_string();
        let config = Arc::clone(&self.context.config);

        runtime.spawn(async move {
            let model_opt = Some(model.clone());

            let models = match provider_name.as_str() {
                "openai" => {
                    let p = OpenAIProvider::new(api_key.unwrap_or_default(), base_url);
                    p.list_models().await.unwrap_or_default()
                }
                "anthropic" => {
                    let p = AnthropicProvider::new(api_key.unwrap_or_default(), base_url);
                    p.list_models().await.unwrap_or_default()
                }
                "ollama" => {
                    let p = OllamaProvider::new(base_url, model_opt.clone());
                    p.list_models().await.unwrap_or_default()
                }
                "deepseek" => {
                    let p = DeepSeekProvider::new(api_key.unwrap_or_default(), base_url, model_opt.clone());
                    p.list_models().await.unwrap_or_default()
                }
                "minimax" => {
                    let p = MiniMaxProvider::new(api_key.unwrap_or_default(), base_url, model_opt.clone());
                    p.list_models().await.unwrap_or_default()
                }
                _ => Vec::new(),
            };

            for m in models {
                config.lock().unwrap().set_model(&provider_name, m);
            }
        });
    }

    fn process_agent_response(&mut self) {
        let finalized_msg = {
            if let Ok(mut finalized) = self.ui_state.finalized_message.lock() {
                finalized.take()
            } else {
                None
            }
        };

        if let Some((role, content)) = finalized_msg {
            if let Some(chat_view) = self.get_chat_view_mut() {
                chat_view.state.add_message(&role, &content);
                chat_view.state.is_streaming = false;
            }
        }

        if let Some(chat_view) = self.get_chat_view_mut() {
            if let Ok(receiver) = self.ui_state.response_receiver.lock() {
                if let Some(ref rx) = *receiver {
                    while let Ok(result) = rx.try_recv() {
                        match result {
                            Ok(chunk) => {
                                self.ui_state.append_streaming(&chunk);
                            }
                            Err(_e) => {
                                if let Ok(mut streaming) = self.ui_state.streaming_text.lock() {
                                    let _ = streaming.take();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn process_debug_rx(&mut self) {
        if let Some(ref rx) = self.debug_rx {
            while let Ok(msg) = rx.try_recv() {
                if let Ok(mut messages) = self.ui_state.debug_messages.lock() {
                    messages.push(msg);
                    if messages.len() > 100 {
                        messages.remove(0);
                    }
                }
            }
        }
    }

    fn log(&self, msg: &str) {
        if let Some(ref path) = self.context.debug_log_path {
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
            let formatted = format!("[{}] {}", timestamp, msg);
            let _ = std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "{}", formatted)
                });
        }
        self.ui_state.add_debug(msg.to_string());
    }
}